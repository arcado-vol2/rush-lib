use super::PageCtx;
use crate::config::DocOut;
use crate::render::ir::{self, Block, Caption, Cell, Link, ParaKind, Run, RunText};
use crate::render::warning::Feature;
use crate::render::Semantic;
use crate::resources::{FileId, Id};
use crate::style::{Align, Style};
use crate::OutFile;
use std::cmp::Reverse;
use std::fmt::{self, Write};
use std::path::Path;

pub(crate) struct Markdown;

const STRIKE: u8 = 1;
const BOLD: u8 = 2;
const ITALIC: u8 = 4;
const MARKS: [(u8, &str); 3] = [(STRIKE, "~~"), (BOLD, "**"), (ITALIC, "*")];

fn delim(mark: u8) -> &'static str {
    MARKS.iter().find(|(m, _)| *m == mark).map_or("", |(_, d)| d)
}

fn longest_run(s: &str, c: char) -> usize {
    s.split(|x| x != c).map(str::len).max().unwrap_or(0)
}

fn escape_char(out: &mut String, c: char) {
    if matches!(c, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '~' | '|' | '&') {
        out.push('\\');
    }
    out.push(c);
}

fn escape(out: &mut String, s: &str) {
    s.chars().for_each(|c| escape_char(out, c));
}

/// Link address; with spaces or brackets it goes in angle brackets
fn dest(out: &mut String, s: &str) {
    if !s.contains([' ', '(', ')', '<', '>']) {
        return out.push_str(s);
    }
    out.push('<');
    for c in s.chars() {
        if matches!(c, '<' | '>' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('>');
}

fn file_href(ctx: &PageCtx, from: Id<FileId>, to: Id<FileId>) -> String {
    Path::new(&ctx.res.href(from, to)).with_extension("md").to_string_lossy().into_owned()
}

/// Paragraph line after merging neighbouring runs with the same styling
enum Piece {
    Text { marks: u8, mono: bool, text: String },
    LinkStart,
    LinkEnd(String),
    Note(u16),
}

/// How many following pieces keep the mark: the longer, the further outside it is opened
fn persist(mark: u8, rest: &[Piece]) -> usize {
    rest.iter()
        .take_while(|p| match p {
            Piece::Text { marks, text, .. } => marks & mark != 0 || text.trim().is_empty(),
            _ => false,
        })
        .count()
}

/// Writes a line: marks as a stack, edge spaces are moved outside the delimiters,
/// otherwise `** a**` is not emphasis
struct Inline<'o> {
    out: &'o mut String,
    /// line continuation after a wrap; None - single-line block
    nl: Option<&'o str>,
    line_start: bool,
    open: Vec<u8>,
    ws: String,
    code: Option<String>,
    link_pending: bool,
    in_link: bool,
}

impl<'o> Inline<'o> {
    fn new(out: &'o mut String, nl: Option<&'o str>) -> Self {
        Self { out, nl, line_start: nl.is_some(), open: Vec::new(), ws: String::new(), code: None, link_pending: false, in_link: false }
    }

    fn write(&mut self, pieces: &[Piece]) {
        for (i, piece) in pieces.iter().enumerate() {
            match piece {
                Piece::Text { marks, mono, text } => self.text(*marks, *mono, text, &pieces[i + 1..]),
                Piece::LinkStart => {
                    self.close();
                    self.link_pending = true;
                }
                Piece::LinkEnd(href) => {
                    self.close();
                    if self.in_link {
                        self.out.push_str("](");
                        dest(self.out, href);
                        self.out.push(')');
                    }
                    self.link_pending = false;
                    self.in_link = false;
                }
                Piece::Note(g) => {
                    self.close();
                    self.flush_ws();
                    let _ = write!(self.out, "[^{g}]");
                    self.line_start = false;
                }
            }
        }
        self.close();
    }

    fn text(&mut self, marks: u8, mono: bool, text: &str, rest: &[Piece]) {
        let body = text.trim_start();
        self.ws.push_str(&text[..text.len() - body.len()]);
        let core = body.trim_end();
        if core.is_empty() {
            return;
        }
        let keep = self.open.iter().take_while(|&&m| marks & m != 0).count();
        let same = keep == self.open.len() && self.open.iter().fold(0, |a, m| a | m) == marks;
        if let (true, true, false, Some(code)) = (mono, same, self.link_pending, self.code.as_mut()) {
            code.push_str(&self.ws);
            code.push_str(core);
            self.ws.clear();
        } else {
            self.flush_code();
            while self.open.len() > keep {
                let m = self.open.pop().unwrap();
                self.out.push_str(delim(m));
            }
            self.flush_ws();
            if self.link_pending {
                self.out.push('[');
                self.link_pending = false;
                self.in_link = true;
                self.line_start = false;
            }
            let held = self.open.iter().fold(0, |a, m| a | m);
            let mut new: Vec<u8> = MARKS.iter().map(|(m, _)| *m).filter(|m| marks & !held & m != 0).collect();
            new.sort_by_key(|&m| Reverse(persist(m, rest)));
            for m in new {
                self.out.push_str(delim(m));
                self.open.push(m);
                self.line_start = false;
            }
            if mono {
                self.code = Some(core.to_string());
            } else {
                self.escaped(core);
            }
        }
        self.ws.push_str(&body[core.len()..]);
    }

    fn newline(&mut self) {
        match self.nl {
            Some(prefix) => {
                self.out.push('\n');
                self.out.push_str(prefix);
                self.line_start = true;
            }
            None if !self.line_start => self.out.push(' '),
            None => (),
        }
    }

    fn flush_ws(&mut self) {
        for c in std::mem::take(&mut self.ws).chars() {
            match c {
                '\n' => self.newline(),
                ' ' | '\t' | '\r' if self.line_start => (),
                '\r' => (),
                _ => {
                    self.out.push(c);
                    self.line_start = false;
                }
            }
        }
    }

    fn flush_code(&mut self) {
        let Some(code) = self.code.take() else { return };
        let mut code = code.replace('\n', " ");
        if self.nl.is_none() {
            code = code.replace('|', "\\|");
        }
        let fence = "`".repeat(longest_run(&code, '`') + 1);
        let pad = if code.starts_with('`') || code.ends_with('`') { " " } else { "" };
        let _ = write!(self.out, "{fence}{pad}{code}{pad}{fence}");
        self.line_start = false;
    }

    fn close(&mut self) {
        self.flush_code();
        while let Some(m) = self.open.pop() {
            self.out.push_str(delim(m));
        }
    }

    fn escaped(&mut self, text: &str) {
        for (i, mut line) in text.split('\n').enumerate() {
            if i > 0 {
                self.newline();
            }
            if self.line_start {
                line = line.trim_start_matches([' ', '\t']);
                if line.is_empty() {
                    continue;
                }
                let digits = line.find(|c: char| !c.is_ascii_digit()).unwrap_or(line.len());
                if let Some(p) = line[digits..].chars().next() {
                    let is_block = if digits == 0 {
                        matches!(p, '#' | '>' | '+' | '-' | '=')
                    } else {
                        digits <= 9 && matches!(p, '.' | ')')
                    };
                    if is_block {
                        self.out.push_str(&line[..digits]);
                        self.out.push('\\');
                        self.out.push(p);
                        line = &line[digits + 1..];
                    }
                }
                self.line_start = false;
            }
            for c in line.chars() {
                escape_char(self.out, c);
            }
        }
    }
}

struct Writer<'a> {
    ctx: &'a PageCtx<'a>,
    file: Id<FileId>,
    /// the whole project in one file at the output root
    joined: bool,
}

