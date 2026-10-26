use super::inline::{Inline, Layout};
use super::metrics::Metrics;
use super::{Block, Item, Kind, LineBox, Piece, PlacedRun};
use crate::config::DocOut;
use crate::render::ir::{self, Anchor, Caption, CaptionKind, Cell, Link, ParaKind, Run, RunText};
use crate::render::warning::{Origin, RenderWarning, RenderWarningKind};
use crate::resources::Resources;
use crate::style::{Align, PageGeometry, Role, Style};
use std::collections::HashMap;

pub(crate) struct Warn<'w> {
    pub(crate) out: &'w mut Vec<RenderWarning>,
    pub(crate) backend: DocOut,
}

impl Warn<'_> {
    fn content(&mut self, file: &str, kind: RenderWarningKind) {
        self.out.push(RenderWarning::content(file, kind));
    }

    pub(crate) fn too_tall(&mut self, file: Option<String>) {
        self.out.push(RenderWarning { origin: Origin::Backend(self.backend), file, kind: RenderWarningKind::BlockTallerThanPage });
    }
}

pub(crate) struct Ctx<'c> {
    pub(crate) style: &'c Style,
    pub(crate) geometry: PageGeometry,
    pub(crate) metrics: Metrics,
    pub(crate) res: &'c Resources,
    /// file index -> document number
    docs: HashMap<usize, usize>,
    /// text area width
    pub(crate) w: f32,
}

impl<'c> Ctx<'c> {
    pub(crate) fn new(style: &'c Style, geometry: PageGeometry, res: &'c Resources, project: &ir::Project) -> Self {
        let docs = project.docs.iter().enumerate().map(|(k, d)| (d.file.idx, k + 1)).collect();
        Self { style, geometry, metrics: Metrics::new(&style.fonts), res, docs, w: geometry.text_w() }
    }

    pub(crate) fn inline(&self, role: &Role) -> Inline<'_> {
        Inline::new(&self.metrics, role, self.style.sup_scale)
    }

    /// One line without wrapping
    pub(crate) fn single(&self, text: &str, role: &Role) -> LineBox {
        let mut il = self.inline(role);
        il.text(text);
        il.lines(Layout { width: f32::MAX / 4.0, first: 0.0, rest: 0.0, align: Align::Left })
            .pop()
            .unwrap_or_else(|| LineBox::empty(role.lh, role.size))
    }

    fn runs(&self, il: &mut Inline, runs: &[Run], file: &str, warn: &mut Warn) {
        il.runs(runs, &mut |link: &Link| match link {
            Link::Url(u) => Some(u.clone()),
            Link::Anchor(a) => Some(format!("#{}", a.id())),
            Link::Doc(id) => match self.docs.get(&id.idx) {
                Some(k) => Some(format!("#{}", Anchor::Doc(*k).id())),
                None => {
                    let path = self.res.files.local_path(*id).to_string_lossy().into_owned();
                    warn.content(file, RenderWarningKind::FileNotInContent(path));
                    None
                }
            },
        });
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct NoteBody {
    pub(crate) items: Vec<Item>,
    pub(crate) height: f32,
}

#[derive(Default)]
pub(crate) struct Flow {
    pub(crate) blocks: Vec<Block>,
    /// note texts indexed by global note number - 1
    pub(crate) notes: Vec<NoteBody>,
}

/// Lines stacked vertically: boxes, height, notes
fn stack(lines: Vec<LineBox>) -> (Vec<Item>, f32, Vec<u16>) {
    let mut y = 0.0;
    let mut notes = Vec::new();
    let mut items = Vec::with_capacity(lines.len());
    for line in lines {
        notes.extend(line.notes.iter().copied());
        let lh = line.lh;
        items.push(Item::Line { y, line });
        y += lh;
    }
    (items, y, notes)
}

fn pieces(lines: Vec<LineBox>) -> Vec<Piece> {
    lines.into_iter().map(|line| Piece {
        height: line.lh,
        notes: line.notes.clone(),
        items: vec![Item::Line { y: 0.0, line }],
    }).collect()
}

fn spaced(role: &Role, kind: Kind) -> Block {
    Block {
        space_before: role.space_before,
        space_after: role.space_after,
        keep_with_next: role.keep_with_next,
        break_before: role.break_before,
        min_first: role.min_first,
        min_last: role.min_last,
        ..Block::new(kind)
    }
}

fn approx_formula_height(src: &str, size: f32, lh: f32) -> f32 {
    let count = |pats: &[&str]| pats.iter().map(|p| src.matches(p).count()).sum::<usize>();
    let rows = 1 + count(&["\\\\"]);
    let fracs = count(&["\\frac", "\\dfrac", "\\tfrac", "\\binom", "\\over"]).min(3);
    let big = count(&["\\sum", "\\prod", "\\int", "\\oint", "\\lim", "\\bigcup", "\\bigcap"]) > 0;
    let h = rows as f32 * size * 1.6 + fracs as f32 * size + if big { size * 0.8 } else { 0.0 };
    h.max(lh)
}

struct LCell<'m> {
    col: usize,
    w: usize,
    h: usize,
    text: Option<Inline<'m>>,
}

