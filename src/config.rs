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
    Markdown,
    Pdf,
    Odt,
    Docx
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub style: DocStyle,
    pub out: DocOut,
    /// one output file for the whole project, otherwise one per rush file
    pub single: bool,
}