impl Writer<'_> {
    fn pieces(&self, runs: &[Run]) -> Vec<Piece> {
        let mut out = Vec::new();
        let mut link: Option<String> = None;
        for run in runs {
            let text = match &run.text {
                RunText::Note(g) => {
                    if let Some(href) = link.take() {
                        out.push(Piece::LinkEnd(href));
                    }
                    out.push(Piece::Note(*g));
                    continue;
                }
                RunText::Text(s) => s.clone(),
                RunText::Space => "\n".into(),
                RunText::Ref(i) => i.to_string(),
                RunText::Error => "??".into(),
            };
            let href = match &run.link {
                Some(Link::Url(u)) => Some(u.clone()),
                Some(Link::Doc(id)) if !self.joined => Some(file_href(self.ctx, self.file, *id)),
                Some(Link::Doc(_) | Link::Anchor(_)) | None => None,
            };
            if href != link {
                if let Some(old) = link.take() {
                    out.push(Piece::LinkEnd(old));
                }
                if href.is_some() {
                    out.push(Piece::LinkStart);
                }
                link = href;
            }
            let marks = [(run.strike, STRIKE), (run.font.bold, BOLD), (run.font.italic, ITALIC)]
                .into_iter()
                .filter_map(|(on, m)| on.then_some(m))
                .fold(0, |a, m| a | m);
            let mono = run.font.mono;
            match out.last_mut() {
                Some(Piece::Text { marks: m, mono: c, text: t }) if *m == marks && *c == mono => t.push_str(&text),
                _ => out.push(Piece::Text { marks, mono, text }),
            }
        }
        if let Some(href) = link {
            out.push(Piece::LinkEnd(href));
        }
        out
    }

    fn runs(&self, out: &mut String, runs: &[Run], nl: Option<&str>) {
        Inline::new(out, nl).write(&self.pieces(runs));
    }

    fn captioned(&self, out: &mut String, caption: &Caption, body: impl FnOnce(&mut String) -> fmt::Result) -> fmt::Result {
        let runs = caption.runs();
        if ir::plain(&runs).is_empty() {
            return body(out);
        }
        if self.ctx.style.captions.of(caption.kind).caption_after {
            body(out)?;
            out.push_str("\n\n");
            self.runs(out, &runs, Some(""));
        } else {
            self.runs(out, &runs, Some(""));
            out.push_str("\n\n");
            body(out)?;
        }
        Ok(())
    }

    fn row(&self, out: &mut String, cells: &[Cell], cols: usize) {
        let mut slots = vec![String::new(); cols];
        for cell in cells {
            if let Cell::Text { col, runs, .. } = cell {
                self.runs(&mut slots[*col], runs, None);
            }
        }
        out.push('|');
        for s in slots {
            out.push(' ');
            out.push_str(&s);
            out.push_str(" |");
        }
    }

    fn block(&self, out: &mut String, block: &Block) -> fmt::Result {
        match block {
            Block::Para { kind: ParaKind::Body, runs } => self.runs(out, runs, Some("")),
            Block::Para { kind: ParaKind::Comment, runs } => {
                out.push_str("> ");
                self.runs(out, runs, Some("> "));
            }
            Block::Header { level, number, runs, .. } => {
                out.push_str(&"#".repeat(*level as usize));
                out.push(' ');
                if let Some(n) = number {
                    escape(out, n);
                    out.push(' ');
                }
                self.runs(out, runs, None);
            }
            Block::List(items) => {
                let mut cols: Vec<usize> = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push('\n');
                    }
                    cols.truncate(item.level);
                    let indent = cols.last().copied().unwrap_or(0);
                    cols.resize(item.level, indent);
                    let digits = item.marker.find(|c: char| !c.is_ascii_digit()).unwrap_or(item.marker.len());
                    let marker = if digits == 0 { "-".to_string() } else { format!("{}.", &item.marker[..digits]) };
                    let col = indent + marker.len() + 1;
                    write!(out, "{}{marker} ", " ".repeat(indent))?;
                    self.runs(out, &item.runs, Some(&" ".repeat(col)));
                    cols.push(col);
                }
            }
            Block::Figure { caption, image, .. } => self.captioned(out, caption, |out| {
                match image {
                    Some(id) => {
                        out.push_str("![");
                        escape(out, &caption.name.as_deref().map(ir::plain).unwrap_or_default());
                        out.push_str("](");
                        let res = self.ctx.res;
                        dest(out, &if self.joined { res.root_relative(*id) } else { res.href(self.file, *id) });
                        out.push(')');
                    }
                    None => out.push_str("??"),
                }
                Ok(())
            })?,
            Block::Formula { name, mathml, src, number, .. } => {
                if let Some(name) = name {
                    self.runs(out, name, Some(""));
                    out.push_str("\n\n");
                }
                match mathml {
                    Some(_) => write!(out, "$$\n{src} \\tag*{{{number}}}\n$$")?,
                    None => out.push_str("??"),
                }
            }
            Block::Code { caption, lang, lines, .. } => self.captioned(out, caption, |out| {
                let run = lines.iter().map(|l| longest_run(l, '`')).max().unwrap_or(0);
                let fence = "`".repeat(run.max(2) + 1);
                let lang = lang.as_deref().filter(|_| self.ctx.style.highlight_code_syntax).unwrap_or("");
                writeln!(out, "{fence}{lang}")?;
                for line in lines {
                    writeln!(out, "{line}")?;
                }
                out.write_str(&fence)
            })?,
            Block::Table { caption, aligns, head, body, .. } => self.captioned(out, caption, |out| {
                let cols = aligns.len();
                let (first, rest) = head.split_first().map_or((&[][..], &[][..]), |(f, r)| (&f[..], r));
                self.row(out, first, cols);
                out.push_str("\n|");
                for a in aligns {
                    out.push_str(match a {
                        Align::Left => " :-- |",
                        Align::Center => " :-: |",
                        Align::Right => " --: |",
                        Align::Justify => " --- |",
                    });
                }
                for row in rest.iter().chain(body) {
                    out.push('\n');
                    self.row(out, row, cols);
                }
                Ok(())
            })?,
            Block::Notes(defs) => {
                for (i, def) in defs.iter().enumerate() {
                    if i > 0 {
                        out.push_str("\n\n");
                    }
                    if def.anchored {
                        write!(out, "[^{}]\n\n", def.g)?;
                    }
                    write!(out, "[^{}]: ", def.g)?;
                    self.runs(out, &def.runs, Some("    "));
                }
            }
        }
        Ok(())
    }
}