struct Row {
    items: Vec<Item>,
    height: f32,
    notes: Vec<u16>,
    reach: usize,
}

/// Owner of each grid cell: cells of one merged area are not separated by a line
fn owners(rows: &[Vec<Cell>], cols: usize) -> Vec<Vec<usize>> {
    let mut grid = vec![vec![usize::MAX; cols]; rows.len()];
    let mut next = 0;
    for (r, row) in rows.iter().enumerate() {
        for cell in row {
            match *cell {
                Cell::Text { col, w, h, .. } => {
                    for line in grid.iter_mut().skip(r).take(h) {
                        for c in line.iter_mut().skip(col).take(w) {
                            *c = next;
                        }
                    }
                    next += 1;
                }
                Cell::Empty { col, join_top, join_left } => {
                    grid[r][col] = if join_top && r > 0 {
                        grid[r - 1][col]
                    } else if join_left && col > 0 {
                        grid[r][col - 1]
                    } else {
                        next += 1;
                        next - 1
                    };
                }
            }
        }
    }
    grid
}

struct Table<'a, 'c> {
    ctx: &'a Ctx<'c>,
    widths: Vec<f32>,
    xs: Vec<f32>,
    extra_w: f32,
    extra_h: f32,
}

impl<'c> Table<'_, 'c> {
    fn cells<'m>(ctx: &'m Ctx, rows: &[Vec<Cell>], role: &Role, file: &str, warn: &mut Warn) -> Vec<Vec<LCell<'m>>> {
        rows.iter().map(|row| row.iter().map(|cell| match cell {
            Cell::Text { col, w, h, runs } => {
                let mut il = ctx.inline(role);
                ctx.runs(&mut il, runs, file, warn);
                LCell { col: *col, w: *w, h: *h, text: Some(il) }
            }
            Cell::Empty { col, .. } => LCell { col: *col, w: 1, h: 1, text: None },
        }).collect()).collect()
    }

    fn column_widths(rows: &[&Vec<LCell>], cols: usize, extra_w: f32, base: f32, total: f32) -> Vec<f32> {
        let mut nat = vec![extra_w + base; cols];
        let mut min = nat.clone();
        for cell in rows.iter().flat_map(|r| r.iter()) {
            if let (1, Some(il)) = (cell.w, &cell.text) {
                nat[cell.col] = nat[cell.col].max(il.natural_width() + extra_w);
                min[cell.col] = min[cell.col].max(il.min_width() + extra_w);
            }
        }
        let (sn, sm): (f32, f32) = (nat.iter().sum(), min.iter().sum());
        if sn <= total {
            nat.iter().map(|n| n + (total - sn) * n / sn).collect()
        } else if sm <= total {
            let flex = sn - sm;
            nat.iter().zip(&min).map(|(n, m)| m + (total - sm) * (n - m) / flex).collect()
        } else {
            min.iter().map(|m| total * m / sm).collect()
        }
    }

    fn rows(&self, rows: Vec<Vec<LCell>>, owner: &[Vec<usize>], role: &Role, aligns: Option<&[Align]>) -> Vec<Row> {
        let st = self.ctx.style;
        let b = st.cell_frame.border;
        let cols = self.widths.len();
        let n = rows.len();
        let laid: Vec<Vec<(LCell, Vec<LineBox>)>> = rows.into_iter().map(|row| row.into_iter().map(|cell| {
            let lines = match &cell.text {
                Some(il) => {
                    let w: f32 = self.widths[cell.col..cell.col + cell.w].iter().sum();
                    let align = aligns.map_or(role.align, |a| a[cell.col]);
                    il.lines(Layout { width: w - self.extra_w, first: 0.0, rest: 0.0, align })
                }
                None => Vec::new(),
            };
            (cell, lines)
        }).collect()).collect();

        let need = |lines: &[LineBox]| lines.len().max(1) as f32 * role.lh + self.extra_h;
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
            let mut items = Vec::new();
            let mut notes = Vec::new();
            let mut reach = r + 1;
            for c in 0..cols {
                if r == 0 || owner[r - 1][c] != owner[r][c] {
                    items.push(Item::Rule { x: self.xs[c] - b / 2.0, y: -b / 2.0, w: self.widths[c] + b, h: b });
                }
            }
            for c in 0..=cols {
                if c == 0 || c == cols || owner[r][c - 1] != owner[r][c] {
                    items.push(Item::Rule { x: self.xs[c] - b / 2.0, y: -b / 2.0, w: b, h: height + b });
                }
            }
            for (cell, lines) in row {
                if cell.text.is_none() {
                    continue;
                }
                let (lines, _, cell_notes) = stack(lines);
                let (dx, dy) = (self.xs[cell.col] + st.cell_frame.pad_x + b / 2.0, st.cell_frame.pad_y + b / 2.0);
                for mut item in lines {
                    item.shift(dx, dy);
                    items.push(item);
                }
                notes.extend(cell_notes);
                reach = reach.max(r + cell.h);
            }
            Row { items, height, notes, reach }
        }).collect()
    }
}

