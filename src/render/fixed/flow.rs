use super::inline::{Inline, Layout};
use super::metrics::Metrics;
use super::{shifted, Block, Item, Kind, LineBox, Piece, PlacedRun};
use crate::config::DocOut;
use crate::render::ir::{self, Anchor, Caption, Cell, Link, ParaKind, Run, RunText};
use crate::render::warning::{Origin, RenderWarning, RenderWarningKind};
use crate::resources::Resources;
use crate::style::{Align, Grid, PageGeometry, Role, Style};
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
    docs: HashMap<usize, usize>,
    pub(crate) w: f32,
}

impl<'c> Ctx<'c> {
    pub(crate) fn new(style: &'c Style, geometry: PageGeometry, res: &'c Resources, project: &ir::Project) -> Self {
        let docs = project.docs.iter().enumerate().map(|(k, d)| (d.file.idx, k + 1)).collect();
        Self { style, geometry, metrics: Metrics::new(&style.fonts), res, docs, w: geometry.text_w() }
    }

    pub(crate) fn width(&self, role: &Role) -> f32 {
        (self.w - 2.0 * role.squeeze).max(self.w / 4.0)
    }

    pub(crate) fn inline(&self, role: &Role) -> Inline<'_> {
        Inline::new(&self.metrics, role, self.style.sup_scale).mono_bg(self.style.mono_bg).ext_link(self.style.ext_link)
    }

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
    pub(crate) notes: Vec<NoteBody>,
    w: f32,
}

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

fn filled((mut items, h, notes): (Vec<Item>, f32, Vec<u16>), role: &Role, w: f32) -> (Vec<Item>, f32, Vec<u16>) {
    if let Some(color) = role.bg_color {
        items.insert(0, Item::Fill { x: 0.0, y: 0.0, w, h, color });
    }
    (items, h, notes)
}

