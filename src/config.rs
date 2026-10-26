/// Document styling, independent of the output format
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DocStyle {
    #[default]
    Gost7_32,
    Modern,
    Light,
}

/// Output format: semantic (`Html`, `Markdown`) or paged (`HtmlPaged`, `Pdf`)
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
