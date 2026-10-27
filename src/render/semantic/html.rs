use super::PageCtx;
use crate::config::DocOut;
use crate::render::ir::{self, Block, Caption, Cell, Link, ParaKind, Run, RunText};
use crate::render::warning::Feature;
use crate::render::{css_fonts, escape, Semantic};
use crate::resources::{FileId, Id, Resources};
use crate::style::{Align, Role, Style};
use crate::OutFile;
use std::fmt::{self, Write};
use std::path::Path;

const BASE_CSS: &str = include_str!("../../templates/html/base.css");
pub(crate) const CSS_NAME: &str = "style.css";

pub(crate) struct Html;

fn align_name(a: Align) -> &'static str {
    match a {
        Align::Justify => "justify",
        Align::Left => "left",
        Align::Center => "center",
        Align::Right => "right",
    }
}

fn role_rule(out: &mut String, selector: &str, role: &Role, indent: Option<String>) {
    let family = if role.font.mono { "var(--mono)" } else { "var(--body)" };
    let first = indent.unwrap_or_else(|| format!("{}pt", role.first - role.rest));
    let _ = write!(out, "{selector} {{ font-family: {family}; font-size: {}pt; line-height: {}pt; font-weight: {}; font-style: {}; \
        text-transform: {}; text-align: {}; text-indent: {first}; margin: {}pt 0 {}pt;",
        role.size, role.lh,
        if role.font.bold { "bold" } else { "normal" },
        if role.font.italic { "italic" } else { "normal" },
        if role.upper { "uppercase" } else { "none" },
        align_name(role.align),
        role.space_before, role.space_after);
    if role.rest > 0.0 {
        let _ = write!(out, " padding-left: {}pt;", role.rest);
    }
    if role.align == Align::Justify {
        out.push_str(" hyphens: auto;");
    }
    if role.keep_with_next {
        out.push_str(" break-after: avoid;");
    }
    if role.break_before {
        out.push_str(" break-before: page;");
    }
    out.push_str(" }\n");
}

fn css(style: &Style) -> String {
    let mut out = css_fonts(style, "fonts");
    let f = &style.fonts;
    let _ = write!(out, ":root {{ --body: \"{}\", {}; --mono: \"{}\", {}; --size: {}pt; --lh: {}pt; --small: {}pt; \
        --border: {}pt; --cell-pad: {}pt {}pt; --code-pad: {}pt {}pt; --note-rule: {}pt; --image-gap: {}pt; }}\n",
        f.body.name, f.body.fallback, f.mono.name, f.mono.fallback,
        style.body.size, style.body.lh, style.cell.size,
        style.border, style.cell_frame.pad_y, style.cell_frame.pad_x, style.code_frame.pad_y, style.code_frame.pad_x,
        style.notes.rule_w, style.images.gap);
    out.push_str(BASE_CSS);
    let list = &style.list;
    let rules: [(&str, &Role, Option<String>); 16] = [
        ("main p", &style.body, None),
        ("main p.comment", &style.comment, None),
        ("main h1", &style.headers[0], None),
        ("main h2", &style.headers[1], None),
        ("main h3", &style.headers[2], None),
        ("main h4", &style.headers[3], None),
        ("main h5", &style.headers[4], None),
        ("main .list > .item", list, Some(format!("calc({}pt * (var(--level, 0) + 1))", list.first - list.rest))),
        ("main .figure", &style.figure, None),
        ("main .formula", &style.formula, None),
        ("main .code", &style.code, None),
        ("main .table", &style.table, None),
        ("main .table td", &style.cell, None),
        ("main .table th", &style.head_cell, None),
        ("main .notes > .note", &style.notes.role, None),
        ("main .formula > .name", &style.body, None),
    ];
    for (sel, role, indent) in rules {
        role_rule(&mut out, sel, role, indent);
    }
    for (sel, cap) in [("main .table > .cap", &style.captions.table), ("main .figure > .cap", &style.captions.image), ("main .code > .cap", &style.captions.code)] {
        role_rule(&mut out, sel, &cap.role, None);
    }
    out
}

fn file_href(res: &Resources, from: Id<FileId>, to: Id<FileId>) -> String {
    Path::new(&res.href(from, to)).with_extension("html").to_string_lossy().into_owned()
}

struct Writer<'a> {
    ctx: &'a PageCtx<'a>,
    file: Id<FileId>,
}

