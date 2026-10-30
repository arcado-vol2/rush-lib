use crate::color::Color;
use crate::render::ir;
use crate::resources::Resources;
use crate::style::{Align, Font, PageGeometry, Style};
use std::collections::HashMap;

pub(crate) mod flow;
mod inline;
pub(crate) mod metrics;
mod paginate;
mod toc;

pub(crate) mod html_paged;

const TABLE_CLOSE: f32 = 1.0;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlacedRun {
    pub(crate) x: f32,
    pub(crate) text: String,
    pub(crate) font: Font,
    pub(crate) size: f32,
    pub(crate) sup: bool,
    pub(crate) under: bool,
    pub(crate) strike: bool,
    pub(crate) color: Option<Color>,
    pub(crate) bg: Option<Color>,
    pub(crate) link: Option<String>,
    pub(crate) error: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LineBox {
    pub(crate) x: f32,
    pub(crate) size: f32,
    pub(crate) lh: f32,
    pub(crate) word_spacing: f32,
    pub(crate) natural: f32,
    pub(crate) runs: Vec<PlacedRun>,
    pub(crate) notes: Vec<u16>,
}

impl LineBox {
    pub(crate) fn empty(lh: f32, size: f32) -> Self {
        Self { x: 0.0, size, lh, word_spacing: 0.0, natural: 0.0, runs: Vec::new(), notes: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    Line { y: f32, line: LineBox },
    Rule { x: f32, y: f32, w: f32, h: f32 },
    Fill { x: f32, y: f32, w: f32, h: f32, color: Color },
    Leader { x: f32, y: f32, w: f32 },
    Image { x: f32, y: f32, w: f32, h: f32, src: String },
    Math { x: f32, y: f32, w: f32, h: f32, size: f32, mathml: String },
    Anchor { y: f32, id: String },
}

impl Item {
    pub(crate) fn shift(&mut self, dx: f32, dy: f32) {
        match self {
            Item::Line { y, line } => {
                line.x += dx;
                *y += dy;
            }
            Item::Rule { x, y, .. } | Item::Fill { x, y, .. } | Item::Leader { x, y, .. } | Item::Image { x, y, .. } | Item::Math { x, y, .. } => {
                *x += dx;
                *y += dy;
            }
            Item::Anchor { y, .. } => *y += dy,
        }
    }
}

fn shifted(items: &[Item], dx: f32, dy: f32, out: &mut Vec<Item>) {
    out.extend(items.iter().cloned().map(|mut i| {
        i.shift(dx, dy);
        i
    }));
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Piece {
    pub(crate) items: Vec<Item>,
    pub(crate) height: f32,
    pub(crate) notes: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Kind {
    Text,
    Table { caption: Vec<Item>, caption_h: f32, caption_after: bool, cont: Vec<Item>, cont_h: f32, head: Vec<Item>, head_h: f32, width: f32, border: f32, frame: bool, head_sep: bool },
    Code { caption: Vec<Item>, caption_h: f32, caption_after: bool, width: f32, border: f32, pad_y: f32, divider: Option<f32> },
    Whole,
    Anchor,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Block {
    pub(crate) kind: Kind,
    pub(crate) pieces: Vec<Piece>,
    pub(crate) space_before: f32,
    pub(crate) space_after: f32,
    pub(crate) indent: f32,
    pub(crate) pad_y: f32,
    pub(crate) bg: Option<Color>,
    pub(crate) width: f32,
    pub(crate) keep_with_next: bool,
    pub(crate) break_before: bool,
    pub(crate) min_first: usize,
    pub(crate) min_last: usize,
    pub(crate) id: Option<String>,
    pub(crate) file: Option<String>,
}

impl Block {
    pub(crate) fn new(kind: Kind) -> Self {
        Self {
            kind,
            pieces: Vec::new(),
            space_before: 0.0,
            space_after: 0.0,
            indent: 0.0,
            pad_y: 0.0,
            bg: None,
            width: 0.0,
            keep_with_next: false,
            break_before: false,
            min_first: usize::MAX,
            min_last: 1,
            id: None,
            file: None,
        }
    }

    pub(crate) fn head(&self, first: bool) -> f32 {
        let pad = if first { self.pad_y } else { 0.0 };
        pad + match &self.kind {
            Kind::Table { caption_h, caption_after, cont_h, head_h, .. } => {
                let cap = match (first, caption_after) {
                    (true, false) => *caption_h,
                    (true, true) => 0.0,
                    (false, _) => *cont_h,
                };
                cap + head_h + TABLE_CLOSE
            }
            Kind::Code { caption_h, caption_after, border, pad_y, .. } => {
                (if first && !caption_after { *caption_h } else { 0.0 }) + 2.0 * (pad_y + border)
            }
            _ => 0.0,
        }
    }

    pub(crate) fn tail(&self) -> f32 {
        self.pad_y + match &self.kind {
            Kind::Table { caption_h, caption_after: true, .. } | Kind::Code { caption_h, caption_after: true, .. } => *caption_h,
            _ => 0.0,
        }
    }

    fn place(&self, from: usize, to: usize, top: bool, y: &mut f32, out: &mut Vec<Item>) {
        if matches!(self.kind, Kind::Anchor) {
            return;
        }
        let first = from == 0;
        let last = to == self.pieces.len();
        if !top {
            *y += self.space_before;
        }
        if first && let Some(id) = &self.id {
            out.push(Item::Anchor { y: *y, id: id.clone() });
        }
        let mark = out.len();
        let fill_top = *y;
        if first {
            *y += self.pad_y;
        }
        let pieces = &self.pieces[from..to];
        let stack = |y: &mut f32, dx: f32, out: &mut Vec<Item>| {
            for p in pieces {
                shifted(&p.items, dx, *y, out);
                *y += p.height;
            }
        };
        match &self.kind {
            Kind::Text | Kind::Whole => stack(y, 0.0, out),
            Kind::Table { caption, caption_h, caption_after, cont, cont_h, head, head_h, width, border, frame, head_sep } => {
                if !first {
                    shifted(cont, 0.0, *y, out);
                    *y += cont_h;
                } else if !caption_after {
                    shifted(caption, 0.0, *y, out);
                    *y += caption_h;
                }
                let rule = |y: f32| Item::Rule { x: -border / 2.0, y: y - border / 2.0, w: width + border, h: *border };
                if *frame {
                    out.push(rule(*y));
                }
                shifted(head, 0.0, *y, out);
                *y += head_h;
                if !first && *head_h > 0.0 && *head_sep {
                    out.push(rule(*y));
                }
                stack(y, 0.0, out);
                if *frame {
                    out.push(rule(*y));
                }
                *y += TABLE_CLOSE;
                if last && *caption_after {
                    shifted(caption, 0.0, *y, out);
                    *y += caption_h;
                }
            }
            Kind::Code { caption, caption_h, caption_after, width, border, pad_y, divider } => {
                if first && !caption_after {
                    shifted(caption, 0.0, *y, out);
                    *y += caption_h;
                }
                let top_y = *y;
                *y += border + pad_y;
                stack(y, 0.0, out);
                *y += pad_y + border;
                let h = *y - top_y;
                if *border > 0.0 {
                    out.push(Item::Rule { x: 0.0, y: top_y, w: *width, h: *border });
                    out.push(Item::Rule { x: 0.0, y: *y - border, w: *width, h: *border });
                    out.push(Item::Rule { x: 0.0, y: top_y, w: *border, h });
                    out.push(Item::Rule { x: width - border, y: top_y, w: *border, h });
                }
                if let Some(x) = divider {
                    out.push(Item::Rule { x: *x, y: top_y, w: *border, h });
                }
                if last && *caption_after {
                    shifted(caption, 0.0, *y, out);
                    *y += caption_h;
                }
            }
            Kind::Anchor => (),
        }
        if last {
            *y += self.pad_y;
        }
        for item in &mut out[mark..] {
            item.shift(self.indent, 0.0);
        }
        if let Some(color) = self.bg {
            out.insert(mark, Item::Fill { x: 0.0, y: fill_top, w: self.width, h: *y - fill_top, color });
        }
        if last {
            *y += self.space_after;
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Page {
    pub(crate) number: usize,
    pub(crate) items: Vec<Item>,
}

pub(crate) struct Layout {
    pub(crate) geometry: PageGeometry,
    pub(crate) title: String,
    pub(crate) pages: Vec<Page>,
}

fn pages(blocks: &[Block], layout: &[paginate::PageOut], first_num: usize, notes: &[flow::NoteBody], ctx: &flow::Ctx) -> Vec<Page> {
    let g = &ctx.geometry;
    let st = ctx.style;
    layout.iter().enumerate().map(|(i, page)| {
        let mut items = Vec::new();
        let mut y = 0.0;
        for f in &page.frags {
            blocks[f.block].place(f.from, f.to, f.top, &mut y, &mut items);
        }
        if !page.notes.is_empty() {
            let bodies: Vec<&flow::NoteBody> = page.notes.iter().filter_map(|&n| notes.get(n as usize - 1)).collect();
            let n = &st.notes;
            let total = note_sep(st) + bodies.iter().map(|b| b.height).sum::<f32>();
            let mut y = g.text_h() - total + n.role.marg_before;
            if n.rule {
                items.push(Item::Rule { x: 0.0, y: y + n.rule_top, w: n.rule_w, h: st.border });
            }
            y += n.sep;
            let pad = n.role.squeeze_y();
            let body_h: f32 = bodies.iter().map(|b| b.height).sum();
            if let Some(color) = n.role.bg_color {
                items.push(Item::Fill { x: 0.0, y, w: g.text_w(), h: body_h + 2.0 * pad, color });
            }
            y += pad;
            for b in bodies {
                shifted(&b.items, 0.0, y, &mut items);
                y += b.height;
            }
        }
        let number = first_num + i;
        if g.numbers {
            let lh = st.body.lh;
            let mut line = ctx.single(&number.to_string(), &st.body);
            line.x = match g.number_align {
                Align::Center => (g.text_w() - line.natural) / 2.0,
                Align::Right => g.text_w() - line.natural,
                Align::Left | Align::Justify => 0.0,
            };
            items.push(Item::Line { y: g.text_h() + g.bottom / 2.0 - lh / 2.0, line });
        }
        Page { number, items }
    }).collect()
}

fn note_sep(style: &Style) -> f32 {
    let role = &style.notes.role;
    role.marg_before + style.notes.sep + 2.0 * role.squeeze_y() + role.marg_after
}

fn page_of_ids(blocks: &[Block], layout: &[paginate::PageOut], first_num: usize) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for (i, page) in layout.iter().enumerate() {
        for f in &page.frags {
            if f.from == 0 && let Some(id) = &blocks[f.block].id {
                map.entry(id.clone()).or_insert(first_num + i);
            }
        }
    }
    map
}

pub(crate) fn layout(project: &ir::Project, style: &Style, geometry: PageGeometry, res: &Resources, warn: &mut flow::Warn) -> Layout {
    let ctx = flow::Ctx::new(style, geometry, res, project);
    let flow = flow::build(project, &ctx, warn);
    let note_h = |g: u16| flow.notes.get(g as usize - 1).map_or(0.0, |n| n.height);
    let room = paginate::Room { height: geometry.text_h() - geometry.slack, note_sep: note_sep(style) };

    let mut toc_pages = 0;
    let mut toc_out = Vec::new();
    let (body, tall) = paginate::paginate(&flow.blocks, room, &note_h);
    for i in tall {
        warn.too_tall(flow.blocks[i].file.clone());
    }
    if style.toc.is_some() {
        let draft = toc::blocks(&project.toc, None, &ctx);
        toc_pages = paginate::paginate(&draft, room, &note_h).0.len();
        let ids = page_of_ids(&flow.blocks, &body, toc_pages + 2);
        let toc = toc::blocks(&project.toc, Some(&ids), &ctx);
        let (toc_layout, _) = paginate::paginate(&toc, room, &note_h);
        toc_out = pages(&toc, &toc_layout, 2, &flow.notes, &ctx);
    }
    let mut out = toc_out;
    out.extend(pages(&flow.blocks, &body, toc_pages + 2, &flow.notes, &ctx));
    let title = project.docs.first().map(|d| d.title.clone()).unwrap_or_default();
    Layout { geometry, title, pages: out }
}
