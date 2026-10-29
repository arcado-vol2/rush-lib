use crate::project::ProjectWarning;
use crate::resources::{FileId, Id, Resources};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ContentEntry {
    pub(crate) name: String,
    /// path, normalized the same way as in rush files
    pub(crate) key: String,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ContentItem {
    pub(crate) name: String,
    pub(crate) file: Id<FileId>,
    pub(crate) depth: usize,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct Content {
    pub(crate) items: Vec<ContentItem>,
}

impl Content {
    pub(crate) fn resolve(entries: &[ContentEntry], res: &Resources) -> (Content, Vec<ProjectWarning>) {
        let mut warns = Vec::new();
        let mut items: Vec<ContentItem> = Vec::new();
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
            items.push(ContentItem { name: entry.name.clone(), file, depth });
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

        let (content, w) = Content::resolve(&project.content, res);

        let got: Vec<(&str, usize)> = content.items.iter().map(|i| (i.name.as_str(), i.depth)).collect();
        assert_eq!(got, vec![("Введение", 1), ("Установка", 2), ("Глубоко", 3)]);
        assert_eq!(w, vec![
            ProjectWarning::UnknownContentPath("missing".into()),
            ProjectWarning::DuplicateContent("intro.rush".into()),
        ]);
        let intro = res.get_file_id("intro").unwrap();
        assert_eq!(content.title_for(intro), Some("Введение"));
    }
}
