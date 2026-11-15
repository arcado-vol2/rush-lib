use super::PageCtx;
use crate::color::Color;
use crate::config::{DocOut, Theme};
use crate::content::Content;
use crate::render::ir::{self, Block, Caption, Cell, Link, ParaKind, Run, RunText};
use crate::render::warning::Feature;
use crate::render::{css_fonts, escape, Semantic};
use crate::resources::{FileId, Id, Resources};
use crate::style::{Align, DirCase, Role, Style};
use crate::OutFile;
use std::fmt::{self, Write};
use std::path::Path;

const BASE_CSS: &str = include_str!("../../templates/html/base.css");
pub(crate) const CSS_NAME: &str = "style.css";

/// Scripts to save choises
const THEME_INIT: &str = "<script>try{var t=localStorage.getItem('rush-theme');\
if(t=='light'||t=='dark')document.documentElement.dataset.theme=t}catch(e){}</script>";
const THEME_SWITCH: &str = "var d=document.documentElement,t=d.dataset.theme||\
(matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light');t=t=='dark'?'light':'dark';\
d.dataset.theme=t;try{localStorage.setItem('rush-theme',t)}catch(e){}";
const NAV_INIT: &str = "<script>(function(){var b=document.getElementById('nav-toggle'),w=matchMedia('(min-width: 901px)');\
try{if(w.matches&&localStorage.getItem('rush-nav')=='hidden')b.checked=true}catch(e){}\
b.addEventListener('change',function(){if(w.matches)try{localStorage.setItem('rush-nav',b.checked?'hidden':'shown')}catch(e){}})})()</script>";

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
    out.push_str(" }\n");
}

fn role_rules(style: &Style) -> Vec<(&'static str, &Role, Option<String>, bool)> {
    let list = &style.list;
    vec![
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
        ("main .table > .cap", &style.captions.table.role, None, false),
        ("main .figure > .cap", &style.captions.image.role, None, false),
        ("main .code > .cap", &style.captions.code.role, None, false),
    ]
}

fn color(c: Option<Color>, none: &str) -> String {
    c.map_or_else(|| none.to_string(), |c| c.to_hex_tag())
}

fn palette_css(out: &mut String, style: &Style, scope: &str) {
    let c = &style.chrome;
    let bar = &style.title_bar;
    let root = if scope.is_empty() { ":root" } else { scope };
    let _ = writeln!(out, "{root} {{ --paper: {}; --ink: {}; --line: {}; --nav-bg: {}; --nav-color: {}; --nav-cur-bg: {}; \
        --nav-cur-color: {}; --nav-cur-bar: {}; --nav-dir: {}; --accent: {}; --bar-bg: {}; --bar-color: {}; }}",
        c.paper.to_hex_tag(), c.ink.to_hex_tag(), c.line.to_hex_tag(), c.nav_bg.to_hex_tag(), c.nav_color.to_hex_tag(),
        color(c.nav_current_bg, "transparent"), c.nav_current_color.to_hex_tag(), c.nav_current_bar.to_hex_tag(),
        c.nav_dir.to_hex_tag(), c.accent.to_hex_tag(), color(bar.bg, "var(--paper)"), color(bar.color, "var(--ink)"));
    let sel = |s: &str| if scope.is_empty() { s.to_string() } else { format!("{scope} {s}") };
    for (s, role, _, _) in role_rules(style) {
        let _ = writeln!(out, "{} {{ background-color: {}; color: {}; }}", sel(s), color(role.bg_color, "transparent"), color(role.text_color, "inherit"));
    }
    for (s, role) in [("main .list", &style.list), ("main .notes", &style.notes.role)] {
        let _ = writeln!(out, "{} {{ background-color: {}; }}", sel(s), color(role.bg_color, "transparent"));
    }
    for (s, cap) in [("main .table > .cap > .index", &style.captions.table), ("main .figure > .cap > .index", &style.captions.image), ("main .code > .cap > .index", &style.captions.code)] {
        let _ = writeln!(out, "{} {{ color: {}; }}", sel(s), color(cap.index_color, "inherit"));
    }
    let _ = writeln!(out, "{} {{ color: {}; }}", sel("main a.ext"), color(style.ext_link.color, "inherit"));
    let _ = writeln!(out, "{} {{ background-color: {}; }}", sel("main .mono"), color(style.mono_bg, "transparent"));
    let _ = writeln!(out, "{} {{ background-color: transparent; }}", sel("main .code .mono"));
    if let Some(bar) = style.comment_bar {
        let _ = writeln!(out, "{} {{ border-left: 3pt solid {}; }}", sel("main p.comment"), bar.to_hex_tag());
    }
}

