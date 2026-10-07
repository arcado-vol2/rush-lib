use std::{collections::{HashMap, hash_map::Entry}, fs, io, path::{Component, Path, PathBuf}};

const IMG_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];
const FILE_EXTENSIONS: &[&str] = &["rush"];


#[derive(Default, Debug, PartialEq)]
pub(crate) struct OuterLinks {
    pub(crate) images: HashMap<String, PathBuf>,
    pub(crate) files: HashMap<String, PathBuf>
}

impl OuterLinks {
    pub fn build_index(root: &Path) -> io::Result<OuterLinks> {
        let mut project = OuterLinks::default();
        walk(root, root, &mut project)?;
        Ok(project)
    }
}

fn walk(root: &Path, dir: &Path, index: &mut OuterLinks) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?; 
        let path = entry.path();
        let ft = entry.file_type()?;

        if ft.is_dir() {
            walk(root, &path, index)?;
        }
        if !ft.is_file() {
            continue;
        }

        let Some(ext) = path.extension() else {continue};
        let map = if IMG_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)){
            &mut index.images
        } else if FILE_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
            &mut index.files
        } else {
            continue;
        };

        let key = path_to_key(root, &path)?;
        match map.entry(key) {
            Entry::Vacant(v) => {
                v.insert(path);
            }
            Entry::Occupied(o) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("Names confict: {} and {}", o.get().display(), path.display()),
                ));
            }
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
                format!("Name {} is not UTF-8", path.display())
            )
        }),
        _ => unreachable!("must be only normal components after strip")
    }).collect()
}


fn path_to_key(root: &Path, path: &Path) -> io::Result<String> {
    let mut segments = split_to_path_segs(root, path)?;
    if let Some(last) = segments.last_mut() {
        *last = Path::new(*last).file_stem().and_then(|s| s.to_str()).unwrap_or(last);
    }
    Ok(segments.join("/"))
}

