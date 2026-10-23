use std::{collections::{HashMap, hash_map::Entry}, fs, io, marker::PhantomData, path::{Component, Path, PathBuf}};
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

use crate::resources::ResourceWarning::FileNameCollision;

const EXT_DIV: char = '.';

const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ').add(b'"').add(b'#').add(b'<').add(b'>').add(b'?').add(b'`')
    .add(b'{').add(b'}').add(b'%').add(b'/').add(b'\\').add(b'[').add(b']').add(b'^').add(b'|');

const IMG_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];
const FILE_EXTENSIONS: &[&str] = &["rush"];

fn get_path<'a>(file: &'a str, exts: &[&str]) -> &'a str{
    if let Some(pos) = file.rfind(EXT_DIV) {
        let name = &file[..pos];
        let ext = &file[pos+EXT_DIV.len_utf8()..];
        if exts.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
            name
        } else { file }
    } else {file}
}


#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ImageId;
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FileId;

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) struct Id<K> {
    pub(crate) idx: usize,
    _kind: PhantomData<K>
}
impl<K> Id<K> {
    pub(crate) fn new(idx: usize) -> Self {
        Self {idx, _kind: PhantomData}
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceWarning {
    FileNameCollision(String),
}

#[derive(Debug, PartialEq)]
struct Resource {
    global_path: PathBuf,
    local_path: PathBuf,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ResTable<K> {
    map: HashMap<String, Id<K>>,
    vec: Vec<Resource>,
}
impl<K> ResTable<K> {
    pub(crate) fn iter(&self) -> impl Iterator<Item = (Id<K>, &Path, &Path)> + '_ {
        self.vec.iter().enumerate().map(|(i, r)| (Id::new(i), r.global_path.as_path(), r.local_path.as_path()))
    }
}

impl<K> Default for ResTable<K> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            vec: Vec::new()
        }
    }
}

#[derive(Default, Debug, PartialEq)]
pub(crate) struct Resources {
    pub(crate) images: ResTable<ImageId>,
    pub(crate) files: ResTable<FileId>,
}

#[derive(Default)]
struct Collected {
    images: Vec<PathBuf>,
    files: Vec<PathBuf>,
}

impl Resources {
    pub(crate) fn build_index(root: &Path) -> io::Result<(Resources, Vec<ResourceWarning>)> {
        let mut collected = Collected::default();
        walk(root, &mut collected)?;

        let mut warns = Vec::new();
        let images = ResTable::build(root, collected.images, &mut warns)?;
        let files = ResTable::build(root, collected.files, &mut warns)?;
        Ok((Resources { images, files }, warns))
    }

    pub(crate) fn get_file_id(&self, file: &str) ->  Option<Id<FileId>> {
        let name = get_path(file, FILE_EXTENSIONS);
        self.files.map.get(name).cloned()
    }

    pub(crate) fn get_image_id(&self, file: &str) ->  Option<Id<ImageId>> {
        let name = get_path(file, IMG_EXTENSIONS);
        self.images.map.get(name).cloned()
    }
}


impl<K> ResTable<K> {
    fn build(root: &Path, mut paths: Vec<PathBuf>, warns: &mut Vec<ResourceWarning>) -> io::Result<Self> {
        paths.sort();

        let segs: Vec<Vec<&str>> = paths
            .iter()
            .map(|p| split_to_path_segs(root, p))
            .collect::<io::Result<_>>()?;

        let common = common_dir_prefix(&segs);

        let mut table = Self::default();
        for (path, segs) in paths.iter().zip(&segs) {
            let key = segs_to_key(&segs[common..]);
            match table.map.entry(key) {
                Entry::Vacant(v) => {
                    let id = Id::new(table.vec.len());
                    let local_path = path.strip_prefix(root).unwrap().to_path_buf();
                    table.vec.push(Resource {global_path: path.clone(), local_path});
                    v.insert(id);
                }
                Entry::Occupied(o) => warns.push(FileNameCollision(o.key().clone())),
            }
        }
        Ok(table)
    }

    pub(crate) fn global_path(&self, id: Id<K>) -> &Path {
        &self.vec[id.idx].global_path
    }


