use crate::project::ProjectWarning;
use crate::resources::{FileId, Id, Resources};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ContentEntry {
    pub(crate) name: String,
    /// path, normalized the same way as in rush files
    pub(crate) key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirEntry {
    pub(crate) name: String,
    pub(crate) key: String,
}

impl DirEntry {
    pub(crate) fn normalize(key: &str) -> String {
        key.replace('\\', "/").split('/').filter(|s| !s.is_empty() && *s != ".").collect::<Vec<_>>().join("/")
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct ContentItem {
    pub(crate) name: String,
    pub(crate) file: Id<FileId>,
    pub(crate) depth: usize,
    pub(crate) dirs: Vec<String>,
    pub(crate) opens: usize,
    pub(crate) heading: String,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct Content {
    pub(crate) items: Vec<ContentItem>,
}

fn dir_chain(local: &str, dirs: &[DirEntry]) -> Vec<(String, String)> {
    let segs: Vec<&str> = local.split('/').filter(|s| !s.is_empty()).collect();
    let Some((_, parents)) = segs.split_last() else { return Vec::new() };
    (1..=parents.len()).map(|n| {
        let key = parents[..n].join("/");
        let name = dirs.iter().find(|d| d.key == key).map_or_else(|| parents[n - 1].to_string(), |d| d.name.clone());
        (key, name)
    }).collect()
}

impl Content {
    pub(crate) fn resolve(entries: &[ContentEntry], dirs: &[DirEntry], res: &Resources) -> (Content, Vec<ProjectWarning>) {
        let mut warns = Vec::new();
        let mut items: Vec<ContentItem> = Vec::new();
        let mut prev: Vec<(String, String)> = Vec::new();
        for entry in entries {
            let Some(file) = res.get_file_id(&entry.key) else {
                warns.push(ProjectWarning::UnknownContentPath(entry.key.clone()));
                continue;
            };
            if items.iter().any(|i| i.file == file) {
                warns.push(ProjectWarning::DuplicateContent(entry.key.clone()));
                continue;
            }
            let depth = entry.key.split('/').filter(|s| !s.is_empty()).count();
            let local = res.files.local_path(file).to_string_lossy().replace('\\', "/");
            let chain = dir_chain(&local, dirs);
            let same = chain.iter().zip(&prev).take_while(|(a, b)| a.0 == b.0).count();
            let opens = chain.len() - same;
            let names: Vec<String> = chain.iter().map(|(_, name)| name.clone()).collect();
            let mut parts: Vec<&str> = names[same..].iter().map(String::as_str).collect();
            parts.push(&entry.name);
            let heading = parts.join("; ");
            items.push(ContentItem { name: entry.name.clone(), file, depth, dirs: names, opens, heading });
            prev = chain;
        }
        (Content { items }, warns)
    }

    pub(crate) fn title_for(&self, file: Id<FileId>) -> Option<&str> {
        self.items.iter().find(|i| i.file == file).map(|i| i.name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;

    #[test]
    fn resolve_depth_and_warnings() {
        let mut project = Project::new();
        for path in ["intro.rush", "guide/install.rush", "guide/deep/x.rush"] {
            project.add_file(path, path, String::new()).unwrap();
        }
        for (name, key) in [
            ("Введение", "intro"),
            ("Установка", "guide/install.rush"),
            ("Глубоко", "guide/deep/x"),
            ("Нет", "missing"),
            ("Повтор", "intro.rush"),
        ] {
            project.add_content(name.into(), key.into());
        }
        let res = &project.resources;

        let (content, w) = Content::resolve(&project.content, &project.dirs, res);

        let got: Vec<(&str, usize)> = content.items.iter().map(|i| (i.name.as_str(), i.depth)).collect();
        assert_eq!(got, vec![("Введение", 1), ("Установка", 2), ("Глубоко", 3)]);
        assert_eq!(w, vec![
            ProjectWarning::UnknownContentPath("missing".into()),
            ProjectWarning::DuplicateContent("intro.rush".into()),
        ]);
        let intro = res.get_file_id("intro").unwrap();
        assert_eq!(content.title_for(intro), Some("Введение"));
    }

    #[test]
    fn resolve_dir_chains_and_opens() {
        let mut project = Project::new();
        for path in ["a/b.rush", "a/c.rush", "a/b/c.rush", "x.rush", "d/y.rush", "a/z.rush"] {
            project.add_file(path, path, String::new()).unwrap();
        }
        for (name, key) in [("B", "a/b.rush"), ("C", "a/c.rush"), ("BC", "a/b/c.rush"), ("X", "x.rush"), ("Y", "d/y.rush"), ("Z", "a/z.rush")] {
            project.add_content(name.into(), key.into());
        }
        project.add_dir("Раздел A".into(), "a/".into());

        let (content, w) = Content::resolve(&project.content, &project.dirs, &project.resources);
        assert!(w.is_empty(), "{w:?}");
        let got: Vec<(Vec<&str>, usize, &str)> = content.items.iter()
            .map(|i| (i.dirs.iter().map(String::as_str).collect(), i.opens, i.heading.as_str()))
            .collect();
        assert_eq!(got, vec![
            (vec!["Раздел A"], 1, "Раздел A; B"),
            (vec!["Раздел A"], 0, "C"),
            (vec!["Раздел A", "b"], 1, "b; BC"),
            (vec![], 0, "X"),
            (vec!["d"], 1, "d; Y"),
            (vec!["Раздел A"], 1, "Раздел A; Z"),
        ]);
    }
}