fn caption_box(lines: Vec<LineBox>, role: &Role, w: f32) -> (Vec<Item>, f32, Vec<u16>) {
    let (mut items, h, notes) = filled(stack(lines), role, w);
    if items.is_empty() {
        return (items, 0.0, notes);
    }
    for item in &mut items {
        item.shift(0.0, role.space_before);
    }
    (items, role.space_before + h + role.space_after, notes)
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
        space_before: role.space_before + role.marg_before,
        space_after: role.space_after + role.marg_after,
        indent: role.squeeze,
        pad_y: role.squeeze_y(),
        bg: role.bg_color,
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

    fn rows(&self, rows: Vec<Vec<LCell>>, owner: &[Vec<usize>], role: &Role, aligns: Option<&[Align]>, grid: Grid, top: bool) -> Vec<Row> {
        let st = self.ctx.style;
        let b = st.cell_frame.border;
        let frame = st.table_lines.frame;
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
            if let Some(color) = role.bg_color {
                items.push(Item::Fill { x: 0.0, y: 0.0, w: self.xs[cols], h: height, color });
            }
            let mut notes = Vec::new();
            let mut reach = r + 1;
            for c in 0..cols {
                if if r == 0 { top } else { grid.rows && owner[r - 1][c] != owner[r][c] } {
                    items.push(Item::Rule { x: self.xs[c] - b / 2.0, y: -b / 2.0, w: self.widths[c] + b, h: b });
                }
            }
            for c in 0..=cols {
                if if c == 0 || c == cols { frame } else { grid.cols && owner[r][c - 1] != owner[r][c] } {
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
        let (lines, height, _) = stack(il.lines(Layout::of(role, ctx.width(role))));
        let mut items = vec![Item::Anchor { y: 0.0, id: Anchor::Note(def.g).id() }];
        shifted(&lines, role.squeeze, 0.0, &mut items);
        self.set_note(def.g, NoteBody { items, height });
    }

    fn push(&mut self, block: Block, file: &str) {
        self.blocks.push(Block { file: Some(file.to_string()), width: self.w, ..block });
    }

    fn text_block(&mut self, role: &Role, lines: Vec<LineBox>, file: &str) {
        if lines.is_empty() {
            return;
        }
        self.push(Block { pieces: pieces(lines), ..spaced(role, Kind::Text) }, file);
    }


    fn caption(&self, caption: &Caption, w: f32, ctx: &Ctx, file: &str, warn: &mut Warn) -> (Vec<Item>, f32, Vec<u16>) {
        let role = &ctx.style.captions.of(caption.kind).role;
        let mut il = ctx.inline(role);
        ctx.runs(&mut il, &caption.runs(), file, warn);
        caption_box(il.lines(Layout::of(role, w)), role, w)
    }

    fn doc(&mut self, doc: &ir::Doc, ctx: &Ctx, warn: &mut Warn) {
        let st = ctx.style;
        let file = &ctx.res.files.local_path(doc.file).to_string_lossy().into_owned();
        let mut il = ctx.inline(&st.doc_title);
        il.text(&doc.title);
        let mut lines = il.lines(Layout::of(&st.doc_title, ctx.width(&st.doc_title)));
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
                    self.text_block(role, il.lines(Layout::of(role, ctx.width(role))), file);
                }
                ir::Block::Header { level, number, runs, anchor } => self.header(*level, number.as_deref(), runs, anchor, ctx, file, warn),
                ir::Block::List(items) => {
                    let role = &st.list;
                    let mut lines = Vec::new();
                    for item in items {
                        let mut il = ctx.inline(role);
                        il.text(&item.marker);
                        ctx.runs(&mut il, &item.runs, file, warn);
                        let first = role.first + st.list_step * item.level as f32;
                        let rest = if st.list_hang { first + ctx.metrics.width(&item.marker, role.font, role.size) } else { role.rest };
                        lines.extend(il.lines(Layout { first, rest, ..Layout::of(role, ctx.width(role)) }));
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
        let lay = Layout { rest: role.rest + hang, ..Layout::of(role, ctx.width(role)) };
        let mut lines = il.lines(lay);
        if lines.is_empty() {
            lines.push(LineBox::empty(role.lh, role.size));
        }
        self.push(Block { pieces: pieces(lines), id: Some(anchor.id()), ..spaced(role, Kind::Text) }, file);
    }

    #[allow(clippy::too_many_arguments)]
    fn figure(&mut self, caption: &Caption, image: Option<crate::resources::Id<crate::resources::ImageId>>, scale: u16, anchor: &Anchor, ctx: &Ctx, file: &str, warn: &mut Warn) {
        let st = ctx.style;
        let fw = ctx.width(&st.figure);
        let (cap, cap_h, notes) = self.caption(caption, fw, ctx, file, warn);
        let max_h = ctx.geometry.text_h() - ctx.geometry.slack - cap_h - st.images.gap - 2.0 * st.figure.squeeze_y();
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
                        (fw / 2.0, fw * 3.0 / 8.0)
                    }
                };
                let fit = (fw / w.max(1.0)).min(max_h / h.max(1.0)).min(1.0);
                let (w, h) = (w * fit, h * fit);
                items.push(Item::Image { x: (fw - w) / 2.0, y: 0.0, w, h, src: ctx.res.root_relative(id) });
                h
            }
            None => {
                let h = st.images.broken_h;
                let role = &st.captions.image.role;
                let mut il = ctx.inline(role);
                il.runs(&[Run::plain(RunText::Error)], &mut |_| None);
                for line in il.lines(Layout::of(role, fw)) {
                    items.push(Item::Line { y: (h - line.lh) / 2.0, line });
                }
                h
            }
        };
        let gap = st.images.gap;
        let (img_y, cap_y) = if st.captions.image.caption_after { (0.0, img_h + gap) } else { (cap_h + gap, 0.0) };
        for item in &mut items {
            item.shift(0.0, img_y);
        }
        for mut item in cap {
            item.shift(0.0, cap_y);
            items.push(item);
        }
        self.push(Block {
            pieces: vec![Piece { items, height: img_h + gap + cap_h, notes }],
            id: Some(anchor.id()),
            ..spaced(&st.figure, Kind::Whole)
        }, file);
    }

    #[allow(clippy::too_many_arguments)]
    fn formula(&mut self, name: Option<&[Run]>, mathml: Option<&str>, src: &str, number: &str, anchor: &Anchor, ctx: &Ctx, file: &str, warn: &mut Warn) {
        let st = ctx.style;
        let w = ctx.width(&st.formula);
        let (mut items, name_h, notes) = match name {
            Some(runs) => {
                let mut il = ctx.inline(&st.body);
                ctx.runs(&mut il, runs, file, warn);
                stack(il.lines(Layout::of(&st.body, w)))
            }
            None => (Vec::new(), 0.0, Vec::new()),
        };
        let h = approx_formula_height(src, st.body.size, st.body.lh);
        match mathml {
            Some(m) => items.push(Item::Math { x: 0.0, y: name_h, w, h, size: st.body.size, mathml: m.to_string() }),
            None => {
                let mut il = ctx.inline(&st.body);
                il.runs(&[Run::plain(RunText::Error)], &mut |_| None);
                for line in il.lines(Layout { width: w, first: 0.0, rest: 0.0, align: Align::Center }) {
                    items.push(Item::Line { y: name_h + (h - line.lh) / 2.0, line });
                }
            }
        }
        let mut num = ctx.single(number, &st.body);
        num.x = w - num.natural;
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
        let w = ctx.width(role);
        let (cap, cap_h, cap_notes) = self.caption(caption, w, ctx, file, warn);
        let cw = ctx.metrics.width("0", role.font, role.size);
        let (b, pad_x, pad_y) = if st.code_border { (frame.border, frame.pad_x, frame.pad_y) } else { (0.0, 0.0, 0.0) };
        let digits = if st.code_numbers { lines.len().max(1).to_string().len() } else { 0 };
        let num_r = b + pad_x + digits as f32 * cw;
        let (text_x, divider) = match (st.code_numbers, st.code_border) {
            (false, _) => (b + pad_x, None),
            (true, framed) => (num_r + 2.0 * frame.pad_x + b, framed.then_some(num_r + frame.pad_x)),
        };
        let max_chars = (((w - text_x - pad_x - b) / cw).floor() as usize).max(1);
        let line_box = |x: f32, text: String| {
            let natural = ctx.metrics.width(&text, role.font, role.size);
            let runs = if text.is_empty() {
                Vec::new()
            } else {
                vec![PlacedRun { x: 0.0, text, font: role.font, size: role.size, sup: false, under: false, strike: false, color: role.text_color, bg: None, link: None, error: false }]
            };
            LineBox { x, size: role.size, lh: role.lh, word_spacing: 0.0, natural, runs, notes: Vec::new() }
        };
        let mut out = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let chars: Vec<char> = line.replace('\t', "    ").chars().collect();
            let boxes: Vec<LineBox> = if chars.is_empty() {
                vec![line_box(text_x, String::new())]
            } else {
                chars.chunks(max_chars).map(|c| line_box(text_x, c.iter().collect())).collect()
            };
            let (mut items, height, _) = stack(boxes);
            if st.code_numbers {
                let mut num = line_box(0.0, (i + 1).to_string());
                num.x = num_r - num.natural;
                items.push(Item::Line { y: 0.0, line: num });
            }
            out.push(Piece { items, height, notes: Vec::new() });
        }
        if out.is_empty() {
            let (items, height, _) = stack(vec![line_box(text_x, String::new())]);
            out.push(Piece { items, height, notes: Vec::new() });
        }
        let caption_after = st.captions.code.caption_after;
        let at = if caption_after { out.len() - 1 } else { 0 };
        out[at].notes = cap_notes;
        self.push(Block {
            pieces: out,
            id: Some(anchor.id()),
            ..spaced(role, Kind::Code { caption: cap, caption_h: cap_h, caption_after, width: w, border: b, pad_y, divider })
        }, file);
    }

    #[allow(clippy::too_many_arguments)]
    fn table(&mut self, caption: &Caption, cont: &str, aligns: &[Align], head: &[Vec<Cell>], body: &[Vec<Cell>], anchor: &Anchor, ctx: &Ctx, file: &str, warn: &mut Warn) {
        let st = ctx.style;
        let cols = aligns.len();
        let tw = ctx.width(&st.table);
        let (cap, cap_h, cap_notes) = self.caption(caption, tw, ctx, file, warn);
        let caption_after = st.captions.table.caption_after;
        let mut head_notes = if caption_after { Vec::new() } else { cap_notes.clone() };
        let frame = &st.cell_frame;
        let extra_w = 2.0 * frame.pad_x + frame.border;
        let extra_h = 2.0 * frame.pad_y + frame.border;

        let head_cells = Table::cells(ctx, head, &st.head_cell, file, warn);
        let body_cells = Table::cells(ctx, body, &st.cell, file, warn);
        let all: Vec<&Vec<LCell>> = head_cells.iter().chain(&body_cells).collect();
        let widths = Table::column_widths(&all, cols, extra_w, st.cell.size, tw);
        let mut xs = vec![0.0];
        for w in &widths {
            xs.push(xs.last().unwrap() + w);
        }
        let t = Table { ctx, widths, xs, extra_w, extra_h };
        let lines = &st.table_lines;
        let body_top = if head.is_empty() { lines.frame } else { lines.head.rows };
        let head_rows = t.rows(head_cells, &owners(head, cols), &st.head_cell, None, lines.head, lines.frame);
        let body_rows = t.rows(body_cells, &owners(body, cols), &st.cell, Some(aligns), lines.body, body_top);

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
        if caption_after {
            groups.last_mut().unwrap().notes.extend(cap_notes);
        }

        let cont_role = &st.captions.table.role;
        let mut il = ctx.inline(cont_role);
        il.text(cont);
        let (cont_items, cont_h, _) = caption_box(il.lines(Layout::of(cont_role, tw)), cont_role, tw);
        self.push(Block {
            pieces: groups,
            id: Some(anchor.id()),
            ..spaced(&st.table, Kind::Table {
                caption: cap,
                caption_h: cap_h,
                caption_after,
                cont: cont_items,
                cont_h,
                head: head_items,
                head_h,
                width: tw,
                border: frame.border,
                frame: lines.frame,
                head_sep: lines.head.rows,
            })
        }, file);
    }
}

