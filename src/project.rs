use std::{default, fs, io, path::Path};
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

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct Project {
    pub(crate) style: DocStyle,
    pub(crate) out: DocOut,
}


#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProjectWarning {
    Unreadable(String),
    InvalidToml(String),
    WrongType { key: &'static str },
    UnknownValue { key: &'static str, value: String },
}


#[derive(Deserialize, Default)]
#[serde(default)]
struct RawProjectFile {
    document: RawDocument,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawDocument {
    style: Option<toml::Value>,
    out: Option<toml::Value>
}

impl Project {
    pub(crate) fn load(root: &Path) -> (Project, Vec<ProjectWarning>) {
        match fs::read_to_string(root.join(PROJECT_FILE)) {
            Ok(src) => Self::parse(&src),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (Project::default(), Vec::new()),
            Err(e) => (Project::default(), vec![ProjectWarning::Unreadable(e.to_string())]),
        }
    }

    pub(crate) fn parse(src: &str) -> (Project, Vec<ProjectWarning>) {
        let mut warnings = Vec::new();
        let raw: RawProjectFile = match toml::from_str(src) {
            Ok(raw) => raw,
            Err(e) => {
                warnings.push(ProjectWarning::InvalidToml(e.to_string()));
                return (Project::default(), warnings);
            }
        };

        let mut out = Project::default();
        if let Some(style) = enum_param(raw.document.style, "document.style", DocStyle::from_name, &mut warnings) {
            out.style = style;
        }
        if let Some(doc_out) = enum_param(raw.document.out, "document.out", DocOut::from_name, &mut warnings) {
            out.out = doc_out;
        }
        (out, warnings)
    }
}

fn enum_param<T>(
    value: Option<toml::Value>,
    key: &'static str,
    from_name: fn(&str) -> Option<T>,
    warnings: &mut Vec<ProjectWarning>,
) -> Option<T> {
    match value? {
        toml::Value::String(s) => {
            let parsed = from_name(&s);
            if parsed.is_none() {
                warnings.push(ProjectWarning::UnknownValue { key, value: s });
            }
            parsed
        }
        _ => {
            warnings.push(ProjectWarning::WrongType { key });
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(src: &str, style: DocStyle, warnings: Vec<ProjectWarning>) {
        let (out, w) = Project::parse(src);
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
            vec![ProjectWarning::UnknownValue { key: "document.style", value: "Gost".into() }],
        );
    }

    #[test]
    fn wrong_type_falls_back() {
        check("[document]\nstyle = 5", DocStyle::GostLike, vec![ProjectWarning::WrongType { key: "document.style" }]);
    }

    #[test]
    fn broken_toml_falls_back() {
        let (out, w) = Project::parse("[document\nstyle = ");
        assert_eq!(out, Project::default());
        assert!(matches!(w.as_slice(), [ProjectWarning::InvalidToml(_)]));
    }

    fn check_out(src: &str, doc_out: DocOut, warnings: Vec<ProjectWarning>) {
        let (out, w) = Project::parse(src);
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
            vec![ProjectWarning::UnknownValue { key: "document.out", value: "exe".into() }],
        );
        check_out("[document]
out = true", DocOut::Html, vec![ProjectWarning::WrongType { key: "document.out" }]);
    }

    #[test]
    fn style_and_out_together() {
        let (out, w) = Project::parse("[document]
style = \"modern\"
out = \"pdf\"");
        assert_eq!(out, Project { style: DocStyle::Modern, out: DocOut::Pdf });
        assert!(w.is_empty());
    }

    #[test]
    fn missing_file() {
        let (out, w) = Project::load(Path::new("definitely/not/a/project"));
        assert_eq!(out, Project::default());
        assert!(w.is_empty());
    }
}
