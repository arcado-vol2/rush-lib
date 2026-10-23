use crate::config::{ConfigWarning, ContentEntry};
use crate::resources::{FileId, Id, Resources};

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
    pub(crate) fn resolve(entries: &[ContentEntry], res: &Resources) -> (Content, Vec<ConfigWarning>) {
        let mut warns = Vec::new();
        let mut items: Vec<ContentItem> = Vec::new();
        for entry in entries {
            let Some(file) = res.get_file_id(&entry.path) else {
                warns.push(ConfigWarning::UnknownContentPath(entry.path.clone()));
                continue;
            };
            if items.iter().any(|i| i.file == file) {
                warns.push(ConfigWarning::DuplicateContent(entry.path.clone()));
                continue;
            }
            let depth = entry.path.split('/').filter(|s| !s.is_empty()).count();
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
    use std::fs;

    fn entry(name: &str, path: &str) -> ContentEntry {
        ContentEntry { name: name.into(), path: path.into() }
    }

    #[test]
    fn resolve_depth_and_warnings() {
        let root = std::env::temp_dir().join("simple_ru_help_content_resolve");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("guide/deep")).unwrap();
        fs::write(root.join("intro.rush"), "").unwrap();
        fs::write(root.join("guide/install.rush"), "").unwrap();
        fs::write(root.join("guide/deep/x.rush"), "").unwrap();
        let (res, _) = Resources::build_index(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();

        let (content, w) = Content::resolve(&[
            entry("Введение", "intro"),
            entry("Установка", "guide/install.rush"),
            entry("Глубоко", "guide/deep/x"),
            entry("Нет", "missing"),
            entry("Повтор", "intro.rush"),
        ], &res);

        let got: Vec<(&str, usize)> = content.items.iter().map(|i| (i.name.as_str(), i.depth)).collect();
        assert_eq!(got, vec![("Введение", 1), ("Установка", 2), ("Глубоко", 3)]);
        assert_eq!(w, vec![
            ConfigWarning::UnknownContentPath("missing".into()),
            ConfigWarning::DuplicateContent("intro.rush".into()),
        ]);
        let intro = res.get_file_id("intro").unwrap();
        assert_eq!(content.title_for(intro), Some("Введение"));
    }
}
