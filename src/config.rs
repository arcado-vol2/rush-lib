#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DocStyle {
    Gost7_32,
    Modern,
    #[default]
    Blog,
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
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub style: DocStyle,
    pub out: DocOut,
    /// one output file for the whole project, otherwise one per rush file
    pub single: bool,
    pub no_js: bool,
    pub theme: Theme,
}
