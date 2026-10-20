use crate::RenderWarning::UnresolvedImage;
use crate::render::common::{Backend, RenderWarning};
use crate::parse::rush::{Code, Comment, Formula, Header, Image, InlineEl, InnerLink, List, ListEl, ListElKind, Node, Note, OuterLink, Paragraph, Rush, Span, Table, TableEl, Aligment};
use crate::parse::common::{Color, Named, RefKind};
use crate::resources::{FileId, Id, Resources};
use std::fmt::{self, Write};
use std::path::Path;
use url::{ParseError, Url};
use latex2mathml::{latex_to_mathml, DisplayStyle};

pub(crate) const GOSTLIKE_CSS: &str = include_str!("../templates/gostlike_7_32/style.css");
pub(crate) const CSS_NAME: &str = "style.css";

pub(crate) struct HtmlBackend;

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

    fn parse_url(link: &str) -> Option<Url> {
        match Url::parse(link) {
            Ok(url) => Some(url),
            Err(ParseError::RelativeUrlWithoutBase) => Url::parse(&format!("https://{link}")).ok(),
            Err(_) => None,
        }
    }

    fn open_link(out: &mut impl Write, href: &str) -> fmt::Result {
        out.write_str("<a href=\"")?;
        Self::escape(out, href)?;
        out.write_str("\">")
    }

    fn write_table_rows(out: &mut impl Write, mut rows: Vec<Vec<TableEl>>, aligments: &[Aligment], tag: &str, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        let cols = aligments.len();
        let n = rows.len() * cols;

        fn find(parent: &mut [usize], mut i: usize) -> usize {
            while parent[i] != i {
                parent[i] = parent[parent[i]];
                i = parent[i];
            }
            i
        }
        let mut parent: Vec<usize> = (0..n).collect();
        for (r, row) in rows.iter().enumerate() {
            for (c, el) in row.iter().enumerate() {
                let (up, left) = match el {
                    TableEl::MergeUp => (true, false),
                    TableEl::MergeLeft => (false, true),
                    TableEl::MergeUpLeft => (true, true),
                    TableEl::Cell(_) => (false, false),
                };
                let i = r * cols + c;
                if up {
                    let (a, b) = (find(&mut parent, i), find(&mut parent, i - cols));
                    parent[a] = b;
                }
                if left {
                    let (a, b) = (find(&mut parent, i), find(&mut parent, i - 1));
                    parent[a] = b;
                }
            }
        }
        let region: Vec<usize> = (0..n).map(|i| find(&mut parent, i)).collect();

        let mut taken = vec![false; n];
        let joinable = |taken: &[bool], rows: &[Vec<TableEl>], r: usize, c: usize, anchor: usize| {
            let i = r * cols + c;
            !taken[i] && region[i] == region[anchor] && !matches!(rows[r][c], TableEl::Cell(_))
        };

        for r in 0..rows.len() {
            out.write_str("<tr>")?;
            for c in 0..cols {
                let i = r * cols + c;
                if taken[i] {
                    continue;
                }
                let align = match aligments[c] {
                    Aligment::Left => "left",
                    Aligment::Center => "center",
                    Aligment::Right => "right",
                };
                if let TableEl::Cell(content) = &mut rows[r][c] {
                    let content = std::mem::take(content);
                    let mut w = 1;
                    while c + w < cols && joinable(&taken, &rows, r, c + w, i) {
                        w += 1;
                    }
                    let mut h = 1;
                    while r + h < rows.len() && (0..w).all(|k| joinable(&taken, &rows, r + h, c + k, i)) {
                        h += 1;
                    }
                    for dr in 0..h {
                        for dc in 0..w {
                            taken[(r + dr) * cols + c + dc] = true;
                        }
                    }

                    write!(out, "<{tag} class=\"{align}\"")?;
                    if w > 1 {
                        write!(out, " colspan=\"{w}\"")?;
                    }
                    if h > 1 {
                        write!(out, " rowspan=\"{h}\"")?;
                    }
                    out.write_str(">")?;
                    Self::span(out, content, res, file, warn)?;
                    write!(out, "</{tag}>")?;
                } else {
                    taken[i] = true;
                    write!(out, "<{tag} class=\"{align}")?;
                    if r > 0 && region[i - cols] == region[i] {
                        out.write_str(" join-top")?;
                    }
                    if c > 0 && region[i - 1] == region[i] {
                        out.write_str(" join-left")?;
                    }
                    write!(out, "\"></{tag}>")?;
                }
            }
            out.write_str("</tr>")?;
        }
        Ok(())
    }

    fn write_err(out: &mut impl Write) -> fmt::Result {
        out.write_str("<span class=\"error\">??</span>")
    }

    fn wrtie_list(out: &mut impl Write, items: Vec<ListEl>, level: u16, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        let mut num: u32 = 0;
        for el in items {
            match el.this {
                Some(ListElKind::Ordered(spans)) => {
                    num += 1;
                    write!(out, "<div class=\"item num\" style=\"--level: {0}\">{1})&nbsp;", level, num)?;
                    Self::span(out, spans, res, file, warn)?;
                    out.write_str("</div>")?;
                },
                Some(ListElKind::Unordered(spans)) => {
                    num = 0;
                    write!(out, "<div class=\"item dash\" style=\"--level: {}\">–&nbsp;", level)?;
                    Self::span(out, spans, res, file, warn)?;
                    out.write_str("</div>")?;
                },
                None => ()
            }
            Self::wrtie_list(out, el.children, level + 1, res, file, warn)?;
        }
        Ok(())
    }
}

