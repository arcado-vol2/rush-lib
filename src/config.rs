#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DocStyle {
    #[default]
    GostLike,
    Gost,
    Modern,
    Light,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DocOut {
    #[default]
    Html,
    Markdown,
    Pdf,
    Odt,
    Docx
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub style: DocStyle,
    pub out: DocOut,
}
