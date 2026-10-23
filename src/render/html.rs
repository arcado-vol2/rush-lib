use crate::render::common::{Backend, RenderWarning};
use crate::parse::rush::{Code, Comment, Formula, Header, Image, InlineEl, InnerLink, List, ListEl, ListElKind, Node, Note, OuterLink, Paragraph, Rush, Span, Table, TableEl, Aligment};
use crate::parse::common::{Color, Named, RefKind};
use crate::resources::{FileId, Id, Resources};
use crate::content::Content;
use std::fmt::{self, Write};
use std::path::Path;
use url::{ParseError, Url};
use latex2mathml::{latex_to_mathml, DisplayStyle};

pub(crate) mod gostlike;
pub(crate) mod gost;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptionKind {
    Table,
    Image,
    Code,
    Formula,
}

pub(crate) struct NavItem<'a> {
    pub(crate) name: &'a str,
    pub(crate) href: String,
    pub(crate) depth: usize,
    pub(crate) current: bool,
}

pub(crate) struct Page<'a> {
    pub(crate) title: &'a str,
    pub(crate) css_href: String,
    pub(crate) nav: Vec<NavItem<'a>>,
}

pub(crate) trait HtmlStyle {
    fn css(&self) -> &'static str;
    fn css_name(&self) -> &'static str;
    fn caption(&self, out: &mut dyn Write, kind: CaptionKind, index: u16, named: bool) -> fmt::Result;
    fn inner_link(&self, out: &mut dyn Write, kind: RefKind, index: u16) -> fmt::Result;
    fn note_ref(&self, out: &mut dyn Write, index: u16) -> fmt::Result;
    fn head(&self, out: &mut dyn Write, page: &Page) -> fmt::Result;
    fn page(&self, out: &mut dyn Write, page: &Page, main: &str) -> fmt::Result;
}

pub(crate) fn escape<W: Write + ?Sized>(out: &mut W, s: &str) -> fmt::Result {
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

pub(crate) fn parse_url(link: &str) -> Option<Url> {
    match Url::parse(link) {
        Ok(url) => Some(url),
        Err(ParseError::RelativeUrlWithoutBase) => Url::parse(&format!("https://{link}")).ok(),
        Err(_) => None,
    }
}

pub(crate) fn align_class(a: Aligment) -> &'static str {
    match a {
        Aligment::Left => "left",
        Aligment::Center => "center",
        Aligment::Right => "right",
    }
}

pub(crate) enum Slot<'a> {
    Cell { col: usize, w: usize, h: usize, content: Vec<Span<'a>> },
    Empty { col: usize, join_top: bool, join_left: bool },
}

pub(crate) fn merge_cells(mut rows: Vec<Vec<TableEl>>, cols: usize) -> Vec<Vec<Slot>> {
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

    let mut out = Vec::with_capacity(rows.len());
    for r in 0..rows.len() {
        let mut row = Vec::new();
        for c in 0..cols {
            let i = r * cols + c;
            if taken[i] {
                continue;
            }
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
                row.push(Slot::Cell { col: c, w, h, content });
            } else {
                taken[i] = true;
                row.push(Slot::Empty {
                    col: c,
                    join_top: r > 0 && region[i - cols] == region[i],
                    join_left: c > 0 && region[i - 1] == region[i],
                });
            }
        }
        out.push(row);
    }
    out
}

pub(crate) struct HtmlBackend<'a> {
    pub(crate) style: &'a dyn HtmlStyle,
}