pub(crate) fn build(project: &ir::Project, ctx: &Ctx, warn: &mut Warn) -> Flow {
    let mut flow = Flow { w: ctx.w, ..Flow::default() };
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
        flow_styled(src, |_| ())
    }

    fn flow_styled(src: &str, tune: impl FnOnce(&mut Style)) -> (Flow, Vec<RenderWarning>) {
        let (proj, project, mut out) = project_of(src);
        let res = &proj.resources;
        let mut style = Style::of(crate::config::DocStyle::Gost7_32);
        tune(&mut style);
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
    fn table_lines_can_be_hidden() {
        let src = "\\№ Т\n/ a / b /\n\\№-\n/ 1 / 2 /\n/ 3 / 4 /\n\\№";
        let rules = |flow: &Flow| flow.blocks[1].pieces.iter().flat_map(|p| &p.items).filter(|i| matches!(i, Item::Rule { .. })).count();
        let (all, _) = flow_of(src);
        assert_eq!(rules(&all), 10);
        let (no_rows, _) = flow_styled(src, |s| s.table_lines.body.rows = false);
        assert_eq!(rules(&no_rows), 8, "остаётся линия под заголовком");
        let (no_cols, _) = flow_styled(src, |s| s.table_lines.body.cols = false);
        assert_eq!(rules(&no_cols), 8, "остаются боковые стороны рамки");
        let (bare, _) = flow_styled(src, |s| {
            s.table_lines.frame = false;
            s.table_lines.head.rows = false;
            s.table_lines.body.cols = false;
        });
        assert_eq!(rules(&bare), 2, "только линия между строками тела");
        let Kind::Table { frame, .. } = bare.blocks[1].kind else { panic!() };
        assert!(!frame);
    }

    #[test]
    fn code_numbers_and_frame() {
        let src = "\\// Л\nfn a()\n\nb\n\\//";
        let (flow, _) = flow_styled(src, |s| s.code_numbers = true);
        let pieces = &flow.blocks[1].pieces;
        assert_eq!(texts(&pieces[1]), vec!["", "2"]);
        let Kind::Code { divider, border, .. } = flow.blocks[1].kind else { panic!() };
        assert!(divider.is_some() && border > 0.0);
        let x = |p: &Piece, s: &str| p.items.iter().find_map(|i| match i {
            Item::Line { line, .. } if line.runs.iter().any(|r| r.text == s) => Some(line.x),
            _ => None,
        }).unwrap();
        assert!(x(&pieces[0], "1") < divider.unwrap() && divider.unwrap() < x(&pieces[0], "fn a()"));

        let (flow, _) = flow_styled(src, |s| {
            s.code_numbers = true;
            s.code_border = false;
        });
        let Kind::Code { divider, border, pad_y, .. } = flow.blocks[1].kind else { panic!() };
        assert_eq!((divider, border, pad_y), (None, 0.0, 0.0));
    }

    #[test]
    fn caption_order_follows_style() {
        let line_y = |p: &Piece| p.items.iter().find_map(|i| match i {
            Item::Line { y, .. } => Some(*y),
            _ => None,
        }).unwrap();
        let src = "\\// Л\nx\n\\//";
        let (before, _) = flow_of(src);
        let (after, _) = flow_styled(src, |s| s.captions.code.caption_after = true);
        let (b, a) = (&before.blocks[1], &after.blocks[1]);
        assert!(b.tail() == 0.0 && a.tail() > 0.0);
        assert_eq!(a.head(true) + a.tail(), b.head(true));

        let mut items = Vec::new();
        a.place(0, a.pieces.len(), true, &mut 0.0, &mut items);
        let caption_y = items.iter().filter_map(|i| match i {
            Item::Line { y, line } if line.runs.iter().any(|r| r.text.contains("Листинг")) => Some(*y),
            _ => None,
        }).next().unwrap();
        assert!(caption_y > line_y(&a.pieces[0]), "подпись под кодом");

        let src = "\\(нет.png, Р)";
        let (after, _) = flow_of(src);
        let (before, _) = flow_styled(src, |s| s.captions.image.caption_after = false);
        let caption_top = |f: &Flow| f.blocks[1].pieces[0].items.iter().find_map(|i| match i {
            Item::Line { y, line } if line.runs.iter().any(|r| r.text.contains("Рисунок")) => Some(*y),
            _ => None,
        }).unwrap();
        assert!(caption_top(&before) < caption_top(&after));
    }

    #[test]
    fn squeeze_narrows_and_shifts_block() {
        let src = "\\// Л\nx\n\\//";
        let (plain, _) = flow_of(src);
        let (sq, _) = flow_styled(src, |s| s.code.squeeze = 16.0);
        let (p, s) = (&plain.blocks[1], &sq.blocks[1]);
        assert_eq!((s.indent, s.pad_y), (16.0, 8.0));
        assert_eq!(s.head(true), p.head(true) + 8.0);
        assert_eq!(s.tail(), p.tail() + 8.0);
        let (Kind::Code { width: pw, .. }, Kind::Code { width: sw, .. }) = (&p.kind, &s.kind) else { panic!() };
        assert_eq!(*sw, pw - 32.0);
        let left = |b: &Block| {
            let mut items = Vec::new();
            b.place(0, b.pieces.len(), true, &mut 0.0, &mut items);
            items.iter().filter_map(|i| match i {
                Item::Rule { x, .. } => Some(*x),
                _ => None,
            }).fold(f32::MAX, f32::min)
        };
        assert_eq!(left(s), left(p) + 16.0);
    }

    #[test]
    fn background_and_text_color() {
        use crate::color::Color;
        let gray = Color::new(233, 233, 233);
        let (flow, _) = flow_styled("\\\"\nтекст\n\\\"", |s| {
            s.comment.bg_color = Some(gray);
            s.comment.text_color = Some(Color::new(1, 2, 3));
        });
        let b = &flow.blocks[1];
        let mut items = Vec::new();
        b.place(0, b.pieces.len(), true, &mut 0.0, &mut items);
        assert!(matches!(&items[0], Item::Fill { w, color, .. } if *w == b.width && *color == gray), "фон под текстом");
        assert!(items.iter().any(|i| matches!(i, Item::Line { line, .. } if line.runs.iter().any(|r| r.color == Some(Color::new(1, 2, 3))))));

        let (flow, _) = flow_styled("\\№ Т\n/ a / b /\n\\№-\n/ 1 / 2 /\n\\№", |s| s.head_cell.bg_color = Some(gray));
        let Kind::Table { head, .. } = &flow.blocks[1].kind else { panic!() };
        assert!(matches!(head[0], Item::Fill { .. }));
    }

    #[test]
    fn mono_fragment_background() {
        use crate::color::Color;
        let eee = Color::new(0xee, 0xee, 0xee);
        let (flow, _) = flow_styled("текст \\!код! текст\n\n\\// Л\nx\n\\//", |s| s.mono_bg = Some(eee));
        let runs = |b: &Block| b.pieces.iter().flat_map(|p| &p.items).filter_map(|i| match i {
            Item::Line { line, .. } => Some(line.runs.clone()),
            _ => None,
        }).flatten().collect::<Vec<PlacedRun>>();
        let para = runs(&flow.blocks[1]);
        assert!(para.iter().any(|r| r.text.contains("код") && r.bg == Some(eee)));
        assert!(para.iter().any(|r| r.text.contains("текст") && r.bg.is_none()));
        let code = flow.blocks.iter().find(|b| matches!(b.kind, Kind::Code { .. })).unwrap();
        assert!(runs(code).iter().all(|r| r.bg.is_none()));
    }

    #[test]
    fn external_links_styled() {
        use crate::color::Color;
        let blue = Color::new(0, 0, 255);
        let (flow, _) = flow_styled("\\,a.ru(сайт) и \\,\\#1(якорь)", |s| s.ext_link = crate::style::ExtLink { color: Some(blue), under: true });
        let runs: Vec<PlacedRun> = flow.blocks[1].pieces.iter().flat_map(|p| &p.items).filter_map(|i| match i {
            Item::Line { line, .. } => Some(line.runs.clone()),
            _ => None,
        }).flatten().collect();
        let site = runs.iter().find(|r| r.text.contains("сайт")).unwrap();
        assert!(site.under && site.color == Some(blue));
        assert!(runs.iter().filter(|r| r.text.contains('и')).all(|r| !r.under && r.color.is_none()));
    }

    #[test]
    fn caption_keeps_role_spacing() {
        let src = "\\// Л\nx\n\\//";
        let (plain, _) = flow_of(src);
        let (spaced, _) = flow_styled(src, |s| {
            s.captions.code.role.space_before = 3.0;
            s.captions.code.role.space_after = 10.0;
        });
        let cap = |f: &Flow| match &f.blocks[1].kind {
            Kind::Code { caption, caption_h, .. } => (*caption_h, caption.iter().find_map(|i| match i {
                Item::Line { y, .. } => Some(*y),
                _ => None,
            }).unwrap()),
            _ => panic!(),
        };
        let ((ph, py), (sh, sy)) = (cap(&plain), cap(&spaced));
        assert_eq!(sh, ph + 13.0);
        assert_eq!(sy, py + 3.0);
    }

    #[test]
    fn page_number_follows_style() {
        let (proj, project, mut out) = project_of("текст");
        let mut style = Style::of(crate::config::DocStyle::Gost7_32);
        style.toc = None;
        let x_of = |style: &Style, out: &mut Vec<RenderWarning>| {
            let mut warn = Warn { out, backend: DocOut::HtmlPaged };
            let lay = super::super::layout(&project, style, style.page.unwrap(), &proj.resources, &mut warn);
            let g = style.page.unwrap();
            lay.pages[0].items.iter().rev().find_map(|i| match i {
                Item::Line { line, .. } => Some((line.x, line.natural, g.text_w())),
                _ => None,
            }).unwrap()
        };
        let (x, n, w) = x_of(&style, &mut out);
        assert!((x - (w - n) / 2.0).abs() < 0.01);
        style.page = Some(PageGeometry { number_align: Align::Right, ..style.page.unwrap() });
        let (x, n, w) = x_of(&style, &mut out);
        assert!((x - (w - n)).abs() < 0.01);
    }

    #[test]
    fn list_levels_and_hanging_marker() {
        let long = "слово ".repeat(40);
        let src = format!("\\. {long}\n\\ . {long}");
        let xs = |flow: &Flow| flow.blocks[1].pieces.iter().filter_map(|p| match &p.items[0] {
            Item::Line { line, .. } => Some(line.x),
            _ => None,
        }).collect::<Vec<f32>>();
        let (flow, _) = flow_styled(&src, |s| {
            s.list_hang = true;
            s.list_step = 20.0;
        });
        let x = xs(&flow);
        assert!(x[1] > x[0]);
        let sub = x.iter().position(|&v| v > x[1]).unwrap();
        assert_eq!(x[sub], x[0] + 20.0);
    }

    #[test]
    fn element_margins_add_to_spacing() {
        let (plain, _) = flow_of("\\// Л\nx\n\\//");
        let (marg, _) = flow_styled("\\// Л\nx\n\\//", |s| {
            s.code.marg_before = 5.0;
            s.code.marg_after = 7.0;
        });
        assert_eq!(marg.blocks[1].space_before, plain.blocks[1].space_before + 5.0);
        assert_eq!(marg.blocks[1].space_after, plain.blocks[1].space_after + 7.0);
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
