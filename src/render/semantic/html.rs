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

fn role_rule(out: &mut String, selector: &str, role: &Role, indent: Option<String>, outer: bool) {
    let family = if role.font.mono { "var(--mono)" } else { "var(--body)" };
    let first = indent.unwrap_or_else(|| format!("{}pt", role.first - role.rest));
    let (before, after) = if outer { (role.marg_before, role.marg_after) } else { (0.0, 0.0) };
    let squeeze = if outer { role.squeeze } else { 0.0 };
    let _ = write!(out, "{selector} {{ font-family: {family}; font-size: {}pt; line-height: {}pt; font-weight: {}; font-style: {}; \
        text-transform: {}; text-align: {}; text-indent: {first}; margin: {}pt 0 {}pt;",
        role.size, role.lh,
        if role.font.bold { "bold" } else { "normal" },
        if role.font.italic { "italic" } else { "normal" },
        if role.upper { "uppercase" } else { "none" },
        align_name(role.align),
        role.space_before + before, role.space_after + after);
    if role.rest > 0.0 || squeeze > 0.0 {
        let y = if outer { role.squeeze_y() } else { 0.0 };
        let _ = write!(out, " padding: {y}pt {squeeze}pt {y}pt {}pt;", squeeze + role.rest);
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
    if let Some(bg_color) = role.bg_color {
        let _ = write!(out, " background-color: {};", bg_color.to_hex_tag());
    }
    if let Some(text_color) = role.text_color {
        let _ = write!(out, " color: {};", text_color.to_hex_tag());
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
    let rules: [(&str, &Role, Option<String>, bool); 16] = [
        ("main p", &style.body, None, true),
        ("main p.comment", &style.comment, None, true),
        ("main h1", &style.headers[0], None, true),
        ("main h2", &style.headers[1], None, true),
        ("main h3", &style.headers[2], None, true),
        ("main h4", &style.headers[3], None, true),
        ("main h5", &style.headers[4], None, true),
        ("main .list > .item", list, Some(list_indent(style)), false),
        ("main .figure", &style.figure, None, true),
        ("main .formula", &style.formula, None, true),
        ("main .code", &style.code, None, true),
        ("main .table", &style.table, None, true),
        ("main .table td", &style.cell, None, false),
        ("main .table th", &style.head_cell, None, false),
        ("main .notes > .note", &style.notes.role, None, false),
        ("main .formula > .name", &style.body, None, false),
    ];
    for (sel, role, indent, outer) in rules {
        role_rule(&mut out, sel, role, indent, outer);
    }
    for (sel, cap) in [("main .table > .cap", &style.captions.table), ("main .figure > .cap", &style.captions.image), ("main .code > .cap", &style.captions.code)] {
        role_rule(&mut out, sel, &cap.role, None, false);
    }
    for (sel, role, top) in [("main .list", list, ""), ("main .notes", &style.notes.role, ".4em + ")] {
        let (y, x) = (role.squeeze_y(), role.squeeze);
        let _ = write!(out, "{sel} {{ margin: {}pt 0 {}pt; padding: calc({top}{y}pt) {x}pt {y}pt;", role.marg_before, role.marg_after);
        if let Some(bg) = role.bg_color {
            let _ = write!(out, " background-color: {};", bg.to_hex_tag());
        }
        out.push_str(" }\n");
    }
    if style.list_hang {
        let _ = writeln!(out, "main .list > .item {{ display: flex; padding-left: calc({}pt + var(--level, 0) * {}pt); }}", list.first, style.list_step);
        out.push_str("main .list > .item > .marker { flex: none; white-space: pre; }\n");
        out.push_str("main .list > .item > .text { flex: 1; min-width: 0; }\n");
    }
    title_bar_css(&mut out, style);
    let ext = &style.ext_link;
    let _ = write!(out, "main a.ext {{ text-decoration: {};", if ext.under { "underline" } else { "none" });
    if let Some(color) = ext.color {
        let _ = write!(out, " color: {};", color.to_hex_tag());
    }
    out.push_str(" }
");
    if let Some(bg) = style.mono_bg {
        let _ = writeln!(out, "main .mono {{ background-color: {}; }}", bg.to_hex_tag());
    }
    code_css(&mut out, style);
    table_css(&mut out, style);
    if !style.notes.rule {
        out.push_str("main .notes { background-image: none; }\n");
    }
    out
}

fn list_indent(style: &Style) -> String {
    let list = &style.list;
    if style.list_hang {
        "0".into()
    } else {
        format!("calc({}pt + var(--level, 0) * {}pt)", list.first - list.rest, style.list_step)
    }
}

fn title_bar_css(out: &mut String, style: &Style) {
    let bar = &style.title_bar;
    let role = &style.doc_title;
    let _ = writeln!(out, ":root {{ --title-h: {}pt; }}", (role.size * 1.5).max(36.0));
    let _ = write!(out, "header.title {{ font-size: {}pt; font-weight: {}; font-style: {}; text-transform: {}; text-align: {};",
        role.size,
        if role.font.bold { "bold" } else { "normal" },
        if role.font.italic { "italic" } else { "normal" },
        if role.upper { "uppercase" } else { "none" },
        align_name(role.align));
    if let Some(bg) = bar.bg {
        let _ = write!(out, " background: {};", bg.to_hex_tag());
    }
    if let Some(color) = bar.color {
        let _ = write!(out, " color: {};", color.to_hex_tag());
    }
    out.push_str(" }\n");
}

fn code_css(out: &mut String, style: &Style) {
    let pad_x = style.code_frame.pad_x;
    if style.code_border {
        out.push_str("main .code > .body { border: var(--border) solid var(--ink); }\n");
        out.push_str("main .code > .body > pre { padding: var(--code-pad); }\n");
        out.push_str("main .code > .body > .nums { border-right: var(--border) solid var(--ink); }\n");
    } else {
        let _ = writeln!(out, "main .code > .body > .nums {{ padding-right: {}pt; }}", 2.0 * pad_x);
    }
}

fn table_css(out: &mut String, style: &Style) {
    let lines = &style.table_lines;
    if !lines.frame {
        out.push_str("main .table > table { border-style: hidden; }\n");
    }
    for (cell, grid) in [("th", lines.head), ("td", lines.body)] {
        if !grid.rows {
            let _ = writeln!(out, "main .table {cell} {{ border-top-style: none; border-bottom-style: none; }}");
        }
        if !grid.cols {
            let _ = writeln!(out, "main .table {cell} {{ border-left-style: none; border-right-style: none; }}");
        }
    }
    if !lines.head.rows {
        out.push_str("main .table thead + tbody > tr:first-child > td { border-top-style: hidden; }\n");
    }
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
                out.write_str(if matches!(link, Link::Url(_)) { "<a class=\"ext\" href=\"" } else { "<a href=\"" })?;
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

    fn captioned(&self, out: &mut String, caption: &Caption, body: impl FnOnce(&mut String) -> fmt::Result) -> fmt::Result {
        let after = self.ctx.style.captions.of(caption.kind).caption_after;
        if !after {
            self.caption(out, caption)?;
        }
        body(out)?;
        if after {
            self.caption(out, caption)?;
        }
        Ok(())
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
                    write!(out, "<div class=\"item\" style=\"--level: {}\"><span class=\"marker\">", item.level)?;
                    escape(out, &item.marker)?;
                    out.write_str("</span><span class=\"text\">")?;
                    self.runs(out, &item.runs)?;
                    out.write_str("</span></div>")?;
                }
                out.write_str("</div>")
            }
            Block::Figure { caption, image, scale, anchor } => {
                write!(out, "<div class=\"figure\" id=\"{}\">", anchor.id())?;
                self.captioned(out, caption, |out| {
                    out.write_str("<div class=\"body\">")?;
                    match image {
                        Some(id) => {
                            out.write_str("<img alt=\"\" src=\"")?;
                            escape(out, &self.ctx.res.href(self.file, *id))?;
                            write!(out, "\" style=\"zoom: {scale}%\">")?;
                        }
                        None => out.write_str("<span class=\"error\">??</span>")?,
                    }
                    out.write_str("</div>")
                })?;
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
                self.captioned(out, caption, |out| {
                    out.write_str("<div class=\"body\">")?;
                    if style.code_numbers {
                        out.write_str("<pre class=\"nums\" aria-hidden=\"true\">")?;
                        for n in 1..=lines.len() {
                            writeln!(out, "{n}")?;
                        }
                        out.write_str("</pre>")?;
                    }
                    out.write_str("<pre class=\"src\"><code>")?;
                    for line in lines {
                        escape(out, line)?;
                        out.write_char('\n')?;
                    }
                    out.write_str("</code></pre></div>")
                })?;
                out.write_str("</div>")
            }
            Block::Table { caption, aligns, head, body, anchor, .. } => {
                write!(out, "<div class=\"table\" id=\"{}\">", anchor.id())?;
                self.captioned(out, caption, |out| {
                    out.write_str("<table>")?;
                    if !head.is_empty() {
                        let head_aligns = vec![style.head_cell.align; aligns.len()];
                        out.write_str("<thead>")?;
                        self.cells(out, head, &head_aligns, "th")?;
                        out.write_str("</thead>")?;
                    }
                    out.write_str("<tbody>")?;
                    self.cells(out, body, aligns, "td")?;
                    out.write_str("</tbody></table>")
                })?;
                out.write_str("</div>")
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
        w.runs(&mut out, &ir::styled_runs(&doc.title, ctx.style))?;
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
                w.runs(&mut out, &ir::styled_runs(&item.name, ctx.style))?;
                out.write_str("</a>")?;
            }
            out.write_str("</nav>")?;
        }
        out.write_str("<main>")?;
        out.write_str(&main)?;
        out.write_str("</main></body></html>")?;
        Ok(out)
    }

    fn joined(&self, _project: &ir::Project, _ctx: &PageCtx) -> Result<String, fmt::Error> {
        unreachable!("html с single собирает постраничный вывод")
    }
}