impl HtmlBackend<'_> {
    fn open_link(out: &mut impl Write, href: &str) -> fmt::Result {
        out.write_str("<a href=\"")?;
        escape(out, href)?;
        out.write_str("\">")
    }

    fn file_href(res: &Resources, from: Id<FileId>, to: Id<FileId>) -> String {
        Path::new(&res.href(from, to)).with_extension("html").to_string_lossy().into_owned()
    }

    fn write_caption(&self, out: &mut impl Write, kind: CaptionKind, index: u16, named: bool) -> fmt::Result {
        out.write_str("<div class=\"index\">")?;
        self.style.caption(out, kind, index, named)?;
        out.write_str("</div>")
    }

    fn write_table_rows(&self, out: &mut impl Write, rows: Vec<Vec<TableEl>>, aligments: &[Aligment], tag: &str, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        for row in merge_cells(rows, aligments.len()) {
            out.write_str("<tr>")?;
            for slot in row {
                match slot {
                    Slot::Cell { col, w, h, content } => {
                        write!(out, "<{tag} class=\"{}\"", align_class(aligments[col]))?;
                        if w > 1 {
                            write!(out, " colspan=\"{w}\"")?;
                        }
                        if h > 1 {
                            write!(out, " rowspan=\"{h}\"")?;
                        }
                        out.write_str(">")?;
                        self.span(out, content, res, file, warn)?;
                        write!(out, "</{tag}>")?;
                    }
                    Slot::Empty { col, join_top, join_left } => {
                        write!(out, "<{tag} class=\"{}", align_class(aligments[col]))?;
                        if join_top {
                            out.write_str(" join-top")?;
                        }
                        if join_left {
                            out.write_str(" join-left")?;
                        }
                        write!(out, "\"></{tag}>")?;
                    }
                }
            }
            out.write_str("</tr>")?;
        }
        Ok(())
    }

    fn write_err(out: &mut impl Write) -> fmt::Result {
        out.write_str("<span class=\"error\">??</span>")
    }

    fn wrtie_list(&self, out: &mut impl Write, items: Vec<ListEl>, level: u16, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        let mut num: u32 = 0;
        for el in items {
            match el.this {
                Some(ListElKind::Ordered(spans)) => {
                    num += 1;
                    write!(out, "<div class=\"item num\" style=\"--level: {0}\">{1})&nbsp;", level, num)?;
                    self.span(out, spans, res, file, warn)?;
                    out.write_str("</div>")?;
                },
                Some(ListElKind::Unordered(spans)) => {
                    num = 0;
                    write!(out, "<div class=\"item dash\" style=\"--level: {}\">–&nbsp;", level)?;
                    self.span(out, spans, res, file, warn)?;
                    out.write_str("</div>")?;
                },
                None => ()
            }
            self.wrtie_list(out, el.children, level + 1, res, file, warn)?;
        }
        Ok(())
    }
}

