use std::{collections::HashMap, ffi::OsStr, fs, io, path::{Path, PathBuf}};

const IMG_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];
const FILE_EXTENSIONS: &[&str] = &["rush"];




fn walk(dir: &Path, images: &mut HashMap<String, PathBuf>, files: &mut HashMap<String, PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_dir() {
            walk(&entry.path(), images, files)?;
        } else if ft.is_file() {
            if let Some(ext) = entry.path().extension() {
                if IMG_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
                    images.insert(entry.path().file_name().unwrap().to_string_lossy().into_owned(), entry.path());
                } else if FILE_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
                    files.insert(entry.path().file_name().unwrap().to_string_lossy().into_owned(), entry.path());
                }
            }
        }
    }
    Ok(())
}


fn create_project(dir: &Path){
    let mut out: Vec<PathBuf> = Vec::new();
    walk(dir,  &mut out);
    
}


#[cfg(test)] 
mod test {
    use std::path::PathBuf;
    use crate::parse::link_project::walk;

    #[test]
    fn link_project_check_walk() {
        let mut out: Vec<PathBuf> = Vec::new();
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("project_2");
        let res = walk(&test_dir.as_path(),  &mut out);
        println!("{res:?} -- {out:?}");
    }
}