impl Flow {
    fn set_note(&mut self, g: u16, body: NoteBody) {
        let i = g as usize - 1;
        if self.notes.len() <= i {
            self.notes.resize_with(i + 1, NoteBody::default);
        }
        self.notes[i] = body;
    }

    fn note_body(&mut self, def: &ir::NoteDef, ctx: &Ctx, file: &str, warn: &mut Warn) {
        let role = &ctx.style.notes.role;
        let mut il = ctx.inline(role);
        il.marker(def.g.to_string());
        il.space();
        ctx.runs(&mut il, &def.runs, file, warn);
        let (lines, height, _) = stack(il.lines(Layout::of(role, ctx.w)));
        let mut items = vec![Item::Anchor { y: 0.0, id: Anchor::Note(def.g).id() }];
        items.extend(lines);
        self.set_note(def.g, NoteBody { items, height });
    }

    fn push(&mut self, block: Block, file: &str) {
        self.blocks.push(Block { file: Some(file.to_string()), ..block });
    }

    fn text_block(&mut self, role: &Role, lines: Vec<LineBox>, file: &str) {
        if lines.is_empty() {
            return;
        }
        self.push(Block { pieces: pieces(lines), ..spaced(role, Kind::Text) }, file);
    }

    fn caption(&self, caption: &Caption, ctx: &Ctx, file: &str, warn: &mut Warn) -> (Vec<Item>, f32, Vec<u16>) {
        let caps = &ctx.style.captions;
        let role = match caption.kind {
            CaptionKind::Table => &caps.table.role,
            CaptionKind::Image => &caps.image.role,
            CaptionKind::Code => &caps.code.role,
        };
        let mut il = ctx.inline(role);
        ctx.runs(&mut il, &caption.runs(), file, warn);
        stack(il.lines(Layout::of(role, ctx.w)))
    }

