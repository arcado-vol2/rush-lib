use std::{fs, io, path::{Path, PathBuf}};
use serde::Deserialize;

pub(crate) const PROJECT_FILE: &str = "project.toml";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocStyle {
    #[default]
    GostLike,
    Gost,
    Modern,
    Light,
}

impl DocStyle {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "gostlike" => Some(Self::GostLike),
            "gost" => Some(Self::Gost),
            "modern" => Some(Self::Modern),
            "light" => Some(Self::Light),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocOut {
    #[default]
    Html,
    Markdown,
    Pdf,
    Odt,
    Docx
}

impl DocOut {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "html" => Some(Self::Html),
            "markdown" => Some(Self::Markdown),
            "pdf" => Some(Self::Pdf),
            "odt" => Some(Self::Odt),
            "docx" => Some(Self::Docx),
            _ => None    
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Config {
    pub(crate) style: DocStyle,
    pub(crate) out: DocOut,
    pub(crate) input: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) content: Vec<ContentEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ContentEntry {
    pub(crate) name: String,
    pub(crate) path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            style: DocStyle::default(),
            out: DocOut::default(),
            input: PathBuf::from("src"),
            output: PathBuf::from("out"),
            content: Vec::new(),
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum ConfigWarning {
    Unreadable(String),
    InvalidToml(String),
    WrongType { key: &'static str },
    UnknownValue { key: &'static str, value: String },
    EmptyPath { key: &'static str },
    ContentMissingKey { index: usize, key: &'static str },
    ContentWrongType { index: usize, key: &'static str },
    ContentEmptyPath { index: usize },
    UnknownContentPath(String),
    DuplicateContent(String),
}


#[derive(Deserialize, Default)]
#[serde(default)]
struct RawProjectFile {
    document: RawDocument,
    build: RawBuild,
    content: Option<toml::Value>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawDocument {
    style: Option<toml::Value>,
    out: Option<toml::Value>
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawBuild {
    input: Option<toml::Value>,
    output: Option<toml::Value>
}

impl Config {
    pub(crate) fn load(root: &Path) -> (Config, Vec<ConfigWarning>) {
        let (mut project, warnings) = match fs::read_to_string(root.join(PROJECT_FILE)) {
            Ok(src) => Self::parse(&src),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (Config::default(), Vec::new()),
            Err(e) => (Config::default(), vec![ConfigWarning::Unreadable(e.to_string())]),
        };
        project.input = root.join(&project.input);
        project.output = root.join(&project.output);
        (project, warnings)
    }

    pub(crate) fn parse(src: &str) -> (Config, Vec<ConfigWarning>) {
        let mut warnings = Vec::new();
        let raw: RawProjectFile = match toml::from_str(src) {
            Ok(raw) => raw,
            Err(e) => {
                warnings.push(ConfigWarning::InvalidToml(e.to_string()));
                return (Config::default(), warnings);
            }
        };

        let mut out = Config::default();
        if let Some(style) = enum_param(raw.document.style, "document.style", DocStyle::from_name, &mut warnings) {
            out.style = style;
        }
        if let Some(doc_out) = enum_param(raw.document.out, "document.out", DocOut::from_name, &mut warnings) {
            out.out = doc_out;
        }
        if let Some(input) = path_param(raw.build.input, "build.input", &mut warnings) {
            out.input = input;
        }
        if let Some(output) = path_param(raw.build.output, "build.output", &mut warnings) {
            out.output = output;
        }
        if let Some(content) = raw.content {
            out.content = content_param(content, &mut warnings);
        }
        (out, warnings)
    }
}

fn path_param(
    value: Option<toml::Value>,
    key: &'static str,
    warnings: &mut Vec<ConfigWarning>,
) -> Option<PathBuf> {
    match value? {
        toml::Value::String(s) if s.is_empty() => {
            warnings.push(ConfigWarning::EmptyPath { key });
            None
        }
        toml::Value::String(s) => Some(PathBuf::from(s)),
        _ => {
            warnings.push(ConfigWarning::WrongType { key });
            None
        }
    }
}

fn content_param(value: toml::Value, warnings: &mut Vec<ConfigWarning>) -> Vec<ContentEntry> {
    let toml::Value::Array(items) = value else {
        warnings.push(ConfigWarning::WrongType { key: "content" });
        return Vec::new();
    };
    let mut out = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        let toml::Value::Table(mut table) = item else {
            warnings.push(ConfigWarning::WrongType { key: "content" });
            continue;
        };
        let mut get = |key: &'static str| match table.remove(key) {
            Some(toml::Value::String(s)) => Some(s),
            Some(_) => {
                warnings.push(ConfigWarning::ContentWrongType { index, key });
                None
            }
            None => {
                warnings.push(ConfigWarning::ContentMissingKey { index, key });
                None
            }
        };
        let (name, path) = (get("name"), get("path"));
        let (Some(name), Some(path)) = (name, path) else { continue };
        if path.is_empty() {
            warnings.push(ConfigWarning::ContentEmptyPath { index });
            continue;
        }
        out.push(ContentEntry { name, path });
    }
    out
}

fn enum_param<T>(
    value: Option<toml::Value>,
    key: &'static str,
    from_name: fn(&str) -> Option<T>,
    warnings: &mut Vec<ConfigWarning>,
) -> Option<T> {
    match value? {
        toml::Value::String(s) => {
            let parsed = from_name(&s);
            if parsed.is_none() {
                warnings.push(ConfigWarning::UnknownValue { key, value: s });
            }
            parsed
        }
        _ => {
            warnings.push(ConfigWarning::WrongType { key });
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str, style: DocStyle, warnings: Vec<ConfigWarning>) {
        let (out, w) = Config::parse(src);
        assert_eq!(out.style, style, "вход: {src:?}");
        assert_eq!(w, warnings, "вход: {src:?}");
    }

    #[test]
    fn valid_values() {
        check("[document]\nstyle = \"gostlike\"", DocStyle::GostLike, vec![]);
        check("[document]\nstyle = \"gost\"", DocStyle::Gost, vec![]);
        check("[document]\nstyle = \"modern\"", DocStyle::Modern, vec![]);
        check("[document]\nstyle = \"light\"", DocStyle::Light, vec![]);
    }

    #[test]
    fn missing_is_default_without_warnings() {
        check("", DocStyle::GostLike, vec![]);
        check("[document]", DocStyle::GostLike, vec![]);
        check("[other]\nx = 1", DocStyle::GostLike, vec![]);
    }

    #[test]
    fn unknown_value_falls_back() {
        check(
            "[document]\nstyle = \"Gost\"",
            DocStyle::GostLike,
            vec![ConfigWarning::UnknownValue { key: "document.style", value: "Gost".into() }],
        );
    }

    #[test]
    fn wrong_type_falls_back() {
        check("[document]\nstyle = 5", DocStyle::GostLike, vec![ConfigWarning::WrongType { key: "document.style" }]);
    }

    #[test]
    fn broken_toml_falls_back() {
        let (out, w) = Config::parse("[document\nstyle = ");
        assert_eq!(out, Config::default());
        assert!(matches!(w.as_slice(), [ConfigWarning::InvalidToml(_)]));
    }

    fn check_out(src: &str, doc_out: DocOut, warnings: Vec<ConfigWarning>) {
        let (out, w) = Config::parse(src);
        assert_eq!(out.out, doc_out, "вход: {src:?}");
        assert_eq!(w, warnings, "вход: {src:?}");
    }

    #[test]
    fn out_valid_values() {
        check_out("[document]
out = \"html\"", DocOut::Html, vec![]);
        check_out("[document]
out = \"markdown\"", DocOut::Markdown, vec![]);
        check_out("[document]
out = \"pdf\"", DocOut::Pdf, vec![]);
        check_out("[document]
out = \"odt\"", DocOut::Odt, vec![]);
        check_out("[document]
out = \"docx\"", DocOut::Docx, vec![]);
    }

    #[test]
    fn out_missing_is_html() {
        check_out("", DocOut::Html, vec![]);
        check_out("[document]
style = \"gost\"", DocOut::Html, vec![]);
    }

    #[test]
    fn out_bad_falls_back_to_html() {
        check_out(
            "[document]
out = \"exe\"",
            DocOut::Html,
            vec![ConfigWarning::UnknownValue { key: "document.out", value: "exe".into() }],
        );
        check_out("[document]
out = true", DocOut::Html, vec![ConfigWarning::WrongType { key: "document.out" }]);
    }

    #[test]
    fn style_and_out_together() {
        let (out, w) = Config::parse("[document]
style = \"modern\"
out = \"pdf\"");
        assert_eq!(out, Config { style: DocStyle::Modern, out: DocOut::Pdf, ..Config::default() });
        assert!(w.is_empty());
    }

    #[test]
    fn missing_file() {
        let root = Path::new("definitely/not/a/project");
        let (out, w) = Config::load(root);
        assert_eq!(out, Config { input: root.join("src"), output: root.join("out"), ..Config::default() });
        assert!(w.is_empty());
    }

    fn check_paths(src: &str, input: &str, output: &str, warnings: Vec<ConfigWarning>) {
        let (out, w) = Config::parse(src);
        assert_eq!(out.input, PathBuf::from(input), "вход: {src:?}");
        assert_eq!(out.output, PathBuf::from(output), "вход: {src:?}");
        assert_eq!(w, warnings, "вход: {src:?}");
    }

    #[test]
    fn build_paths() {
        check_paths("", "src", "out", vec![]);
        check_paths("[build]\ninput = \"docs\"\noutput = \"dist/html\"", "docs", "dist/html", vec![]);
        check_paths("[build]\ninput = 1\noutput = \"\"", "src", "out", vec![
            ConfigWarning::WrongType { key: "build.input" },
            ConfigWarning::EmptyPath { key: "build.output" },
        ]);
    }

    fn entry(name: &str, path: &str) -> ContentEntry {
        ContentEntry { name: name.into(), path: path.into() }
    }

    #[test]
    fn content_valid() {
        let (out, w) = Config::parse("[[content]]
name = \"Введение\"
path = \"intro\"
[[content]]
name = \"Установка\"
path = \"guide/install.rush\"");
        assert_eq!(out.content, vec![entry("Введение", "intro"), entry("Установка", "guide/install.rush")]);
        assert!(w.is_empty());
    }

    #[test]
    fn content_missing_is_empty() {
        let (out, w) = Config::parse("[document]
style = \"gost\"");
        assert!(out.content.is_empty());
        assert!(w.is_empty());
    }

    #[test]
    fn content_bad_entries_skipped() {
        let (out, w) = Config::parse("[[content]]
path = \"a\"
[[content]]
name = 1
path = \"b\"
[[content]]
name = \"C\"
path = \"\"
[[content]]
name = \"D\"
path = \"d\"");
        assert_eq!(out.content, vec![entry("D", "d")]);
        assert_eq!(w, vec![
            ConfigWarning::ContentMissingKey { index: 0, key: "name" },
            ConfigWarning::ContentWrongType { index: 1, key: "name" },
            ConfigWarning::ContentEmptyPath { index: 2 },
        ]);
    }

    #[test]
    fn content_wrong_type() {
        let (out, w) = Config::parse("content = \"a\"");
        assert!(out.content.is_empty());
        assert_eq!(w, vec![ConfigWarning::WrongType { key: "content" }]);
    }

    #[test]
    fn build_paths_resolve_from_root() {
        let root = std::env::temp_dir().join("simple_ru_help_build_paths");
        fs::create_dir_all(&root).unwrap();
        let abs = std::env::temp_dir().join("abs_out");
        let src = format!("[build]\ninput = \"text\"\noutput = '{}'", abs.display());
        fs::write(root.join(PROJECT_FILE), src).unwrap();

        let (out, w) = Config::load(&root);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(out.input, root.join("text"));
        assert_eq!(out.output, abs);
        assert!(w.is_empty());
    }
}
