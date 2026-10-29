use std::{collections::{HashMap, hash_map::Entry}, marker::PhantomData, path::Path};
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};

const EXT_DIV: char = '.';

/// Symbols that are percent-encoded to keep the href valid
const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ').add(b'"').add(b'#').add(b'<').add(b'>').add(b'?').add(b'`')
    .add(b'{').add(b'}').add(b'%').add(b'/').add(b'\\').add(b'[').add(b']').add(b'^').add(b'|');

pub(crate) const IMG_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];
pub(crate) const FILE_EXTENSIONS: &[&str] = &["rush"];

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

#[derive(Debug, PartialEq)]
struct Resource<D> {
    local_path: String,
    data: D,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ResTable<K: ResKind> {
    map: HashMap<String, Id<K>>,
    vec: Vec<Resource<K::Data>>,
}
impl<K: ResKind> ResTable<K> {
    pub(crate) fn iter(&self) -> impl Iterator<Item = (Id<K>, &Path, &K::Data)> + '_ {
        self.vec.iter().enumerate().map(|(i, r)| (Id::new(i), Path::new(&r.local_path), &r.data))
    }

    pub(crate) fn local_paths(&self) -> impl Iterator<Item = &str> + '_ {
        self.vec.iter().map(|r| r.local_path.as_str())
    }
}

impl<K: ResKind> Default for ResTable<K> {
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

impl Resources {
    pub(crate) fn get_file_id(&self, file: &str) ->  Option<Id<FileId>> {
        let name = get_path(file, FILE_EXTENSIONS);
        self.files.map.get(name).cloned()
    }

    pub(crate) fn get_image_id(&self, file: &str) ->  Option<Id<ImageId>> {
        let name = get_path(file, IMG_EXTENSIONS);
        self.images.map.get(name).cloned()
    }
}


impl<K: ResKind> ResTable<K> {
    pub(crate) fn insert(&mut self, key: &str, local_path: &str, data: K::Data) -> Result<(), String> {
        let key = get_path(key, K::EXTENSIONS);
        match self.map.entry(key.to_string()) {
            Entry::Vacant(v) => {
                v.insert(Id::new(self.vec.len()));
                self.vec.push(Resource { local_path: local_path.to_string(), data });
                Ok(())
            }
            Entry::Occupied(o) => Err(o.key().clone()),
        }
    }

    pub(crate) fn local_path(&self, id: Id<K>) -> &Path {
        Path::new(&self.vec[id.idx].local_path)
    }

    pub(crate) fn data(&self, id: Id<K>) -> &K::Data {
        &self.vec[id.idx].data
    }
}

pub(crate) trait ResKind: Sized {
    type Data;
    const EXTENSIONS: &'static [&'static str];
    fn table(res: &Resources) -> &ResTable<Self>;
}
impl ResKind for ImageId {
    type Data = Option<(u32, u32)>;
    const EXTENSIONS: &'static [&'static str] = IMG_EXTENSIONS;
    fn table(res: &Resources) -> &ResTable<Self> {
        &res.images
    }
}
impl ResKind for FileId {
    type Data = String;
    const EXTENSIONS: &'static [&'static str] = FILE_EXTENSIONS;
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

    pub(crate) fn root_href(&self, from: Id<FileId>, to: &str) -> String {
        let from_dir = self.files.local_path(from).parent().unwrap();
        relative_path(from_dir, Path::new(to))
    }

    /// Resource path relative to the output root
    pub(crate) fn root_relative<K: ResKind>(&self, to: Id<K>) -> String {
        relative_path(Path::new(""), K::table(self).local_path(to))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_path_is_percent_encoded() {
        let href = relative_path(Path::new("a/b"), Path::new("a/файл #1%.html"));
        assert_eq!(href, "../%D1%84%D0%B0%D0%B9%D0%BB%20%231%25.html");
    }
}
