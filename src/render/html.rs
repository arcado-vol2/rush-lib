use crate::render::common::Backend;
use crate::parse::rush::{Code, Comment, Formula, Header, Image, InlineEl, InnerLink, List, Node, Note, OuterLink, Paragraph, Span, Table, Rush};
use crate::parse::common::{Color, RefKind};
use std::fmt::{self, Write};
struct HtmlBackend;

impl HtmlBackend {
    fn escape(out: &mut impl Write, s: &str) -> fmt::Result {
        let mut last = 0;
        for (i, c) in s.char_indices() {
            let rep = match c {
                '<' => "&lt;",
                '>' => "&gt;",
                '&' => "&amp;",
                '"' => "&quot;",
                _ => continue
            };
            out.write_str(&s[last..i])?;
            out.write_str(rep)?;
            last = i + 1;
        }
        out.write_str(&s[last..])
    }

    fn write_err(out: &mut impl Write) -> fmt::Result {
        out.write_str("<span class=\"error\">??</span>")
    }
}

impl Backend for HtmlBackend {
    fn span(out: &mut impl Write, inp: Vec<Span>) -> fmt::Result {
        for span in inp{
            let style = &span.style;
            let tags = [
                (style.is_bold, "<b>", "</b>"),
                (style.is_italic, "<i>", "</i>"),
                (style.is_mono, "<span class=\"mono\">", "</mono>"),
                (style.is_strike, "<s>", "</s>"),
                (style.is_under, "<u>", "</u>")
            ];

            for &(on, open, _) in &tags {
                if on {out.write_str(open)?}
            }
            if let Some(col) = &style.color {
                write!(out, "<span class=\"color\" style=\"color:{}\">", col.to_hex_tag())?;
            }
            if let Some(link) = &style.link{
                todo!("Link here")
            }

            for el in span.text {
                match el {
                    InlineEl::String(s) => Self::escape(out, s)?,
                    InlineEl::Space => out.write_char(' ')?,
                    InlineEl::InnerLink(link ) => {
                        if let Some(link) = link {
                            match link.kind {
                                RefKind::Table => write!(out, "<a class=\"ref-table\" href=\"#t-{0}\">{0}</a>", link.index)?,
                                RefKind::Code  => write!(out, "<a class=\"ref-table\" href=\"#c-{0}\">{0}</a>", link.index)?,
                                RefKind::Formula  => write!(out, "<a class=\"ref-table\" href=\"#f-{0}\">{0}</a>", link.index)?,
                                RefKind::Image  => write!(out, "<a class=\"ref-table\" href=\"#i-{0}\">{0}</a>", link.index)?,
                            }
                        } else {
                            Self::write_err(out)?;
                        }
                    },
                    InlineEl::Note(note) => {
                        if let Some(i) = note {
                            write!(out, "<a class=\"ref-note\" href=\"#n-{0}\">{0}</a> ", i)?;
                        } else {
                            Self::write_err(out)?;
                        }
                    }
                }
            }

            if style.link.is_some() { todo!("link close here")}
            if style.color.is_some() {out.write_str("</span>")?}
            for &(on, _, close) in tags.iter().rev() {
                if on {
                    out.write_str(close)?;
                }
            }
        }

        Ok(())
    }


    fn paragraph(out: &mut impl Write, paragraph: Paragraph) -> std::fmt::Result {
        out.write_str("<p>")?;
        Self::span(out, paragraph.content)?;
        out.write_str("</p>")?;
        Ok(())
    }

    fn code(out: &mut impl Write, code: Code) -> fmt::Result {
        unimplemented!("code");
    }
    
    fn comment(out: &mut impl Write, comment: Comment) -> fmt::Result {
        unimplemented!("comm");
    }

    fn formula(out: &mut impl Write, formula: Formula) -> fmt::Result {
        unimplemented!("form");
    }

    fn header(out: &mut impl Write, header: Header) -> fmt::Result {
        let level = header.level;
        debug_assert!((1..=5).contains(&level), "Header can be only 1-5 level");
        let index = &header.index[..level as usize];

        write!(out, "<h{level} id=\"h")?;
        for i in index {
            write!(out, "-{i}")?;
        }
        out.write_str("\"><span class=\"index\">")?;
        for (l, i) in index.iter().enumerate() {
            write!(out, "<span class=\"l{0}\">{i}<span class=\"dot\">.</span></span>", l + 1)?;
        }
        out.write_str("</span><span class=\"body\">")?;
        Self::span(out, header.content)?;
        write!(out, "</span></h{level}>")?;
        Ok(())
    }

    fn image(out: &mut impl Write, image: Image) -> fmt::Result {
        unimplemented!("imgae");
    }

    fn list(out: &mut impl Write, list: List) -> fmt::Result {
        unimplemented!("list");
    }

    fn note(out: &mut impl Write, note: Note) -> fmt::Result {
        out.write_str("<div class=\"note\">")?;
        for note in note.content {
            write!(out, "<div class=\"item\" id=\"n-{0}\"><div class=\"index\">{0}) </div><div class=\"body\">", note.index)?;
            Self::span(out, note.content)?;
            out.write_str("</div></div>")?;
        }
        out.write_str("</div>")?;
        Ok(())
    }

    fn table(out: &mut impl Write, table: Table) -> fmt::Result {
        unimplemented!("table");
    }

    fn render(file: Rush) -> Result<String, fmt::Error> {
        let mut out = "<!DOCTYPE html> <html lang=\"ru\"><head><meta charset=\"UTF-8\"><link rel=\"stylesheet\", href=\"gostlike_7_32.css\"></head><body><main>".to_string();
        for node in file.nodes {
            Self::block(&mut out, node)?;
        }
        out.write_str("</main></body></html>")?;
        Ok(out)
    }
}


#[cfg(test)] mod test {
    use std::{collections::HashMap, fs::File, io::Write};
    use crate::parse::rush::*;
    use crate::parse::link_project::*;
    use crate::render::common::Backend;
    use crate::render::html::HtmlBackend;
    #[test]
    fn main() {
        let source = "Насрал<в рот> тебе";
        let proj = GlobalProject { images: HashMap::new(), files: HashMap::new() };
        let test = Rush::new(source, &proj);
        println!("{test:?}");
        let a = HtmlBackend::render(test);
        if let Ok(s) = a {
            let mut file = File::create("lazy_test.html").unwrap();
            file.write_all(s.as_bytes()).unwrap();
        }
    }
}