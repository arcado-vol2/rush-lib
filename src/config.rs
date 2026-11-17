pub trait ConfigConst {
    fn out_name(&self) -> &'static str;
    fn from_name(name: &str) -> Option<Self> where Self: Sized;
}


#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DocStyle {
    Gost7_32,
    Modern,
    #[default]
    Blog,
}

impl ConfigConst for DocStyle {
    fn out_name(&self) -> &'static str {
        match &self {
            DocStyle::Gost7_32 => "gost7-32",
            DocStyle::Modern => "modern",
            DocStyle::Blog => "blog",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "gost7-32" => Some(DocStyle::Gost7_32),
            "modern" => Some(DocStyle::Modern),
            "blog" => Some(DocStyle::Blog),
            _ => None,
        }
    }
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

impl ConfigConst for DocOut {
    fn out_name(&self) -> &'static str {
        match &self {
            DocOut::Html => "html",
            DocOut::Markdown => "markdown",
            DocOut::Pdf => "pdf",
            DocOut::Odt => "odt",
            DocOut::Docx => "docx",
        }
    }

    fn from_name(name: &str) -> Option<Self> where Self: Sized {
        match name {
            "html" => Some(DocOut::Html),
            "markdown" => Some(DocOut::Markdown),
            "pdf" => Some(DocOut::Pdf),
            "odt" => Some(DocOut::Odt),
            "docx" => Some(DocOut::Docx),
            _ => None
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

impl ConfigConst for Theme {
    fn out_name(&self) -> &'static str {
        match &self {
            Theme::Auto => "auto",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }
    
    fn from_name(name: &str) -> Option<Self> where Self: Sized {
        match name {
            "auto" => Some(Theme::Auto),
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }
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