impl Backend for HtmlBackend {
    fn span(out: &mut impl Write, inp: Vec<Span>, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        for span in inp{
            let style = &span.style;
            let tags = [
                (style.is_bold, "<b>", "</b>"),
                (style.is_italic, "<i>", "</i>"),
                (style.is_mono, "<span class=\"mono\">", "</span>"),
                (style.is_strike, "<s>", "</s>"),
                (style.is_under, "<u>", "</u>")
            ];

            for &(on, open, _) in &tags {
                if on {out.write_str(open)?}
            }
            if let Some(col) = &style.color {
                write!(out, "<span class=\"color\" style=\"color:{}\">", col.to_hex_tag())?;
            }
            let mut link_open = false;
            if let Some(link) = &style.link{
                match *link {
                    OuterLink::URL(link) => match Self::parse_url(link) {
                        Some(url) => {
                            Self::open_link(out, url.as_str())?;
                            link_open = true;
                        }
                        None => warn.push(RenderWarning::InvalidUrl(link.to_string())),
                    }
                    OuterLink::File(id) => {
                        let href = Path::new(&res.href(file, id)).with_extension("html");
                        Self::open_link(out, &href.to_string_lossy())?;
                        link_open = true;
                    }
                }
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
                            warn.push(RenderWarning::UnresolvedRef);
                            Self::write_err(out)?;
                        }
                    },
                    InlineEl::Note(note) => {
                        if let Some(i) = note {
                            write!(out, "<a class=\"ref-note\" href=\"#n-{0}\">{0}</a> ", i)?;
                        } else {
                            warn.push(RenderWarning::UnresolvedNote);
                            Self::write_err(out)?;
                        }
                    }
                }
            }

            if link_open {out.write_str("</a>")?}
            if style.color.is_some() {out.write_str("</span>")?}
            for &(on, _, close) in tags.iter().rev() {
                if on {
                    out.write_str(close)?;
                }
            }
        }

        Ok(())
    }


    fn paragraph(out: &mut impl Write, paragraph: Paragraph, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> std::fmt::Result {
        out.write_str("<p>")?;
        Self::span(out, paragraph.content, res, file, warn)?;
        out.write_str("</p>")?;
        Ok(())
    }

    fn code(out: &mut impl Write, code: Code, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        match code.named {
            Named::WithName { name, index } => {
                write!(out, "<div class=\"code\" id=\"c-{0}\"><div class=\"index\">Листинг&nbsp;{0}&nbsp;—&nbsp;</div><div class=\"name\">", index)?;
                Self::span(out, name, res, file, warn)?;
                out.write_str("</div><div class=\"body\"><pre><code>")?;
            }
            Named::Anonymous{index} => {
                write!(out, "<div class=\"code\" id=\"c-{0}\"><div class=\"index\">Листинг&nbsp;{0}</div><div class=\"body\"><pre><code>", index)?;
            }
        }
        for line in code.content {
            out.write_str(line)?;
            out.write_str("\n")?;
        }
        out.write_str("</code></pre></div></div>")?;
        Ok(())
    }
    
    fn comment(out: &mut impl Write, comment: Comment, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        out.write_str("<p class=\"comment\">")?;
        Self::span(out, comment.content, res, file, warn)?;
        out.write_str("</p>")?;
        Ok(())
    }

    fn formula(out: &mut impl Write, formula: Formula, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        match formula.named {
            Named::WithName { name, index } => {
                write!(out, "<div class=\"formula\" id=\"f-{0}\"><div class=\"index\">({0})</div><div class=\"name\">", index)?;
                Self::span(out, name, res, file, warn)?;
                out.write_str("</div><div class=\"body\">")?;
            }
            Named::Anonymous { index } => {
                write!(out, "<div class=\"formula\" id=\"f-{0}\"><div class=\"index\">({0})</div><div class=\"body\">", index)?;
            }
        }
        match latex_to_mathml(&formula.content.join("\n"), DisplayStyle::Block) {
            Ok(mathml) => out.write_str(&mathml)?,
            Err(e) => {
                warn.push(RenderWarning::InvalidFormula(e.to_string()));
                Self::write_err(out)?;
            }
        }
        out.write_str("</div></div>")?;
        Ok(())
    }

    fn header(out: &mut impl Write, header: Header, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
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
        Self::span(out, header.content, res, file, warn)?;
        write!(out, "</span></h{level}>")?;
        Ok(())
    }

    fn image(out: &mut impl Write, image: Image, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        let name = image.meta.named;
        let scale = image.meta.scale;
        match name {
            Named::WithName { name, index } => {
                write!(out, "<div class=\"image\" id=\"i-{0}\"><div class=\"index\">Рисукнок&nbsp;{0}&nbsp;—</div><div class=\"name\">", index)?;
                Self::span(out, name, res, file, warn)?;
                out.write_str("</div><div class=\"body\">")?;
            }
            Named::Anonymous { index } => {
                write!(out, "<div class=\"image\" id=\"i-{0}\"><div class=\"index\">Рисукнок&nbsp;{0}</div><div class=\"body\">", index)?;
            }
        }
        if let Some(id) = image.path {
            out.write_str("<img src=\"")?;
            let href = &res.href(file, id);
            Self::escape(out, href)?;
            write!(out, "\" style=\"zoom: {scale}%\">")?;
        } else {
            warn.push(UnresolvedImage);
        }
        
        out.write_str("</div></div>")?;
        Ok(())
        
    }

    fn list(out: &mut impl Write, list: List, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        out.write_str("<div class=\"list\">")?;
        Self::wrtie_list(out, list.content, 0, res, file, warn)?;
        out.write_str("</div>")?;
        Ok(())
    }

    fn note(out: &mut impl Write, note: Note, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        out.write_str("<div class=\"note\">")?;
        for note in note.content {
            write!(out, "<div class=\"item\" id=\"n-{0}\"><div class=\"index\">{0}) </div><div class=\"body\">", note.index)?;
            Self::span(out, note.content, res, file, warn)?;
            out.write_str("</div></div>")?;
        }
        out.write_str("</div>")?;
        Ok(())
    }

    fn table(out: &mut impl Write, table: Table, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        match table.named {
            Named::WithName { name, index } => {
                write!(out, "<div class=\"table\" id=\"t-{0}\"><div class=\"index\">Таблица&nbsp;{0}&nbsp;—&nbsp;</div><div class=\"name\">", index)?;
                Self::span(out, name, res, file, warn)?;
                out.write_str("</div>")?;
            }
            Named::Anonymous { index } => {
                write!(out, "<div class=\"table\" id=\"t-{0}\"><div class=\"index\">Таблица&nbsp;{0}</div>", index)?;
            }
        }
        out.write_str("<table>")?;
        if !table.headers.is_empty() {
            out.write_str("<thead class=\"header\">")?;
            Self::write_table_rows(out, table.headers, &table.aligments, "th", res, file, warn)?;
            out.write_str("</thead>")?;
        }
        out.write_str("<tbody class=\"body\">")?;
        Self::write_table_rows(out, table.cells, &table.aligments, "td", res, file, warn)?;
        out.write_str("</tbody></table></div>")?;
        Ok(())
    }

    fn render(rush: Rush, res: &Resources, file: Id<FileId>, css_href: &str) -> Result<(String, Vec<RenderWarning>), fmt::Error> {
        let mut out = "<!DOCTYPE html> <html lang=\"ru\"><head><meta charset=\"UTF-8\"><link rel=\"stylesheet\" href=\"".to_string();
        Self::escape(&mut out, css_href)?;
        out.write_str("\"></head><body><main>")?;
        let mut warn = Vec::new();
        for node in rush.nodes {
            Self::block(&mut out, node, res, file, &mut warn)?;
        }
        out.write_str("</main></body></html>")?;
        Ok((out, warn))
    }
}