impl Backend for HtmlBackend<'_> {
    fn span(&self, out: &mut impl Write, inp: Vec<Span>, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
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
                    OuterLink::URL(link) => match parse_url(link) {
                        Some(url) => {
                            Self::open_link(out, url.as_str())?;
                            link_open = true;
                        }
                        None => warn.push(RenderWarning::InvalidUrl(link.to_string())),
                    }
                    OuterLink::File(id) => {
                        Self::open_link(out, &Self::file_href(res, file, id))?;
                        link_open = true;
                    }
                }
            }

            for el in span.text {
                match el {
                    InlineEl::String(s) => escape(out, s)?,
                    InlineEl::Space => out.write_char(' ')?,
                    InlineEl::LongDash => out.write_char('—')?,
                    InlineEl::InnerLink(link ) => match link {
                        Ok(link) => {
                            let prefix = match link.kind {
                                RefKind::Table => 't',
                                RefKind::Code => 'c',
                                RefKind::Formula => 'f',
                                RefKind::Image => 'i',
                            };
                            write!(out, "<a class=\"ref-table\" href=\"#{prefix}-{}\">", link.index)?;
                            self.style.inner_link(out, link.kind, link.index)?;
                            out.write_str("</a>")?;
                        }
                        Err((name, kind)) => {
                            warn.push(RenderWarning::UnresolvedRef { kind, name: name.into_owned() });
                            Self::write_err(out)?;
                        }
                    },
                    InlineEl::Note(note) => match note {
                        Ok(i) => {
                            write!(out, "<a class=\"ref-note\" href=\"#n-{i}\">")?;
                            self.style.note_ref(out, i)?;
                            out.write_str("</a> ")?;
                        }
                        Err(i) => {
                            warn.push(RenderWarning::UnresolvedNote(i));
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


    fn paragraph(&self, out: &mut impl Write, paragraph: Paragraph, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> std::fmt::Result {
        out.write_str("<p>")?;
        self.span(out, paragraph.content, res, file, warn)?;
        out.write_str("</p>")?;
        Ok(())
    }

    fn code(&self, out: &mut impl Write, code: Code, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        match code.named {
            Named::WithName { name, index } => {
                write!(out, "<div class=\"code\" id=\"c-{index}\">")?;
                self.write_caption(out, CaptionKind::Code, index, true)?;
                out.write_str("<div class=\"name\">")?;
                self.span(out, name, res, file, warn)?;
                out.write_str("</div><div class=\"body\"><pre><code>")?;
            }
            Named::Anonymous{index} => {
                write!(out, "<div class=\"code\" id=\"c-{index}\">")?;
                self.write_caption(out, CaptionKind::Code, index, false)?;
                out.write_str("<div class=\"body\"><pre><code>")?;
            }
        }
        for line in code.content {
            out.write_str(line)?;
            out.write_str("\n")?;
        }
        out.write_str("</code></pre></div></div>")?;
        Ok(())
    }
    
    fn comment(&self, out: &mut impl Write, comment: Comment, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        out.write_str("<p class=\"comment\">")?;
        self.span(out, comment.content, res, file, warn)?;
        out.write_str("</p>")?;
        Ok(())
    }

    // TODO: write an own formula parser with its own markup, based on the one used in LibreOffice
    fn formula(&self, out: &mut impl Write, formula: Formula, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        match formula.named {
            Named::WithName { name, index } => {
                write!(out, "<div class=\"formula\" id=\"f-{index}\">")?;
                self.write_caption(out, CaptionKind::Formula, index, true)?;
                out.write_str("<div class=\"name\">")?;
                self.span(out, name, res, file, warn)?;
                out.write_str("</div><div class=\"body\">")?;
            }
            Named::Anonymous { index } => {
                write!(out, "<div class=\"formula\" id=\"f-{index}\">")?;
                self.write_caption(out, CaptionKind::Formula, index, false)?;
                out.write_str("<div class=\"body\">")?;
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

    fn header(&self, out: &mut impl Write, header: Header, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
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
        self.span(out, header.content, res, file, warn)?;
        write!(out, "</span></h{level}>")?;
        Ok(())
    }

    fn image(&self, out: &mut impl Write, image: Image, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        let name = image.meta.named;
        let scale = image.meta.scale;
        match name {
            Named::WithName { name, index } => {
                write!(out, "<div class=\"image\" id=\"i-{index}\">")?;
                self.write_caption(out, CaptionKind::Image, index, true)?;
                out.write_str("<div class=\"name\">")?;
                self.span(out, name, res, file, warn)?;
                out.write_str("</div><div class=\"body\">")?;
            }
            Named::Anonymous { index } => {
                write!(out, "<div class=\"image\" id=\"i-{index}\">")?;
                self.write_caption(out, CaptionKind::Image, index, false)?;
                out.write_str("<div class=\"body\">")?;
            }
        }
        match image.path {
            Ok(id) => {
                out.write_str("<img src=\"")?;
                let href = &res.href(file, id);
                escape(out, href)?;
                write!(out, "\" style=\"zoom: {scale}%\">")?;
            }
            Err(path) => warn.push(RenderWarning::UnresolvedImage(path.into_owned())),
        }
        
        out.write_str("</div></div>")?;
        Ok(())
        
    }

    fn list(&self, out: &mut impl Write, list: List, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        out.write_str("<div class=\"list\">")?;
        self.wrtie_list(out, list.content, 0, res, file, warn)?;
        out.write_str("</div>")?;
        Ok(())
    }

    fn note(&self, out: &mut impl Write, note: Note, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        out.write_str("<div class=\"note\">")?;
        for note in note.content {
            write!(out, "<div class=\"item\" id=\"n-{0}\"><div class=\"index\">{0}) </div><div class=\"body\">", note.index)?;
            self.span(out, note.content, res, file, warn)?;
            out.write_str("</div></div>")?;
        }
        out.write_str("</div>")?;
        Ok(())
    }

    fn table(&self, out: &mut impl Write, table: Table, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        match table.named {
            Named::WithName { name, index } => {
                write!(out, "<div class=\"table\" id=\"t-{index}\">")?;
                self.write_caption(out, CaptionKind::Table, index, true)?;
                out.write_str("<div class=\"name\">")?;
                self.span(out, name, res, file, warn)?;
                out.write_str("</div>")?;
            }
            Named::Anonymous { index } => {
                write!(out, "<div class=\"table\" id=\"t-{index}\">")?;
                self.write_caption(out, CaptionKind::Table, index, false)?;
            }
        }
        out.write_str("<table>")?;
        if !table.headers.is_empty() {
            out.write_str("<thead class=\"header\">")?;
            self.write_table_rows(out, table.headers, &table.aligments, "th", res, file, warn)?;
            out.write_str("</thead>")?;
        }
        out.write_str("<tbody class=\"body\">")?;
        self.write_table_rows(out, table.cells, &table.aligments, "td", res, file, warn)?;
        out.write_str("</tbody></table></div>")?;
        Ok(())
    }

    fn render(&self, rush: Rush, res: &Resources, file: Id<FileId>, content: &Content) -> Result<(String, Vec<RenderWarning>), fmt::Error> {
        let mut warn = Vec::new();
        let mut main = String::new();
        for node in rush.nodes {
            self.block(&mut main, node, res, file, &mut warn)?;
        }

        let stem = res.files.local_path(file).file_stem().unwrap_or_default().to_string_lossy();
        let page = Page {
            title: content.title_for(file).unwrap_or(&stem),
            css_href: res.root_href(file, self.style.css_name()),
            nav: content.items.iter().map(|item| NavItem {
                name: &item.name,
                href: Self::file_href(res, file, item.file),
                depth: item.depth,
                current: item.file == file,
            }).collect(),
        };

        let mut out = "<!DOCTYPE html> <html lang=\"ru\"><head><meta charset=\"UTF-8\">".to_string();
        self.style.head(&mut out, &page)?;
        out.write_str("</head><body>")?;
        self.style.page(&mut out, &page, &main)?;
        out.write_str("</body></html>")?;
        Ok((out, warn))
    }
}