fn css(style: &Style) -> String {
    let mut out = css_fonts(style, "fonts");
    let f = &style.fonts;
    let [top, right, bottom, left] = style.screen.pad;
    let _ = write!(out, ":root {{ --body: \"{}\", {}; --mono: \"{}\", {}; --size: {}pt; --lh: {}pt; --small: {}pt; \
        --border: {}pt; --cell-pad: {}pt {}pt; --code-pad: {}pt {}pt; --note-rule: {}pt; --image-gap: {}pt; \
        --column: {}pt; --column-pad: {top}pt {right}pt {bottom}pt {left}pt; }}\n",
        f.body.name, f.body.fallback, f.mono.name, f.mono.fallback,
        style.body.size, style.body.lh, style.cell.size,
        style.border, style.cell_frame.pad_y, style.cell_frame.pad_x, style.code_frame.pad_y, style.code_frame.pad_x,
        style.notes.rule_w, style.images.gap, style.screen.column);
    out.push_str(BASE_CSS);
    let list = &style.list;
    for (sel, role, indent, outer) in role_rules(style) {
        role_rule(&mut out, sel, role, indent, outer);
    }
    for (sel, role, top) in [("main .list", list, ""), ("main .notes", &style.notes.role, ".4em + ")] {
        let (y, x) = (role.squeeze_y(), role.squeeze);
        let _ = writeln!(out, "{sel} {{ margin: {}pt 0 {}pt; padding: calc({top}{y}pt) {x}pt {y}pt; }}", role.marg_before, role.marg_after);
    }
    if style.list_hang {
        let _ = writeln!(out, "main .list > .item {{ display: flex; padding-left: calc({}pt + var(--level, 0) * {}pt); }}", list.first, style.list_step);
        out.push_str("main .list > .item > .marker { flex: none; white-space: pre; }\n");
        out.push_str("main .list > .item > .text { flex: 1; min-width: 0; }\n");
    }
    title_bar_css(&mut out, style);
    chrome_css(&mut out, style);
    let ext = &style.ext_link;
    let _ = writeln!(out, "main a.ext {{ text-decoration: {}; }}", if ext.under { "underline" } else { "none" });
    code_css(&mut out, style);
    table_css(&mut out, style);
    if !style.notes.rule {
        out.push_str("main .notes { background-image: none; }\n");
    }
    palette_css(&mut out, style, "");
    if let Some(dark) = style.dark {
        // dark theme only on screen, in printing always light
        let dark = dark();
        out.push_str("@media screen {\n");
        out.push_str(":root[data-theme=\"dark\"] { color-scheme: dark; }\n");
        palette_css(&mut out, &dark, ":root[data-theme=\"dark\"]");
        out.push_str("}\n@media screen and (prefers-color-scheme: dark) {\n");
        out.push_str(":root:not([data-theme=\"light\"]) { color-scheme: dark; }\n");
        palette_css(&mut out, &dark, ":root:not([data-theme=\"light\"])");
        out.push_str("}\n");
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
    let role = &style.doc_title;
    let size = style.title_bar.size.unwrap_or(role.size);
    let _ = writeln!(out, ":root {{ --title-h: {}pt; }}", (size * 1.5).max(36.0));
    let _ = writeln!(out, "header.title > .name {{ font-size: {size}pt; font-weight: {}; font-style: {}; text-transform: {}; text-align: {}; }}",
        if role.font.bold { "bold" } else { "normal" },
        if role.font.italic { "italic" } else { "normal" },
        if role.upper { "uppercase" } else { "none" },
        align_name(role.align));
}

fn chrome_css(out: &mut String, style: &Style) {
    let c = &style.chrome;
    let _ = writeln!(out, ":root {{ --nav-cur-weight: {}; }}", if c.nav_current_bold { "bold" } else { "normal" });
    let case = match c.dir_case {
        DirCase::Plain => "",
        DirCase::Upper => " text-transform: uppercase; letter-spacing: .04em;",
        DirCase::SmallCaps => " font-variant: small-caps; letter-spacing: .06em;",
    };
    let _ = writeln!(out, "nav.toc details.dir > summary {{ font-weight: bold;{case} }}");
}

fn code_css(out: &mut String, style: &Style) {
    let pad_x = style.code_frame.pad_x;
    if style.code_border {
        out.push_str("main .code > .body { border: var(--border) solid var(--line); }\n");
        out.push_str("main .code > .body > pre { padding: var(--code-pad); }\n");
        out.push_str("main .code > .body > .nums { border-right: var(--border) solid var(--line); }\n");
    } else {
        let _ = writeln!(out, "main .code > .body > .nums {{ padding-right: {}pt; }}", 2.0 * pad_x);
    }
    if style.code_radius > 0.0 {
        let _ = writeln!(out, "main .code {{ border-radius: {}pt; }}", style.code_radius);
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
        if caption.label.is_empty() && caption.name.is_none() {
            return Ok(());
        }
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
            // TODO: add support with syntect
            Block::Code { caption, lines, anchor, lang: _lang } => {
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
            Block::Table { caption, aligns, widths, head, body, anchor, .. } => {
                write!(out, "<div class=\"table\" id=\"{}\">", anchor.id())?;
                self.captioned(out, caption, |out| {
                    match widths {
                        Some(widths) => {
                            out.write_str("<table style=\"table-layout: fixed\"><colgroup>")?;
                            for w in widths {
                                write!(out, "<col style=\"width: {:.2}%\">", w * 100.0)?;
                            }
                            out.write_str("</colgroup>")?;
                        }
                        None => out.write_str("<table>")?,
                    }
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

fn group_has(items: &[crate::content::ContentItem], from: usize, depth: usize, file: Id<FileId>) -> bool {
    let mut j = from;
    loop {
        if items[j].file == file {
            return true;
        }
        j += 1;
        if j >= items.len() || items[j].dirs.len() - items[j].opens <= depth {
            return false;
        }
    }
}

impl Writer<'_> {
    fn nav(&self, out: &mut String, content: &Content) -> fmt::Result {
        let (res, style) = (self.ctx.res, self.ctx.style);
        out.write_str("<nav class=\"toc\">")?;
        if let Some(title) = style.chrome.nav_title {
            out.write_str("<div class=\"nav-title\">")?;
            escape(out, title)?;
            out.write_str("</div>")?;
        }
        let items = &content.items;
        let mut open = 0;
        for (i, item) in items.iter().enumerate() {
            let shared = item.dirs.len() - item.opens;
            while open > shared {
                out.write_str("</details>")?;
                open -= 1;
            }
            for depth in shared..item.dirs.len() {
                out.write_str("<details class=\"dir\"")?;
                if group_has(items, i, depth, self.file) {
                    out.write_str(" open")?;
                }
                write!(out, "><summary style=\"--level: {depth}\">")?;
                self.runs(out, &ir::styled_runs(&item.dirs[depth], style))?;
                out.write_str("</summary>")?;
            }
            open = item.dirs.len();
            out.write_str("<a class=\"item")?;
            if item.file == self.file {
                out.write_str(" current\" aria-current=\"page")?;
            }
            write!(out, "\" style=\"--level: {}\" href=\"", item.dirs.len())?;
            escape(out, &file_href(res, self.file, item.file))?;
            out.write_str("\">")?;
            self.runs(out, &ir::styled_runs(&item.name, style))?;
            out.write_str("</a>")?;
        }
        for _ in 0..open {
            out.write_str("</details>")?;
        }
        out.write_str("</nav>")
    }

    fn page_toc(&self, out: &mut String, doc: &ir::Doc) -> fmt::Result {
        let max = self.ctx.style.toc.map_or(3, |t| t.max_level);
        let headers: Vec<_> = doc.blocks.iter().filter_map(|b| match b {
            Block::Header { level, number, runs, anchor } if *level <= max => Some((*level, number, runs, anchor)),
            _ => None,
        }).collect();
        let Some(top) = headers.iter().map(|h| h.0).min() else { return Ok(()) };
        out.write_str("<aside class=\"page-toc\">")?;
        for (level, number, runs, anchor) in headers {
            write!(out, "<a style=\"--level: {}\" href=\"#{}\">", level - top, anchor.id())?;
            if let Some(n) = number {
                escape(out, n)?;
                out.write_char(' ')?;
            }
            self.runs(out, runs)?;
            out.write_str("</a>")?;
        }
        out.write_str("</aside>")
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
        let (res, style) = (ctx.res, ctx.style);
        let file = doc.file;
        let w = Writer { ctx, file };
        let mut main = String::new();
        for block in &doc.blocks {
            w.block(&mut main, block)?;
        }
        let has_nav = !ctx.content.items.is_empty();
        let burger = style.nav_toggle && ctx.scripts && has_nav;
        let theme_button = style.theme_toggle && ctx.scripts && style.dark.is_some();

        let mut out = String::from("<!DOCTYPE html><html lang=\"ru\"");
        match ctx.theme {
            Theme::Light if style.dark.is_some() => out.push_str(" data-theme=\"light\""),
            Theme::Dark => out.push_str(" data-theme=\"dark\""),
            _ => {}
        }
        out.push_str("><head><meta charset=\"UTF-8\">");
        out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>");
        escape(&mut out, &doc.title)?;
        out.write_str("</title><link rel=\"stylesheet\" href=\"")?;
        escape(&mut out, &res.root_href(file, CSS_NAME))?;
        out.write_str("\">")?;
        if theme_button {
            out.push_str(THEME_INIT);
        }
        out.write_str("</head><body>")?;
        if burger {
            out.push_str("<input type=\"checkbox\" id=\"nav-toggle\" class=\"nav-toggle\">");
            out.push_str(NAV_INIT);
        }
        out.write_str("<header class=\"title\">")?;
        if burger {
            out.push_str("<label for=\"nav-toggle\" class=\"burger\" title=\"Меню\" aria-label=\"Меню\">☰</label>");
        }
        out.write_str("<span class=\"name\">")?;
        w.runs(&mut out, &ir::styled_runs(&doc.title, style))?;
        out.write_str("</span>")?;
        if theme_button {
            write!(out, "<button type=\"button\" class=\"theme\" title=\"Тема\" aria-label=\"Сменить тему\" onclick=\"{THEME_SWITCH}\">◐</button>")?;
        }
        out.write_str("</header>")?;
        if has_nav {
            w.nav(&mut out, ctx.content)?;
        }
        out.write_str("<main>")?;
        out.write_str(&main)?;
        out.write_str("</main>")?;
        if style.page_toc {
            w.page_toc(&mut out, doc)?;
        }
        out.write_str("</body></html>")?;
        Ok(out)
    }

    fn joined(&self, _project: &ir::Project, _ctx: &PageCtx) -> Result<String, fmt::Error> {
        unreachable!("html с single собирает постраничный вывод")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DocStyle};
    use crate::project::Project;

    fn site(config: Config) -> (Vec<OutFile>, Vec<crate::Warning>) {
        let mut p = Project::new();
        p.add_file("intro", "intro.rush", "\\++ Раздел\n\nтекст".into()).unwrap();
        p.add_file("a/one", "a/one.rush", "текст".into()).unwrap();
        p.add_file("a/b/two", "a/b/two.rush", "текст".into()).unwrap();
        p.add_file("c/three", "c/three.rush", "текст".into()).unwrap();
        for (name, key) in [("Введение", "intro"), ("Один", "a/one"), ("Два", "a/b/two"), ("Три", "c/three")] {
            p.add_content(name.into(), key.into());
        }
        p.add_dir("Раздел A".into(), "a".into());
        crate::build(&config, &p).unwrap()
    }

    fn page<'f>(files: &'f [OutFile], path: &str) -> &'f str {
        std::str::from_utf8(&files.iter().find(|f| f.path == path).unwrap().data).unwrap()
    }

    #[test]
    fn nav_groups_dirs_and_opens_current() {
        let (files, _) = site(Config { style: DocStyle::Blog, ..Config::default() });
        let two = page(&files, "a/b/two.html");
        assert!(two.contains("<details class=\"dir\" open><summary style=\"--level: 0\">Раздел A</summary>"), "{two}");
        assert!(two.contains("<details class=\"dir\" open><summary style=\"--level: 1\">b</summary>"), "{two}");
        assert!(two.contains("<details class=\"dir\"><summary style=\"--level: 0\">c</summary>"), "{two}");
        assert!(two.contains("Два</a></details></details><details"), "{two}");
        let intro = page(&files, "intro.html");
        assert!(!intro.contains("<details class=\"dir\" open>"), "{intro}");
    }

    #[test]
    fn controls_by_style() {
        for (style, burger, theme, toc) in [
            (DocStyle::Gost7_32, false, false, false),
            (DocStyle::Modern, true, false, true),
            (DocStyle::Blog, true, true, true),
        ] {
            let (files, _) = site(Config { style, ..Config::default() });
            let intro = page(&files, "intro.html");
            assert_eq!(intro.contains("class=\"burger\""), burger, "{style:?}");
            assert_eq!(intro.contains("class=\"theme\""), theme, "{style:?}");
            assert_eq!(intro.contains("<aside class=\"page-toc\">"), toc, "{style:?}");
            assert_eq!(intro.contains("<script>"), burger || theme, "{style:?}");
        }
    }

    #[test]
    fn no_js_has_no_scripts() {
        for style in [DocStyle::Gost7_32, DocStyle::Modern, DocStyle::Blog] {
            let (files, _) = site(Config { style, no_js: true, ..Config::default() });
            let intro = page(&files, "intro.html");
            assert!(!intro.contains("<script") && !intro.contains("onclick"), "{style:?}: {intro}");
            assert!(!intro.contains("class=\"burger\"") && !intro.contains("class=\"theme\""), "{style:?}");
        }
    }

    #[test]
    fn blog_theme_attribute() {
        for (theme, attr) in [(Theme::Auto, None), (Theme::Light, Some("light")), (Theme::Dark, Some("dark"))] {
            let (files, warns) = site(Config { style: DocStyle::Blog, no_js: true, theme, ..Config::default() });
            assert!(warns.is_empty(), "{warns:?}");
            let intro = page(&files, "intro.html");
            match attr {
                Some(a) => assert!(intro.starts_with(&format!("<!DOCTYPE html><html lang=\"ru\" data-theme=\"{a}\">")), "{intro}"),
                None => assert!(intro.starts_with("<!DOCTYPE html><html lang=\"ru\">"), "{intro}"),
            }
        }
        let (files, _) = site(Config { style: DocStyle::Blog, ..Config::default() });
        let css = page(&files, "style.css");
        assert!(css.contains("@media screen and (prefers-color-scheme: dark)"), "{css}");
        assert!(css.contains(":root[data-theme=\"dark\"] main p {"), "{css}");
    }

    #[test]
    fn blog_headers_without_numbers() {
        let (files, _) = site(Config { style: DocStyle::Blog, ..Config::default() });
        let intro = page(&files, "intro.html");
        assert!(intro.contains("\">Раздел</h2>") && !intro.contains("<span class=\"index\">"), "{intro}");
    }
}
