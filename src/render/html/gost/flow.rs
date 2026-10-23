use super::inline::{Align, Base, Ctx, Inline, Layout, Line, Notes};
use super::metrics::{width, Font};
use super::*;
use crate::parse::common::Named;
use crate::parse::rush::{Code, Comment, Formula, Header, Image, InlineEl, List, ListEl, ListElKind, Node, Paragraph, Span, Table, TableEl};
use crate::render::html::gostlike::gost_caption;
use crate::render::html::{align_class, merge_cells, CaptionKind, Slot};
use crate::parse::rush::Aligment;
use latex2mathml::{latex_to_mathml, DisplayStyle};
use std::collections::HashSet;

const IMG_GAP: f32 = 6.0;
const PX: f32 = 0.75;
const BROKEN_IMG_H: f32 = 3.0 * LH;

#[derive(Debug, Default, Clone)]
pub(crate) struct NoteBody {
    pub(crate) html: String,
    pub(crate) height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TocEntry {
    pub(crate) level: u8,
    pub(crate) text: String,
    pub(crate) id: String,
}

#[derive(Default)]
pub(crate) struct Flow {
    pub(crate) blocks: Vec<Block>,
    pub(crate) toc: Vec<TocEntry>,
    /// note texts indexed by global note number - 1
    pub(crate) notes: Vec<NoteBody>,
}

fn pieces(lines: Vec<Line>, height: f32) -> Vec<Piece> {
    lines.into_iter().map(|l| Piece { html: l.html(), height, notes: l.notes }).collect()
}

fn lines_html(lines: &[Line]) -> String {
    lines.iter().map(Line::html).collect()
}

fn line_notes(lines: &[Line]) -> Vec<u16> {
    lines.iter().flat_map(|l| l.notes.iter().copied()).collect()
}

fn empty_line() -> Line {
    Line { style: String::new(), body: String::new(), notes: Vec::new() }
}

fn bold(size: f32) -> Base {
    Base { size, font: Font { bold: true, ..Font::default() }, upper: false }
}

/// Header text without markup, for the table of contents
fn plain(spans: &[Span]) -> String {
    let mut s = String::new();
    for span in spans {
        for el in &span.text {
            match el {
                InlineEl::String(t) => s.push_str(t),
                InlineEl::Space => s.push(' '),
                InlineEl::LongDash => s.push('—'),
                InlineEl::InnerLink(Ok(l)) => s.push_str(&l.index.to_string()),
                InlineEl::InnerLink(Err(_)) => s.push_str("??"),
                InlineEl::Note(_) => (),
            }
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn refs_in(spans: &[Span], out: &mut HashSet<u16>) {
    for el in spans.iter().flat_map(|s| s.text.iter()) {
        if let InlineEl::Note(Ok(i)) = el {
            out.insert(*i);
        }
    }
}

fn named_refs(named: &Named<Vec<Span>>, out: &mut HashSet<u16>) {
    if let Named::WithName { name, .. } = named {
        refs_in(name, out);
    }
}

fn list_refs(items: &[ListEl], out: &mut HashSet<u16>) {
    for el in items {
        if let Some(ListElKind::Ordered(s) | ListElKind::Unordered(s)) = &el.this {
            refs_in(s, out);
        }
        list_refs(&el.children, out);
    }
}

fn note_refs(nodes: &[Node]) -> HashSet<u16> {
    let mut out = HashSet::new();
    for node in nodes {
        match node {
            Node::Paragraph(p) => refs_in(&p.content, &mut out),
            Node::Comment(c) => refs_in(&c.content, &mut out),
            Node::Header(h) => refs_in(&h.content, &mut out),
            Node::Formula(f) => named_refs(&f.named, &mut out),
            Node::Code(c) => named_refs(&c.named, &mut out),
            Node::Image(i) => named_refs(&i.meta.named, &mut out),
            Node::List(l) => list_refs(&l.content, &mut out),
            Node::Table(t) => {
                named_refs(&t.named, &mut out);
                for el in t.headers.iter().chain(&t.cells).flatten() {
                    if let TableEl::Cell(s) = el {
                        refs_in(s, &mut out);
                    }
                }
            }
            Node::Note(_) => (),
        }
    }
    out
}

fn approx_formula_height(src: &str) -> f32 {
    let count = |pats: &[&str]| pats.iter().map(|p| src.matches(p).count()).sum::<usize>();
    let rows = 1 + count(&["\\\\"]);
    let fracs = count(&["\\frac", "\\dfrac", "\\tfrac", "\\binom", "\\over"]).min(3);
    let big = count(&["\\sum", "\\prod", "\\int", "\\oint", "\\lim", "\\bigcup", "\\bigcap"]) > 0;
    let h = rows as f32 * SIZE * 1.6 + fracs as f32 * SIZE + if big { SIZE * 0.8 } else { 0.0 };
    h.max(LH)
}

struct Cell {
    col: usize,
    w: usize,
    h: usize,
    text: Option<Inline>,
    join_top: bool,
    join_left: bool,
}

fn cells(rows: Vec<Vec<Slot>>, ctx: &mut Ctx) -> Vec<Vec<Cell>> {
    rows.into_iter().map(|row| row.into_iter().map(|slot| match slot {
        Slot::Cell { col, w, h, content } => {
            let mut il = Inline::new(Base::new(SMALL));
            il.spans(content, ctx);
            Cell { col, w, h, text: Some(il), join_top: false, join_left: false }
        }
        Slot::Empty { col, join_top, join_left } => Cell { col, w: 1, h: 1, text: None, join_top, join_left },
    }).collect()).collect()
}

const CELL_EXTRA_W: f32 = 2.0 * CELL_PAD_X + BORDER;
const CELL_EXTRA_H: f32 = 2.0 * CELL_PAD_Y + BORDER;

fn column_widths(rows: &[&Vec<Cell>], cols: usize) -> Vec<f32> {
    let mut nat = vec![CELL_EXTRA_W + SMALL; cols];
    let mut min = nat.clone();
    for cell in rows.iter().flat_map(|r| r.iter()) {
        if let (1, Some(il)) = (cell.w, &cell.text) {
            nat[cell.col] = nat[cell.col].max(il.natural_width() + CELL_EXTRA_W);
            min[cell.col] = min[cell.col].max(il.min_width() + CELL_EXTRA_W);
        }
    }
    let (sn, sm): (f32, f32) = (nat.iter().sum(), min.iter().sum());
    if sn <= TEXT_W {
        nat.iter().map(|n| n + (TEXT_W - sn) * n / sn).collect()
    } else if sm <= TEXT_W {
        let flex = sn - sm;
        nat.iter().zip(&min).map(|(n, m)| m + (TEXT_W - sm) * (n - m) / flex).collect()
    } else {
        min.iter().map(|m| TEXT_W * m / sm).collect()
    }
}

struct Row {
    html: String,
    height: f32,
    notes: Vec<u16>,
    reach: usize,
}

fn table_rows(rows: Vec<Vec<Cell>>, widths: &[f32], aligns: &[Aligment], tag: &str) -> Vec<Row> {
    let n = rows.len();
    let mut laid: Vec<Vec<(Cell, Vec<Line>)>> = Vec::with_capacity(n);
    for row in rows {
        laid.push(row.into_iter().map(|cell| {
            let lines = match &cell.text {
                Some(il) => {
                    let w: f32 = widths[cell.col..cell.col + cell.w].iter().sum();
                    let align = match (tag, aligns[cell.col]) {
                        ("th", _) | (_, Aligment::Center) => Align::Center,
                        (_, Aligment::Left) => Align::Left,
                        (_, Aligment::Right) => Align::Right,
                    };
                    il.lines(Layout { width: w - CELL_EXTRA_W, first: 0.0, rest: 0.0, align })
                }
                None => Vec::new(),
            };
            (cell, lines)
        }).collect());
    }
    let need = |lines: &[Line]| lines.len().max(1) as f32 * SMALL_LH + CELL_EXTRA_H;
    let mut heights = vec![need(&[]); n];
    for (r, row) in laid.iter().enumerate() {
        for (cell, lines) in row {
            if cell.h == 1 {
                heights[r] = heights[r].max(need(lines));
            }
        }
    }
    for (r, row) in laid.iter().enumerate() {
        for (cell, lines) in row {
            if cell.h > 1 {
                let span: f32 = heights[r..r + cell.h].iter().sum();
                if need(lines) > span {
                    heights[r + cell.h - 1] += need(lines) - span;
                }
            }
        }
    }

    laid.into_iter().zip(heights).enumerate().map(|(r, (row, height))| {
        let mut html = format!("<tr style=\"height:{height:.2}pt\">");
        let mut notes = Vec::new();
        let mut reach = r + 1;
        for (cell, lines) in row {
            let align = if tag == "th" { "center" } else { align_class(aligns[cell.col]) };
            if cell.text.is_none() {
                html.push_str(&format!("<{tag} class=\"{align}"));
                if cell.join_top {
                    html.push_str(" join-top");
                }
                if cell.join_left {
                    html.push_str(" join-left");
                }
                html.push_str(&format!("\"></{tag}>"));
                continue;
            }
            html.push_str(&format!("<{tag} class=\"{align}\""));
            if cell.w > 1 {
                html.push_str(&format!(" colspan=\"{}\"", cell.w));
            }
            if cell.h > 1 {
                html.push_str(&format!(" rowspan=\"{}\"", cell.h));
            }
            html.push('>');
            html.push_str(&lines_html(&lines));
            html.push_str(&format!("</{tag}>"));
            notes.extend(line_notes(&lines));
            reach = reach.max(r + cell.h);
        }
        html.push_str("</tr>");
        Row { html, height, notes, reach }
    }).collect()
}

impl Flow {
    fn push(&mut self, block: Block) {
        self.blocks.push(block);
    }

    fn set_note(&mut self, g: u16, body: NoteBody) {
        let i = g as usize - 1;
        if self.notes.len() <= i {
            self.notes.resize_with(i + 1, NoteBody::default);
        }
        self.notes[i] = body;
    }

    fn note_body(&mut self, g: u16, spans: Vec<Span>, ctx: &mut Ctx) {
        let mut il = Inline::new(Base::new(SMALL));
        il.raw(format!("<sup class=\"fr\">{g}</sup>"), g.to_string(), 0.7);
        il.space();
        il.spans(spans, ctx);
        let lines = il.lines(Layout { width: TEXT_W, first: INDENT, rest: 0.0, align: Align::Justify });
        let height = lines.len() as f32 * SMALL_LH;
        let html = format!("<div class=\"note\" id=\"n-{g}\">{}</div>", lines_html(&lines));
        self.set_note(g, NoteBody { html, height });
    }

    fn doc(&mut self, k: usize, doc: Doc, ctx: &mut Ctx) {
        let mut il = Inline::new(Base { upper: true, ..bold(SIZE) });
        il.text(doc.name);
        let mut lines = il.lines(Layout { width: TEXT_W, first: 0.0, rest: 0.0, align: Align::Center });
        if lines.is_empty() {
            lines.push(empty_line());
        }
        let id = format!("d-{k}");
        self.toc.push(TocEntry { level: 1, text: doc.name.to_string(), id: id.clone() });
        self.push(Block {
            pieces: pieces(lines, LH),
            space_after: LH,
            keep_with_next: true,
            break_before: true,
            id: Some(id),
            ..Block::new(Kind::Text { open: "<h1 class=\"doc\">".into(), close: "</h1>" })
        });

        let referenced = note_refs(&doc.rush.nodes);
        let mut defs: HashMap<u16, Vec<Span>> = HashMap::new();
        for node in doc.rush.nodes {
            match node {
                Node::Paragraph(p) => self.paragraph(p, ctx),
                Node::Comment(c) => self.comment(c, ctx),
                Node::Header(h) => self.header(h, ctx),
                Node::Formula(f) => self.formula(f, ctx),
                Node::Code(c) => self.code(c, ctx),
                Node::Image(i) => self.image(i, ctx),
                Node::List(l) => self.list(l, ctx),
                Node::Table(t) => self.table(t, ctx),
                Node::Note(n) => for item in n.content {
                    if item.index != 0 && referenced.contains(&item.index) {
                        defs.insert(item.index, item.content);
                        continue;
                    }
                    let g = if item.index == 0 { ctx.notes.fresh() } else { ctx.notes.global(item.index) };
                    self.note_body(g, item.content, ctx);
                    self.push(Block {
                        pieces: vec![Piece { html: String::new(), height: 0.0, notes: vec![g] }],
                        ..Block::new(Kind::Anchor)
                    });
                },
            }
        }
        loop {
            let target_notes: Vec<(u16, u16)> = ctx.notes.local.iter()
                .filter(|(l, _)| defs.contains_key(l))
                .map(|(&l, &g)| (l, g))
                .collect();
            if target_notes.is_empty() {
                break;
            }
            for (l, g) in target_notes {
                let spans = defs.remove(&l).unwrap();
                self.note_body(g, spans, ctx);
            }
        }
    }

    fn text_block(&mut self, open: &str, lines: Vec<Line>) {
        if lines.is_empty() {
            return;
        }
        self.push(Block {
            pieces: pieces(lines, LH),
            min_first: 2,
            min_last: 2,
            ..Block::new(Kind::Text { open: open.into(), close: "</p>" })
        });
    }

    fn paragraph(&mut self, p: Paragraph, ctx: &mut Ctx) {
        let mut il = Inline::new(Base::new(SIZE));
        il.spans(p.content, ctx);
        let lines = il.lines(Layout { width: TEXT_W, first: INDENT, rest: 0.0, align: Align::Justify });
        self.text_block("<p>", lines);
    }

    fn comment(&mut self, c: Comment, ctx: &mut Ctx) {
        let mut il = Inline::new(Base { size: SIZE, font: Font { italic: true, ..Font::default() }, upper: false });
        il.text("Комментарий:");
        il.space();
        il.spans(c.content, ctx);
        let lines = il.lines(Layout { width: TEXT_W, first: INDENT, rest: 0.0, align: Align::Justify });
        self.text_block("<p class=\"comment\">", lines);
    }

    fn header(&mut self, h: Header, ctx: &mut Ctx) {
        let level = h.level.clamp(1, 5);
        let index = &h.index[..level as usize];
        let id = format!("h-{}", index.iter().map(u16::to_string).collect::<Vec<_>>().join("-"));
        let number = (2..=4).contains(&level)
            .then(|| index[1..].iter().map(u16::to_string).collect::<Vec<_>>().join("."));
        if level <= 3 {
            let text = plain(&h.content);
            let text = match &number {
                Some(n) => format!("{n} {text}"),
                None => text,
            };
            self.toc.push(TocEntry { level, text, id: id.clone() });
        }

        let (base, lay) = if level == 1 {
            (Base { upper: true, ..bold(SIZE) }, Layout { width: TEXT_W, first: 0.0, rest: 0.0, align: Align::Center })
        } else {
            let base = if level <= 3 { bold(SIZE) } else { Base::new(SIZE) };
            let hang = number.as_ref().map_or(0.0, |n| width(n, base.font, SIZE) + width(" ", base.font, SIZE));
            (base, Layout { width: TEXT_W, first: INDENT, rest: INDENT + hang, align: Align::Left })
        };
        let mut il = Inline::new(base);
        if let Some(n) = &number {
            il.text(n);
            il.space();
        }
        il.spans(h.content, ctx);
        let mut lines = il.lines(lay);
        if lines.is_empty() {
            lines.push(empty_line());
        }
        let (open, close) = match level {
            1 => ("<h1>", "</h1>"),
            2 => ("<h2>", "</h2>"),
            3 => ("<h3>", "</h3>"),
            4 => ("<h4>", "</h4>"),
            _ => ("<h5>", "</h5>"),
        };
        self.push(Block {
            pieces: pieces(lines, LH),
            space_before: LH,
            space_after: if level == 1 { LH } else { LH / 2.0 },
            keep_with_next: true,
            id: Some(id),
            ..Block::new(Kind::Text { open: open.into(), close })
        });
    }

    fn list_items(&mut self, items: Vec<ListEl>, level: usize, out: &mut Vec<Line>, ctx: &mut Ctx) {
        let mut num: u32 = 0;
        for el in items {
            let item = match el.this {
                Some(ListElKind::Ordered(spans)) => {
                    num += 1;
                    Some((format!("{num})\u{a0}"), spans))
                }
                Some(ListElKind::Unordered(spans)) => {
                    num = 0;
                    Some(("–\u{a0}".to_string(), spans))
                }
                None => None,
            };
            if let Some((prefix, spans)) = item {
                let mut il = Inline::new(Base::new(SIZE));
                il.text(&prefix);
                il.spans(spans, ctx);
                let lay = Layout { width: TEXT_W, first: INDENT * (level + 1) as f32, rest: 0.0, align: Align::Justify };
                out.extend(il.lines(lay));
            }
            self.list_items(el.children, level + 1, out, ctx);
        }
    }

    fn list(&mut self, list: List, ctx: &mut Ctx) {
        let mut lines = Vec::new();
        self.list_items(list.content, 0, &mut lines, ctx);
        self.text_block("<p class=\"list\">", lines);
    }

    /// Caption lines "Kind N - name"
    fn caption(&mut self, kind: CaptionKind, named: Named<Vec<Span>>, lay: Layout, ctx: &mut Ctx) -> Vec<Line> {
        let mut il = Inline::new(Base::new(SIZE));
        match named {
            Named::WithName { name, index } => {
                il.text(&gost_caption(kind, index, true));
                if kind == CaptionKind::Image {
                    il.space();
                }
                il.spans(name, ctx);
            }
            Named::Anonymous { index } => il.text(&gost_caption(kind, index, false)),
        }
        il.lines(lay)
    }

    fn image(&mut self, image: Image, ctx: &mut Ctx) {
        let index = match &image.meta.named {
            Named::WithName { index, .. } | Named::Anonymous { index } => *index,
        };
        let scale = image.meta.scale as f32 / 100.0;
        let lay = Layout { width: TEXT_W, first: 0.0, rest: 0.0, align: Align::Center };
        let cap = self.caption(CaptionKind::Image, image.meta.named, lay, ctx);
        let cap_h = cap.len() as f32 * LH;
        let max_h = TEXT_H - SLACK - cap_h - IMG_GAP;

        let (body, img_h) = match image.path {
            Ok(id) => {
                let local = ctx.res.images.local_path(id).to_string_lossy().into_owned();
                let (w, h) = match *ctx.res.images.data(id) {
                    Some((w, h)) => (w as f32 * PX * scale, h as f32 * PX * scale),
                    None => {
                        ctx.warn.push(RenderWarning::ImageSizeUnknown(local));
                        (TEXT_W / 2.0, TEXT_W * 3.0 / 8.0)
                    }
                };
                let fit = (TEXT_W / w.max(1.0)).min(max_h / h.max(1.0)).min(1.0);
                let (w, h) = (w * fit, h * fit);
                let mut img = String::from("<img alt=\"\" src=\"");
                let _ = escape(&mut img, &ctx.res.root_relative(id));
                let _ = write!(img, "\" style=\"width:{w:.2}pt;height:{h:.2}pt\">");
                (format!("<div class=\"img\" style=\"height:{h:.2}pt\">{img}</div>"), h)
            }
            Err(path) => {
                ctx.warn.push(RenderWarning::UnresolvedImage(path.into_owned()));
                (format!("<div class=\"img broken\" style=\"height:{BROKEN_IMG_H}pt\"><span class=\"error\">??</span></div>"), BROKEN_IMG_H)
            }
        };
        let notes = line_notes(&cap);
        let html = format!("<div class=\"figure\">{body}<div class=\"cap center\">{}</div></div>", lines_html(&cap));
        self.push(Block {
            pieces: vec![Piece { html, height: img_h + IMG_GAP + cap_h, notes }],
            space_before: LH / 2.0,
            space_after: LH / 2.0,
            id: Some(format!("i-{index}")),
            ..Block::new(Kind::Whole)
        });
    }

    fn formula(&mut self, formula: Formula, ctx: &mut Ctx) {
        let (index, name) = match formula.named {
            Named::WithName { name, index } => (index, Some(name)),
            Named::Anonymous { index } => (index, None),
        };
        let mut name_lines = Vec::new();
        if let Some(name) = name {
            let mut il = Inline::new(Base::new(SIZE));
            il.spans(name, ctx);
            name_lines = il.lines(Layout { width: TEXT_W, first: INDENT, rest: 0.0, align: Align::Justify });
        }
        let src = formula.content.join("\n");
        let math = match latex_to_mathml(&src, DisplayStyle::Block) {
            Ok(m) => m,
            Err(e) => {
                ctx.warn.push(RenderWarning::InvalidFormula(e.to_string()));
                "<span class=\"error\">??</span>".to_string()
            }
        };
        let h = approx_formula_height(&src);
        let html = format!(
            "{}<div class=\"formula\" style=\"height:{h:.2}pt\"><div class=\"m\">{math}</div><span class=\"fnum\">{}</span></div>",
            lines_html(&name_lines),
            gost_caption(CaptionKind::Formula, index, false),
        );
        self.push(Block {
            pieces: vec![Piece { html, height: name_lines.len() as f32 * LH + h, notes: line_notes(&name_lines) }],
            space_before: LH,
            space_after: LH,
            id: Some(format!("f-{index}")),
            ..Block::new(Kind::Whole)
        });
    }

    fn code(&mut self, code: Code, ctx: &mut Ctx) {
        let index = match &code.named {
            Named::WithName { index, .. } | Named::Anonymous { index } => *index,
        };
        let lay = Layout { width: TEXT_W, first: 0.0, rest: 0.0, align: Align::Left };
        let cap = self.caption(CaptionKind::Code, code.named, lay, ctx);
        let cw = width("0", Font::MONO, SMALL);
        let max_chars = (((TEXT_W - 2.0 * CODE_PAD_X - 2.0 * BORDER) / cw).floor() as usize).max(1);
        let mut out = Vec::new();
        for line in code.content {
            let chars: Vec<char> = line.replace('\t', "    ").chars().collect();
            let mut piece = Piece { html: String::new(), height: 0.0, notes: vec![] };
            for chunk in chars.chunks(max_chars) {
                piece.html.push_str("<span class=\"l\">");
                let _ = escape(&mut piece.html, &chunk.iter().collect::<String>());
                piece.html.push_str("</span>");
                piece.height += SMALL_LH;
            }
            if chars.is_empty() {
                piece.html.push_str("<span class=\"l\"></span>");
                piece.height = SMALL_LH;
            }
            out.push(piece);
        }
        if out.is_empty() {
            out.push(Piece { html: "<span class=\"l\"></span>".into(), height: SMALL_LH, notes: vec![] });
        }
        out[0].notes = line_notes(&cap);
        self.push(Block {
            pieces: out,
            space_before: LH / 2.0,
            space_after: LH / 2.0,
            min_first: 2,
            min_last: 2,
            id: Some(format!("c-{index}")),
            ..Block::new(Kind::Code {
                caption: format!("<div class=\"cap\">{}</div>", lines_html(&cap)),
                caption_h: cap.len() as f32 * LH,
            })
        });
    }

    fn table(&mut self, table: Table, ctx: &mut Ctx) {
        let cols = table.aligments.len();
        if cols == 0 {
            return;
        }
        let index = match &table.named {
            Named::WithName { index, .. } | Named::Anonymous { index } => *index,
        };
        let lay = Layout { width: TEXT_W, first: 0.0, rest: 0.0, align: Align::Left };
        let cap = self.caption(CaptionKind::Table, table.named, lay, ctx);

        let head = cells(merge_cells(table.headers, cols), ctx);
        let body = cells(merge_cells(table.cells, cols), ctx);
        let widths = column_widths(&head.iter().chain(&body).collect::<Vec<_>>(), cols);
        let head = table_rows(head, &widths, &table.aligments, "th");
        let body = table_rows(body, &widths, &table.aligments, "td");

        let mut open = String::from("<table><colgroup>");
        for w in &widths {
            let _ = write!(open, "<col style=\"width:{w:.3}pt\">");
        }
        open.push_str("</colgroup>");
        let head_h: f32 = head.iter().map(|r| r.height).sum();
        let mut head_html = String::new();
        let mut head_notes = line_notes(&cap);
        if !head.is_empty() {
            head_html.push_str("<thead>");
            for r in head {
                head_html.push_str(&r.html);
                head_notes.extend(r.notes);
            }
            head_html.push_str("</thead>");
        }

        let mut groups: Vec<Piece> = Vec::new();
        let mut cur = Piece { html: String::new(), height: 0.0, notes: vec![] };
        let mut reach = 0;
        for (r, row) in body.into_iter().enumerate() {
            reach = reach.max(row.reach);
            cur.html.push_str(&row.html);
            cur.height += row.height;
            cur.notes.extend(row.notes);
            if reach <= r + 1 {
                groups.push(std::mem::replace(&mut cur, Piece { html: String::new(), height: 0.0, notes: vec![] }));
            }
        }
        if !cur.html.is_empty() {
            groups.push(cur);
        }
        if groups.is_empty() {
            groups.push(Piece { html: String::new(), height: 0.0, notes: vec![] });
        }
        groups[0].notes.splice(0..0, head_notes);

        let cont = format!("<div class=\"cap\"><span class=\"l\">Продолжение таблицы {index}</span></div>");
        self.push(Block {
            pieces: groups,
            space_before: LH / 2.0,
            space_after: LH / 2.0,
            min_first: 1,
            min_last: 1,
            id: Some(format!("t-{index}")),
            ..Block::new(Kind::Table {
                caption: format!("<div class=\"cap\">{}</div>", lines_html(&cap)),
                caption_h: cap.len() as f32 * LH,
                cont,
                cont_h: LH,
                open,
                head: head_html,
                head_h,
            })
        });
    }
}

pub(crate) fn build(docs: Vec<Doc>, res: &Resources, warn: &mut Vec<RenderWarning>) -> Flow {
    let file_docs: HashMap<usize, usize> = docs.iter().enumerate().map(|(k, d)| (d.file.idx, k + 1)).collect();
    let mut flow = Flow::default();
    let mut last_note = 0;
    for (k, doc) in docs.into_iter().enumerate() {
        let mut ctx = Ctx { res, docs: &file_docs, notes: Notes { local: HashMap::new(), last: last_note }, warn: &mut *warn };
        flow.doc(k + 1, doc, &mut ctx);
        last_note = ctx.notes.last;
    }
    flow
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::Resources;

    fn flow_of(src: &str) -> (Flow, Vec<RenderWarning>) {
        let res = Resources::default();
        let rush = crate::parse::rush::Rush::new(src, &res);
        let docs = vec![Doc { file: Id::new(0), name: "Документ", rush }];
        let mut warn = Vec::new();
        (build(docs, &res, &mut warn), warn)
    }

    #[test]
    fn document_title_and_toc() {
        let (flow, _) = flow_of("\\++ Раздел\n\nтекст\n\n\\+++ Подраздел\n\n\\++++ Пункт");
        let toc: Vec<(u8, &str)> = flow.toc.iter().map(|e| (e.level, e.text.as_str())).collect();
        assert_eq!(toc, vec![(1, "Документ"), (2, "1 Раздел"), (3, "1.1 Подраздел")]);
        assert!(flow.blocks[0].break_before);
        assert!(flow.blocks[0].pieces[0].html.contains("ДОКУМЕНТ"));
    }

    #[test]
    fn comment_is_italic_paragraph_with_prefix() {
        let (flow, _) = flow_of("\\\"\nтекст\n\\\"");
        let b = &flow.blocks[1];
        assert_eq!(b.kind, Kind::Text { open: "<p class=\"comment\">".into(), close: "</p>" });
        assert!(b.pieces[0].html.contains("Комментарий: текст"), "{}", b.pieces[0].html);
    }

    #[test]
    fn notes_renumbered_and_attached_to_lines() {
        let (flow, _) = flow_of("а\\*5 б\n\n\\**5 сноска\n\\**0 без ссылки");
        let refs: Vec<&Vec<u16>> = flow.blocks.iter().flat_map(|b| b.pieces.iter().map(|p| &p.notes)).filter(|n| !n.is_empty()).collect();
        assert_eq!(refs, vec![&vec![1], &vec![2]]);
        assert!(flow.notes[0].html.contains("сноска"));
        assert!(matches!(flow.blocks.last().unwrap().kind, Kind::Anchor));
    }

    #[test]
    fn table_groups_keep_rowspan_together() {
        let (flow, _) = flow_of("\\№ Т\n/ a / b /\n\\№-\n/ 1 / 2 /\n/ \\= / 3 /\n/ 4 / 5 /\n\\№");
        let t = &flow.blocks[1];
        assert_eq!(t.pieces.len(), 2);
        assert!(t.pieces[0].html.contains("rowspan=\"2\""));
        let Kind::Table { cont, head, .. } = &t.kind else { panic!() };
        assert!(cont.contains("Продолжение таблицы 1"));
        assert!(head.contains("<thead>"));
    }

    #[test]
    fn long_code_lines_wrap() {
        let long = "x".repeat(200);
        let (flow, _) = flow_of(&format!("\\// Л\n{long}\n\\//"));
        let pieces = &flow.blocks[1].pieces;
        assert_eq!(pieces.len(), 1, "строка исходника неделима");
        assert!(pieces[0].height >= 2.0 * SMALL_LH);
        assert!(pieces[0].html.matches("class=\"l\"").count() >= 2);
    }
}
