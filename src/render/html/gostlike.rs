use super::{escape, CaptionKind, HtmlStyle, Page};
use crate::parse::common::RefKind;
use std::fmt::{self, Write};

const CSS: &str = include_str!("../../templates/gostlike_7_32/style.css");
const CSS_NAME: &str = "style.css";

const NAV_MAX_LEVEL: usize = 3;

pub(crate) struct GostLike;

pub(crate) fn gost_caption(kind: CaptionKind, index: u16, named: bool) -> String {
    match (kind, named) {
        (CaptionKind::Formula, _) => format!("({index})"),
        (CaptionKind::Code, true) => format!("Листинг\u{a0}{index}\u{a0}—\u{a0}"),
        (CaptionKind::Code, false) => format!("Листинг\u{a0}{index}"),
        (CaptionKind::Image, true) => format!("Рисунок\u{a0}{index}\u{a0}—"),
        (CaptionKind::Image, false) => format!("Рисунок\u{a0}{index}"),
        (CaptionKind::Table, true) => format!("Таблица\u{a0}{index}\u{a0}—\u{a0}"),
        (CaptionKind::Table, false) => format!("Таблица\u{a0}{index}"),
    }
}

impl HtmlStyle for GostLike {
    fn css(&self) -> &'static str {
        CSS
    }

    fn css_name(&self) -> &'static str {
        CSS_NAME
    }

    fn caption(&self, out: &mut dyn Write, kind: CaptionKind, index: u16, named: bool) -> fmt::Result {
        out.write_str(&gost_caption(kind, index, named).replace('\u{a0}', "&nbsp;"))
    }

    fn inner_link(&self, out: &mut dyn Write, _kind: RefKind, index: u16) -> fmt::Result {
        write!(out, "{index}")
    }

    fn note_ref(&self, out: &mut dyn Write, index: u16) -> fmt::Result {
        write!(out, "{index}")
    }

    fn head(&self, out: &mut dyn Write, page: &Page) -> fmt::Result {
        out.write_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>")?;
        escape(out, page.title)?;
        out.write_str("</title><link rel=\"stylesheet\" href=\"")?;
        escape(out, &page.css_href)?;
        out.write_str("\">")
    }

    fn page(&self, out: &mut dyn Write, page: &Page, main: &str) -> fmt::Result {
        out.write_str("<header class=\"title\">")?;
        escape(out, page.title)?;
        out.write_str("</header>")?;
        if !page.nav.is_empty() {
            out.write_str("<nav class=\"toc\">")?;
            for item in &page.nav {
                let level = item.depth.clamp(1, NAV_MAX_LEVEL) - 1;
                out.write_str("<a class=\"item")?;
                if item.current {
                    out.write_str(" current\" aria-current=\"page")?;
                }
                write!(out, "\" style=\"--level: {level}\" href=\"")?;
                escape(out, &item.href)?;
                out.write_str("\">")?;
                if level > 0 {
                    out.write_str("–&nbsp;")?;
                }
                escape(out, item.name)?;
                out.write_str("</a>")?;
            }
            out.write_str("</nav>")?;
        }
        out.write_str("<main>")?;
        out.write_str(main)?;
        out.write_str("</main>")
    }
}