impl Semantic for Markdown {
    fn out(&self) -> DocOut {
        DocOut::Markdown
    }

    fn supports(&self, feature: Feature) -> bool {
        match feature {
            Feature::Strike | Feature::Formula | Feature::Notes => true,
            Feature::Paging | Feature::Color | Feature::Underline | Feature::MergedCells | Feature::ImageScale | Feature::ColumnWidths => false,
        }
    }

    fn ext(&self) -> &'static str {
        "md"
    }

    fn assets(&self, _style: &Style) -> Vec<OutFile> {
        Vec::new()
    }

    fn page(&self, doc: &ir::Doc, ctx: &PageCtx) -> Result<String, fmt::Error> {
        let mut out = String::new();
        write_doc(&mut out, doc, ctx, false)?;
        Ok(out)
    }

    fn joined(&self, project: &ir::Project, ctx: &PageCtx) -> Result<String, fmt::Error> {
        let mut out = String::new();
        for (i, doc) in project.docs.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            write_doc(&mut out, doc, ctx, true)?;
        }
        Ok(out)
    }
}

fn write_doc(out: &mut String, doc: &ir::Doc, ctx: &PageCtx, joined: bool) -> fmt::Result {
    let w = Writer { ctx, file: doc.file, joined };
    out.push_str("# ");
    escape(out, &doc.title);
    for block in &doc.blocks {
        out.push_str("\n\n");
        w.block(out, block)?;
    }
    out.push('\n');
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;
    use crate::render::ir::tests::project_of;
    use crate::style::Style;
    use crate::config::DocStyle;

    fn md(src: &str) -> String {
        md_with(src, DocStyle::Gost7_32)
    }

    fn md_with(src: &str, style: DocStyle) -> String {
        let (p, project, _) = project_of(src);
        let (content, _) = Content::resolve(&p.content, &p.dirs, &p.resources);
        let style = Style::of(style);
        let ctx = PageCtx { res: &p.resources, content: &content, style: &style, theme: crate::config::Theme::Light, scripts: true };
        let page = Markdown.page(&project.docs[0], &ctx).unwrap();
        page.strip_prefix("# Документ\n\n").unwrap().trim_end().to_string()
    }

    #[test]
    fn same_marks_merge() {
        assert_eq!(md("\\:aa:\\:bb:"), "**aabb**");
        assert_eq!(md("\\:a \\:b: c:"), "**a b c**");
    }

    #[test]
    fn nested_marks_stack() {
        assert_eq!(md("\\:a \\;b; c:"), "**a *b* c**");
        assert_eq!(md("\\;\\:a: b;"), "***a** b*");
    }

    #[test]
    fn spaces_outside_markers() {
        assert_eq!(md("x\\: a :y"), "x **a** y");
    }

    #[test]
    fn mono_and_line_start() {
        assert_eq!(md("\\!a b!"), "`a b`");
        assert_eq!(md("- не список"), "\\- не список");
    }

    #[test]
    fn notes_and_no_html() {
        let page = md("а\\*5 б\n\n\\**5 сноска\n\\**0 без ссылки");
        assert_eq!(page, "а[^1]б\n\n[^1]: сноска\n\n[^2]\n\n[^2]: без ссылки");
        assert!(!md("\\=a= \\%КРАС(b)").contains('<'));
    }

    #[test]
    fn code_lang_in_fence_only_with_highlight() {
        let src = "\\//py 0\nprint(1)\n\\//";
        let modern = md_with(src, DocStyle::Modern);
        assert!(modern.contains("```py\nprint(1)\n```"), "{modern}");
        let gost = md(src);
        assert!(gost.contains("```\nprint(1)\n```"), "{gost}");
    }

    #[test]
    fn column_widths_unsupported() {
        assert!(!Markdown.supports(Feature::ColumnWidths));
    }
}