    pub(crate) fn local_path(&self, id: Id<K>) -> &Path {
        &self.vec[id.idx].local_path
    }
}

pub(crate) trait ResKind: Sized {
    fn table(res: &Resources) -> &ResTable<Self>;
}
impl ResKind for ImageId {
    fn table(res: &Resources) -> &ResTable<Self> {
        &res.images
    }
}
impl ResKind for FileId {
    fn table(res: &Resources) -> &ResTable<Self> {
        &res.files
    }
}

impl Resources {
    pub(crate) fn href<K: ResKind>(&self, from: Id<FileId>, to: Id<K>) -> String {
        let from_dir = self.files.local_path(from).parent().unwrap();
        let target = K::table(self).local_path(to);
        relative_path(from_dir, target)
    }

    pub(crate) fn root_href(&self, from: Id<FileId>, target: &str) -> String {
        let from_dir = self.files.local_path(from).parent().unwrap();
        relative_path(from_dir, Path::new(target))
    }

    /// Resource path relative to the output root
    pub(crate) fn root_relative<K: ResKind>(&self, to: Id<K>) -> String {
        relative_path(Path::new(""), K::table(self).local_path(to))
    }

    pub(crate) fn copy_tree(&self, new_root: &Path) -> io::Result<()> {
        for res in &self.images.vec {
            let dst = new_root.join(&res.local_path);
            if let Some(dir) = dst.parent() {
                fs::create_dir_all(dir)?;
            }
            fs::copy(&res.global_path, &dst)?;
        }

        for res in &self.files.vec {
            if let Some(dir) = new_root.join(&res.local_path).parent() {
                fs::create_dir_all(dir)?;
            }
        }
        Ok(())
    }
}

fn relative_path(from_dir: &Path, target: &Path) -> String {
    let mut from_path = from_dir.components().peekable();
    let mut target_path = target.components().peekable();
    while let Some(f) = from_path.peek() && let Some(t) = target_path.peek() && f == t {
        from_path.next();
        target_path.next();
    }
    let mut parts: Vec<String> = Vec::new();
    for _ in from_path {
        parts.push("..".to_string());
    }
    for c in target_path {
        let seg = c.as_os_str().to_string_lossy();
        parts.push(utf8_percent_encode(&seg, PATH_SEGMENT).to_string());
    }
    parts.join("/")
}

fn walk(dir: &Path, out: &mut Collected) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;

        if ft.is_dir() {
            walk(&path, out)?;
            continue;
        }
        if !ft.is_file() {
            continue;
        }

        let Some(ext) = path.extension() else { continue };
        if IMG_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
            out.images.push(path);
        } else if FILE_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
            out.files.push(path);
        }
    }
    Ok(())
}

fn split_to_path_segs<'a>(root: &Path, path: &'a Path) -> io::Result<Vec<&'a str>> {
    let rel = path.strip_prefix(root).expect("path in root");
    rel.components().map(|c| match c {
        Component::Normal(s) => s.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Name {} is not UTF-8", path.display()),
            )
        }),
        _ => unreachable!("must be only normal components after strip"),
    }).collect()
}

fn common_dir_prefix(all: &[Vec<&str>]) -> usize {
    let Some((first, rest)) = all.split_first() else { return 0 };
    let mut n = first.len() - 1; 
    for s in rest {
        let dirs = &s[..s.len() - 1];
        n = first[..n].iter().zip(dirs).take_while(|(a, b)| a == b).count();
        if n == 0 { break; }
    }
    n
}

fn segs_to_key(segs: &[&str]) -> String {
    let (last, dirs) = segs.split_last().expect("file has a name");
    let stem = Path::new(last).file_stem().and_then(|s| s.to_str()).unwrap_or(last);
    let mut key = dirs.join("/");
    if !key.is_empty() {
        key.push('/');
    }
    key.push_str(stem);
    key
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_is_percent_encoded() {
        let href = relative_path(Path::new("a/b"), Path::new("a/файл #1%.html"));
        assert_eq!(href, "../%D1%84%D0%B0%D0%B9%D0%BB%20%231%25.html");
    }
}
