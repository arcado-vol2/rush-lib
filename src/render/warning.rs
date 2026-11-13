use crate::config::{DocOut, DocStyle};
use crate::parse::common::RefKind;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Feature {
    Paging,
    Color,
    Underline,
    Strike,
    MergedCells,
    ColumnWidths,
    ImageScale,
    Formula,
    Notes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// the source refers to something missing or has an error
    Content,
    /// the style cannot format something in the source
    Style(DocStyle),
    /// the output format cannot express something in the source
    Backend(DocOut),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderWarningKind {
    UnresolvedRef { kind: RefKind, name: String },
    UnresolvedNote(u16),
    UnresolvedImage(String),
    InvalidUrl(String),
    InvalidFormula(String),
    FileNotInContent(String),
    ImageSizeUnknown(String),
    BlockTallerThanPage,
    Unsupported(Feature),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderWarning {
    pub origin: Origin,
    pub file: Option<String>,
    pub kind: RenderWarningKind,
}

impl RenderWarning {
    pub(crate) fn content(file: &str, kind: RenderWarningKind) -> Self {
        Self { origin: Origin::Content, file: Some(file.to_string()), kind }
    }
}