impl Writer<'_> {
    fn runs(&self, out: &mut String, runs: &[Run]) -> fmt::Result {
        for run in runs {
            let mut close: Vec<&str> = Vec::new();
            if let Some(link) = &run.link {
                let href = match link {
                    Link::Url(u) => u.clone(),
                    Link::Doc(id) => file_href(self.ctx.res, self.file, *id),
                    Link::Anchor(a) => format!("#{}", a.id()),
                };
                out.write_str("<a href=\"")?;
                escape(out, &href)?;
                out.write_str("\">")?;
                close.push("</a>");
            }
            let tags = [
                (run.font.bold, "<b>", "</b>"),
                (run.font.italic, "<i>", "</i>"),
                (run.font.mono, "<span class=\"mono\">", "</span>"),
                (run.strike, "<s>", "</s>"),
                (run.under, "<u>", "</u>"),
            ];
            for (on, open, c) in tags {
                if on {
                    out.write_str(open)?;
                    close.push(c);
                }
            }
            if let Some(col) = &run.color {
                write!(out, "<span style=\"color:{}\">", col.to_hex_tag())?;
                close.push("</span>");
            }
            match &run.text {
                RunText::Text(s) => escape(out, s)?,
                RunText::Space => out.write_char(' ')?,
                RunText::Ref(i) => write!(out, "{i}")?,
                RunText::Note(g) => write!(out, "<sup class=\"ref-note\">{g}</sup>")?,
                RunText::Error => out.write_str("<span class=\"error\">??</span>")?,
            }
            for c in close.iter().rev() {
                out.write_str(c)?;
            }
        }
        Ok(())
    }

    fn caption(&self, out: &mut String, caption: &Caption) -> fmt::Result {
        out.write_str("<div class=\"cap\"><span class=\"index\">")?;
        escape(out, &caption.label)?;
        out.write_str("</span>")?;
        if let Some(name) = &caption.name {
            out.write_str("<span class=\"name\">")?;
            self.runs(out, name)?;
            out.write_str("</span>")?;
        }
        out.write_str("</div>")
    }

    fn cells(&self, out: &mut String, rows: &[Vec<Cell>], aligns: &[Align], tag: &str) -> fmt::Result {
        for row in rows {
            out.write_str("<tr>")?;
            for cell in row {
                match cell {
                    Cell::Text { col, w, h, runs } => {
                        write!(out, "<{tag} class=\"{}\"", align_name(aligns[*col]))?;
                        if *w > 1 {
                            write!(out, " colspan=\"{w}\"")?;
                        }
                        if *h > 1 {
                            write!(out, " rowspan=\"{h}\"")?;
                        }
                        out.write_str(">")?;
                        self.runs(out, runs)?;
                        write!(out, "</{tag}>")?;
                    }
                    Cell::Empty { col, join_top, join_left } => {
                        write!(out, "<{tag} class=\"{}", align_name(aligns[*col]))?;
                        if *join_top {
                            out.write_str(" join-top")?;
                        }
                        if *join_left {
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

    fn block(&self, out: &mut String, block: &Block) -> fmt::Result {
        let style = self.ctx.style;
        match block {
            Block::Para { kind, runs } => {
                out.write_str(match kind {
                    ParaKind::Body => "<p>",
                    ParaKind::Comment => "<p class=\"comment\">",
                })?;
                self.runs(out, runs)?;
                out.write_str("</p>")
            }
            Block::Header { level, number, runs, anchor } => {
                write!(out, "<h{level} id=\"{}\">", anchor.id())?;
                if let Some(n) = number {
                    out.write_str("<span class=\"index\">")?;
                    escape(out, n)?;
                    out.write_str("</span> ")?;
                }
                self.runs(out, runs)?;
                write!(out, "</h{level}>")
            }
            Block::List(items) => {
                out.write_str("<div class=\"list\">")?;
                for item in items {
                    write!(out, "<div class=\"item\" style=\"--level: {}\">", item.level)?;
                    escape(out, &item.marker)?;
                    self.runs(out, &item.runs)?;
                    out.write_str("</div>")?;
                }
                out.write_str("</div>")
            }
            Block::Figure { caption, image, scale, anchor } => {
                write!(out, "<div class=\"figure\" id=\"{}\"><div class=\"body\">", anchor.id())?;
                match image {
                    Some(id) => {
                        out.write_str("<img alt=\"\" src=\"")?;
                        escape(out, &self.ctx.res.href(self.file, *id))?;
                        write!(out, "\" style=\"zoom: {scale}%\">")?;
                    }
                    None => out.write_str("<span class=\"error\">??</span>")?,
                }
                out.write_str("</div>")?;
                self.caption(out, caption)?;
                out.write_str("</div>")
            }
            Block::Formula { name, mathml, number, anchor, .. } => {
                write!(out, "<div class=\"formula\" id=\"{}\">", anchor.id())?;
                if let Some(name) = name {
                    out.write_str("<div class=\"name\">")?;
                    self.runs(out, name)?;
                    out.write_str("</div>")?;
                }
                out.write_str("<div class=\"body\">")?;
                match mathml {
                    Some(m) => out.write_str(m)?,
                    None => out.write_str("<span class=\"error\">??</span>")?,
                }
                out.write_str("</div><div class=\"index\">")?;
                escape(out, number)?;
                out.write_str("</div></div>")
            }
            Block::Code { caption, lines, anchor } => {
                write!(out, "<div class=\"code\" id=\"{}\">", anchor.id())?;
                self.caption(out, caption)?;
                out.write_str("<pre><code>")?;
                for line in lines {
                    escape(out, line)?;
                    out.write_char('\n')?;
                }
                out.write_str("</code></pre></div>")
            }
            Block::Table { caption, aligns, head, body, anchor, .. } => {
                write!(out, "<div class=\"table\" id=\"{}\">", anchor.id())?;
                self.caption(out, caption)?;
                out.write_str("<table>")?;
                if !head.is_empty() {
                    let head_aligns = vec![style.head_cell.align; aligns.len()];
                    out.write_str("<thead>")?;
                    self.cells(out, head, &head_aligns, "th")?;
                    out.write_str("</thead>")?;
                }
                out.write_str("<tbody>")?;
                self.cells(out, body, aligns, "td")?;
                out.write_str("</tbody></table></div>")
            }
            Block::Notes(defs) => {
                out.write_str("<div class=\"notes\">")?;
                for def in defs {
                    write!(out, "<div class=\"note\" id=\"{}\"><sup>{}</sup> ", ir::Anchor::Note(def.g).id(), def.g)?;
                    self.runs(out, &def.runs)?;
                    out.write_str("</div>")?;
                }
                out.write_str("</div>")
            }
        }
    }
}

impl Semantic for Html {
    fn out(&self) -> DocOut {
        DocOut::Html
    }

    fn supports(&self, _feature: Feature) -> bool {
        true
    }

    fn ext(&self) -> &'static str {
        "html"
    }

    fn assets(&self, style: &Style) -> Vec<OutFile> {
        vec![OutFile { path: CSS_NAME.to_string(), data: css(style).into_bytes() }]
    }

    fn page(&self, doc: &ir::Doc, ctx: &PageCtx) -> Result<String, fmt::Error> {
        let res = ctx.res;
        let file = doc.file;
        let w = Writer { ctx, file };
        let mut main = String::new();
        for block in &doc.blocks {
            w.block(&mut main, block)?;
        }

        let max_level = ctx.style.toc.map_or(1, |t| t.max_level as usize);
        let mut out = String::from("<!DOCTYPE html><html lang=\"ru\"><head><meta charset=\"UTF-8\">");
        out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>");
        escape(&mut out, &doc.title)?;
        out.write_str("</title><link rel=\"stylesheet\" href=\"")?;
        escape(&mut out, &res.root_href(file, CSS_NAME))?;
        out.write_str("\"></head><body><header class=\"title\">")?;
        escape(&mut out, &doc.title)?;
        out.write_str("</header>")?;
        if !ctx.content.items.is_empty() {
            out.write_str("<nav class=\"toc\">")?;
            for item in &ctx.content.items {
                let level = item.depth.clamp(1, max_level) - 1;
                out.write_str("<a class=\"item")?;
                if item.file == file {
                    out.write_str(" current\" aria-current=\"page")?;
                }
                write!(out, "\" style=\"--level: {level}\" href=\"")?;
                escape(&mut out, &file_href(res, file, item.file))?;
                out.write_str("\">")?;
                if level > 0 {
                    out.write_str("–&nbsp;")?;
                }
                escape(&mut out, &item.name)?;
                out.write_str("</a>")?;
            }
            out.write_str("</nav>")?;
        }
        out.write_str("<main>")?;
        out.write_str(&main)?;
        out.write_str("</main></body></html>")?;
        Ok(out)
    }
}
