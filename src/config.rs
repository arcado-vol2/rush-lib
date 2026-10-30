#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DocStyle {
    #[default]
    Gost7_32,
    Modern,
    Light,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DocOut {
    #[default]
    Html,
    HtmlPaged,
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