    fn doc(&mut self, doc: &ir::Doc, ctx: &Ctx, warn: &mut Warn) {
        let st = ctx.style;
        let file = &ctx.res.files.local_path(doc.file).to_string_lossy().into_owned();
        let mut il = ctx.inline(&st.doc_title);
        il.text(&doc.title);
        let mut lines = il.lines(Layout::of(&st.doc_title, ctx.w));
        if lines.is_empty() {
            lines.push(LineBox::empty(st.doc_title.lh, st.doc_title.size));
        }
        self.push(Block { pieces: pieces(lines), id: Some(doc.anchor.id()), ..spaced(&st.doc_title, Kind::Text) }, file);

        for block in &doc.blocks {
            match block {
                ir::Block::Para { kind, runs } => {
                    let role = match kind {
                        ParaKind::Body => &st.body,
                        ParaKind::Comment => &st.comment,
                    };
                    let mut il = ctx.inline(role);
                    ctx.runs(&mut il, runs, file, warn);
                    self.text_block(role, il.lines(Layout::of(role, ctx.w)), file);
                }
                ir::Block::Header { level, number, runs, anchor } => self.header(*level, number.as_deref(), runs, anchor, ctx, file, warn),
                ir::Block::List(items) => {
                    let role = &st.list;
                    let mut lines = Vec::new();
                    for item in items {
                        let mut il = ctx.inline(role);
                        il.text(&item.marker);
                        ctx.runs(&mut il, &item.runs, file, warn);
                        let lay = Layout { first: role.first * (item.level + 1) as f32, ..Layout::of(role, ctx.w) };
                        lines.extend(il.lines(lay));
                    }
                    self.text_block(role, lines, file);
                }
                ir::Block::Figure { caption, image, scale, anchor } => self.figure(caption, *image, *scale, anchor, ctx, file, warn),
                ir::Block::Formula { name, mathml, src, number, anchor } => {
                    self.formula(name.as_deref(), mathml.as_deref(), src, number, anchor, ctx, file, warn)
                }
                ir::Block::Code { caption, lines, anchor } => self.code(caption, lines, anchor, ctx, file, warn),
                ir::Block::Table { caption, cont, aligns, head, body, anchor } => {
                    self.table(caption, cont, aligns, head, body, anchor, ctx, file, warn)
                }
                ir::Block::Notes(defs) => for def in defs {
                    self.note_body(def, ctx, file, warn);
                    if def.anchored {
                        let anchor = Piece { items: Vec::new(), height: 0.0, notes: vec![def.g] };
                        self.push(Block { pieces: vec![anchor], ..Block::new(Kind::Anchor) }, file);
                    }
                },
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn header(&mut self, level: u8, number: Option<&str>, runs: &[Run], anchor: &Anchor, ctx: &Ctx, file: &str, warn: &mut Warn) {
        let role = ctx.style.header(level);
        let hang = match number {
            Some(n) if role.align != Align::Center => {
                ctx.metrics.width(n, role.font, role.size) + ctx.metrics.width(" ", role.font, role.size)
            }
            _ => 0.0,
        };
        let mut il = ctx.inline(role);
        if let Some(n) = number {
            il.text(n);
            il.space();
        }
        ctx.runs(&mut il, runs, file, warn);
        let lay = Layout { rest: role.rest + hang, ..Layout::of(role, ctx.w) };
        let mut lines = il.lines(lay);
        if lines.is_empty() {
            lines.push(LineBox::empty(role.lh, role.size));
        }
        self.push(Block { pieces: pieces(lines), id: Some(anchor.id()), ..spaced(role, Kind::Text) }, file);
    }

    #[allow(clippy::too_many_arguments)]
    fn figure(&mut self, caption: &Caption, image: Option<crate::resources::Id<crate::resources::ImageId>>, scale: u16, anchor: &Anchor, ctx: &Ctx, file: &str, warn: &mut Warn) {
        let st = ctx.style;
        let (cap, cap_h, notes) = self.caption(caption, ctx, file, warn);
        let max_h = ctx.geometry.text_h() - ctx.geometry.slack - cap_h - st.images.gap;
        let scale = scale as f32 / 100.0;
        let mut items = Vec::new();
        let img_h = match image {
            Some(id) => {
                let px = st.images.px;
                let (w, h) = match *ctx.res.images.data(id) {
                    Some((w, h)) => (w as f32 * px * scale, h as f32 * px * scale),
                    None => {
                        let local = ctx.res.images.local_path(id).to_string_lossy().into_owned();
                        warn.content(file, RenderWarningKind::ImageSizeUnknown(local));
                        (ctx.w / 2.0, ctx.w * 3.0 / 8.0)
                    }
                };
                let fit = (ctx.w / w.max(1.0)).min(max_h / h.max(1.0)).min(1.0);
                let (w, h) = (w * fit, h * fit);
                items.push(Item::Image { x: (ctx.w - w) / 2.0, y: 0.0, w, h, src: ctx.res.root_relative(id) });
                h
            }
            None => {
                let h = st.images.broken_h;
                let role = &st.captions.image.role;
                let mut il = ctx.inline(role);
                il.runs(&[Run::plain(RunText::Error)], &mut |_| None);
                for line in il.lines(Layout::of(role, ctx.w)) {
                    items.push(Item::Line { y: (h - line.lh) / 2.0, line });
                }
                h
            }
        };
        let top = img_h + st.images.gap;
        for mut item in cap {
            item.shift(0.0, top);
            items.push(item);
        }
        self.push(Block {
            pieces: vec![Piece { items, height: top + cap_h, notes }],
            id: Some(anchor.id()),
            ..spaced(&st.figure, Kind::Whole)
        }, file);
    }

    #[allow(clippy::too_many_arguments)]
    fn formula(&mut self, name: Option<&[Run]>, mathml: Option<&str>, src: &str, number: &str, anchor: &Anchor, ctx: &Ctx, file: &str, warn: &mut Warn) {
        let st = ctx.style;
        let (mut items, name_h, notes) = match name {
            Some(runs) => {
                let mut il = ctx.inline(&st.body);
                ctx.runs(&mut il, runs, file, warn);
                stack(il.lines(Layout::of(&st.body, ctx.w)))
            }
            None => (Vec::new(), 0.0, Vec::new()),
        };
        let h = approx_formula_height(src, st.body.size, st.body.lh);
        match mathml {
            Some(m) => items.push(Item::Math { x: 0.0, y: name_h, w: ctx.w, h, size: st.body.size, mathml: m.to_string() }),
            None => {
                let mut il = ctx.inline(&st.body);
                il.runs(&[Run::plain(RunText::Error)], &mut |_| None);
                for line in il.lines(Layout { width: ctx.w, first: 0.0, rest: 0.0, align: Align::Center }) {
                    items.push(Item::Line { y: name_h + (h - line.lh) / 2.0, line });
                }
            }
        }
        let mut num = ctx.single(number, &st.body);
        num.x = ctx.w - num.natural;
        items.push(Item::Line { y: name_h + (h - num.lh) / 2.0, line: num });
        self.push(Block {
            pieces: vec![Piece { items, height: name_h + h, notes }],
            id: Some(anchor.id()),
            ..spaced(&st.formula, Kind::Whole)
        }, file);
    }

    #[allow(clippy::too_many_arguments)]
    fn code(&mut self, caption: &Caption, lines: &[String], anchor: &Anchor, ctx: &Ctx, file: &str, warn: &mut Warn) {
        let st = ctx.style;
        let role = &st.code;
        let frame = &st.code_frame;
        let (cap, cap_h, cap_notes) = self.caption(caption, ctx, file, warn);
        let cw = ctx.metrics.width("0", role.font, role.size);
        let max_chars = (((ctx.w - 2.0 * frame.pad_x - 2.0 * frame.border) / cw).floor() as usize).max(1);
        let line_box = |text: String| {
            let natural = ctx.metrics.width(&text, role.font, role.size);
            let runs = if text.is_empty() {
                Vec::new()
            } else {
                vec![PlacedRun { x: 0.0, text, font: role.font, size: role.size, sup: false, under: false, strike: false, color: None, link: None, error: false }]
            };
            LineBox { x: 0.0, size: role.size, lh: role.lh, word_spacing: 0.0, natural, runs, notes: Vec::new() }
        };
        let mut out = Vec::new();
        for line in lines {
            let chars: Vec<char> = line.replace('\t', "    ").chars().collect();
            let boxes: Vec<LineBox> = if chars.is_empty() {
                vec![line_box(String::new())]
            } else {
                chars.chunks(max_chars).map(|c| line_box(c.iter().collect())).collect()
            };
            let (items, height, _) = stack(boxes);
            out.push(Piece { items, height, notes: Vec::new() });
        }
        if out.is_empty() {
            let (items, height, _) = stack(vec![line_box(String::new())]);
            out.push(Piece { items, height, notes: Vec::new() });
        }
        out[0].notes = cap_notes;
        self.push(Block {
            pieces: out,
            id: Some(anchor.id()),
            ..spaced(role, Kind::Code { caption: cap, caption_h: cap_h, width: ctx.w, border: frame.border, pad_x: frame.pad_x, pad_y: frame.pad_y })
        }, file);
    }

    #[allow(clippy::too_many_arguments)]
    fn table(&mut self, caption: &Caption, cont: &str, aligns: &[Align], head: &[Vec<Cell>], body: &[Vec<Cell>], anchor: &Anchor, ctx: &Ctx, file: &str, warn: &mut Warn) {
        let st = ctx.style;
        let cols = aligns.len();
        let (cap, cap_h, mut head_notes) = self.caption(caption, ctx, file, warn);
        let frame = &st.cell_frame;
        let extra_w = 2.0 * frame.pad_x + frame.border;
        let extra_h = 2.0 * frame.pad_y + frame.border;

        let head_cells = Table::cells(ctx, head, &st.head_cell, file, warn);
        let body_cells = Table::cells(ctx, body, &st.cell, file, warn);
        let all: Vec<&Vec<LCell>> = head_cells.iter().chain(&body_cells).collect();
        let widths = Table::column_widths(&all, cols, extra_w, st.cell.size, ctx.w);
        let mut xs = vec![0.0];
        for w in &widths {
            xs.push(xs.last().unwrap() + w);
        }
        let t = Table { ctx, widths, xs, extra_w, extra_h };
        let head_rows = t.rows(head_cells, &owners(head, cols), &st.head_cell, None);
        let body_rows = t.rows(body_cells, &owners(body, cols), &st.cell, Some(aligns));

        let mut head_items = Vec::new();
        let mut head_h = 0.0;
        for r in head_rows {
            for mut item in r.items {
                item.shift(0.0, head_h);
                head_items.push(item);
            }
            head_h += r.height;
            head_notes.extend(r.notes);
        }

        let empty = || Piece { items: Vec::new(), height: 0.0, notes: vec![] };
        let mut groups: Vec<Piece> = Vec::new();
        let mut cur = empty();
        let mut reach = 0;
        for (r, row) in body_rows.into_iter().enumerate() {
            reach = reach.max(row.reach);
            for mut item in row.items {
                item.shift(0.0, cur.height);
                cur.items.push(item);
            }
            cur.height += row.height;
            cur.notes.extend(row.notes);
            if reach <= r + 1 {
                groups.push(std::mem::replace(&mut cur, empty()));
            }
        }
        if !cur.items.is_empty() {
            groups.push(cur);
        }
        if groups.is_empty() {
            groups.push(empty());
        }
        groups[0].notes.splice(0..0, head_notes);

        let cont_role = &st.captions.table.role;
        let mut il = ctx.inline(cont_role);
        il.text(cont);
        let (cont_items, cont_h, _) = stack(il.lines(Layout::of(cont_role, ctx.w)));
        self.push(Block {
            pieces: groups,
            id: Some(anchor.id()),
            ..spaced(&st.table, Kind::Table {
                caption: cap,
                caption_h: cap_h,
                cont: cont_items,
                cont_h,
                head: head_items,
                head_h,
                width: ctx.w,
                border: frame.border,
            })
        }, file);
    }
}

pub(crate) fn build(project: &ir::Project, ctx: &Ctx, warn: &mut Warn) -> Flow {
    let mut flow = Flow::default();
    for doc in &project.docs {
        flow.doc(doc, ctx, warn);
    }
    flow
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::ir::tests::project_of;

    fn flow_of(src: &str) -> (Flow, Vec<RenderWarning>) {
        let (proj, project, mut out) = project_of(src);
        let res = &proj.resources;
        let style = Style::of(crate::config::DocStyle::Gost7_32);
        let ctx = Ctx::new(&style, style.page.unwrap(), res, &project);
        let mut warn = Warn { out: &mut out, backend: DocOut::HtmlPaged };
        let flow = build(&project, &ctx, &mut warn);
        (flow, out)
    }

    fn texts(piece: &Piece) -> Vec<String> {
        piece.items.iter().filter_map(|i| match i {
            Item::Line { line, .. } => Some(line.runs.iter().map(|r| r.text.as_str()).collect()),
            _ => None,
        }).collect()
    }

    #[test]
    fn document_title_breaks_page() {
        let (flow, _) = flow_of("текст");
        assert!(flow.blocks[0].break_before);
        assert_eq!(texts(&flow.blocks[0].pieces[0]), vec!["ДОКУМЕНТ"]);
        assert_eq!(flow.blocks[0].id.as_deref(), Some("d-1"));
    }

    #[test]
    fn notes_attached_to_lines() {
        let (flow, _) = flow_of("а\\*5 б\n\n\\**5 сноска\n\\**0 без ссылки");
        let refs: Vec<&Vec<u16>> = flow.blocks.iter().flat_map(|b| b.pieces.iter().map(|p| &p.notes)).filter(|n| !n.is_empty()).collect();
        assert_eq!(refs, vec![&vec![1], &vec![2]]);
        assert!(flow.notes[0].items.iter().any(|i| matches!(i, Item::Line { line, .. } if line.runs.iter().any(|r| r.text.contains("сноска")))));
        assert!(matches!(flow.blocks.last().unwrap().kind, Kind::Anchor));
    }

    #[test]
    fn table_groups_keep_rowspan_together() {
        let (flow, _) = flow_of("\\№ Т\n/ a / b /\n\\№-\n/ 1 / 2 /\n/ \\= / 3 /\n/ 4 / 5 /\n\\№");
        let t = &flow.blocks[1];
        assert_eq!(t.pieces.len(), 2);
        let Kind::Table { cont, head, .. } = &t.kind else { panic!() };
        let cont: Vec<String> = cont.iter().filter_map(|i| match i {
            Item::Line { line, .. } => Some(line.runs.iter().map(|r| r.text.as_str()).collect()),
            _ => None,
        }).collect();
        assert_eq!(cont, vec!["Продолжение таблицы 1"]);
        assert!(!head.is_empty());
        let rules = t.pieces[0].items.iter().filter(|i| matches!(i, Item::Rule { w, .. } if *w > 1.0)).count();
        assert_eq!(rules, 3, "верх двух клеток первой строки и верх второй клетки второй");
    }

    #[test]
    fn long_code_lines_wrap() {
        let long = "x".repeat(200);
        let (flow, _) = flow_of(&format!("\\// Л\n{long}\n\\//"));
        let pieces = &flow.blocks[1].pieces;
        assert_eq!(pieces.len(), 1, "строка исходника неделима");
        assert!(pieces[0].height >= 2.0 * 14.0);
        assert!(texts(&pieces[0]).len() >= 2);
    }
}
