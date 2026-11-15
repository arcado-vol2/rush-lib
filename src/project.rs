use std::{fs, io, path::{Component, Path, PathBuf}};

use crate::content::{ContentEntry, DirEntry};
use crate::resources::{FILE_EXTENSIONS, IMG_EXTENSIONS, Resources};

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectWarning {
    FileNameCollision(String),
    UnknownContentPath(String),
    DuplicateContent(String),
    /// the file is not in the single output: it is missing from the table of contents
    NotInContent(String),
}

#[derive(Debug, Default, PartialEq)]
pub struct Project {
    pub(crate) resources: Resources,
    pub(crate) content: Vec<ContentEntry>,
    pub(crate) dirs: Vec<DirEntry>,
}

impl Project {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, key: &str, path: &str, text: String) -> Result<(), ProjectWarning> {
        self.resources.files.insert(key, path, text).map_err(ProjectWarning::FileNameCollision)
    }

    pub fn add_image(&mut self, key: &str, path: &str, size: Option<(u32, u32)>) -> Result<(), ProjectWarning> {
        self.resources.images.insert(key, path, size).map_err(ProjectWarning::FileNameCollision)
    }

    pub fn add_content(&mut self, name: String, key: String) {
        self.content.push(ContentEntry { name, key });
    }

    /// `key` - dir path from source root
    pub fn add_dir(&mut self, name: String, key: String) {
        self.dirs.push(DirEntry { name, key: DirEntry::normalize(&key) });
    }

    pub fn images(&self) -> impl Iterator<Item = &str> + '_ {
        self.resources.images.local_paths()
    }

     /// `exclude` - a folder inside `root` that holds no sources, for example the output
    pub fn index_dir_common(root: &Path, exclude: Option<&Path>) -> io::Result<(Project, Vec<ProjectWarning>)> {
        let mut collected = Collected::default();
        walk(root, exclude, &mut collected)?;

        let mut project = Project::new();
        let mut warns = Vec::new();
        for (key, local, global) in keyed(root, collected.images)? {
            let size = imagesize::size(&global).ok().map(|s| (s.width as u32, s.height as u32));
            if let Err(w) = project.add_image(&key, &local, size) {
                warns.push(w);
            }
        }
        for (key, local, global) in keyed(root, collected.files)? {
            let text = fs::read_to_string(&global)?;
            if let Err(w) = project.add_file(&key, &local, text) {
                warns.push(w);
            }
        }
        Ok((project, warns))
    }
}

#[derive(Default)]
struct Collected {
    images: Vec<PathBuf>,
    files: Vec<PathBuf>,
}

fn keyed(root: &Path, mut paths: Vec<PathBuf>) -> io::Result<Vec<(String, String, PathBuf)>> {
    paths.sort();

    let names: Vec<(String, String)> = {
        let segs: Vec<Vec<&str>> = paths
            .iter()
            .map(|p| split_to_path_segs(root, p))
            .collect::<io::Result<_>>()?;
        let common = common_dir_prefix(&segs);
        segs.iter().map(|s| (segs_to_key(&s[common..]), s.join("/"))).collect()
    };
    Ok(names.into_iter().zip(paths).map(|((key, local), global)| (key, local, global)).collect())
}

fn walk(dir: &Path, exclude: Option<&Path>, out: &mut Collected) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;

        if ft.is_dir() {
            if exclude != Some(path.as_path()) {
                walk(&path, exclude, out)?;
            }
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
    fn index_dir_common_strips_prefix_and_reports_collisions() {
        let root = std::env::temp_dir().join("rush_lib_index_dir_common");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/guide")).unwrap();
        fs::write(root.join("docs/intro.rush"), "текст").unwrap();
        fs::write(root.join("docs/guide/install.rush"), "").unwrap();
        fs::write(root.join("docs/pic.jpg"), "").unwrap();
        fs::write(root.join("docs/pic.png"), "не картинка").unwrap();
        let (project, w) = Project::index_dir_common(&root, None).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(w, vec![ProjectWarning::FileNameCollision("pic".into())]);
        let res = &project.resources;
        let intro = res.get_file_id("intro").unwrap();
        assert_eq!(res.files.local_path(intro), Path::new("docs/intro.rush"));
        assert_eq!(res.files.data(intro), "текст");
        let pic = res.get_image_id("pic").unwrap();
        assert_eq!(*res.images.data(pic), None);
        assert_eq!(project.images().collect::<Vec<_>>(), vec!["docs/pic.jpg"]);
    }

    #[test]
    fn index_dir_common_skips_excluded_dir() {
        let root = std::env::temp_dir().join("rush_lib_index_exclude");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("out")).unwrap();
        fs::write(root.join("pic.png"), "").unwrap();
        fs::write(root.join("out/pic.png"), "").unwrap();
        let (project, w) = Project::index_dir_common(&root, Some(&root.join("out"))).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert!(w.is_empty(), "{w:?}");
        assert_eq!(project.images().collect::<Vec<_>>(), vec!["pic.png"]);
    }
}
