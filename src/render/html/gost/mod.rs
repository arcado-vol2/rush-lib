use crate::parse::rush::Rush;
use crate::render::common::RenderWarning;
use crate::render::html::escape;
use crate::resources::{FileId, Id, Resources};
use std::collections::HashMap;
use std::fmt::Write;

mod flow;
mod inline;
pub(crate) mod metrics;
mod paginate;
mod toc;

pub(crate) const CSS: &str = include_str!("../../../templates/gost_7_32/style.css");
pub(crate) const CSS_NAME: &str = "style.css";
pub(crate) const FONTS_DIR: &str = "fonts";
pub(crate) const OUT_NAME: &str = "index.html";

const MM: f32 = 72.0 / 25.4;
const MARGIN_LEFT: f32 = 30.0;
const MARGIN_RIGHT: f32 = 15.0;
const MARGIN_TOP: f32 = 20.0;
const MARGIN_BOTTOM: f32 = 20.0;
pub(crate) const TEXT_W: f32 = (210.0 - MARGIN_LEFT - MARGIN_RIGHT) * MM;
pub(crate) const TEXT_H: f32 = (297.0 - MARGIN_TOP - MARGIN_BOTTOM) * MM;
pub(crate) const SLACK: f32 = 4.0;

pub(crate) const SIZE: f32 = 14.0;
pub(crate) const LH: f32 = 21.0;
pub(crate) const SMALL: f32 = 12.0;
pub(crate) const SMALL_LH: f32 = 14.0;
pub(crate) const INDENT: f32 = 12.5 * MM;
pub(crate) const NOTE_SEP: f32 = 10.0;
pub(crate) const TABLE_BORDER: f32 = 1.0;
pub(crate) const BORDER: f32 = 0.75;
pub(crate) const CELL_PAD_X: f32 = 4.0;
pub(crate) const CELL_PAD_Y: f32 = 2.0;
pub(crate) const CODE_PAD_X: f32 = 6.0;
pub(crate) const CODE_PAD_Y: f32 = 4.0;
pub(crate) const CODE_FRAME: f32 = CODE_PAD_Y * 2.0 + BORDER * 2.0;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Piece {
    pub(crate) html: String,
    pub(crate) height: f32,
    pub(crate) notes: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Kind {
    Text { open: String, close: &'static str },
    Table {
        caption: String,
        caption_h: f32,
        cont: String,
        cont_h: f32,
        open: String,
        head: String,
        head_h: f32,
    },
    Code { caption: String, caption_h: f32 },
    Whole,
    Anchor,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Block {
    pub(crate) kind: Kind,
    pub(crate) pieces: Vec<Piece>,
    pub(crate) space_before: f32,
    pub(crate) space_after: f32,
    pub(crate) keep_with_next: bool,
    pub(crate) break_before: bool,
    pub(crate) min_first: usize,
    pub(crate) min_last: usize,
    pub(crate) id: Option<String>,
}

impl Block {
    pub(crate) fn new(kind: Kind) -> Self {
        Self {
            kind,
            pieces: Vec::new(),
            space_before: 0.0,
            space_after: 0.0,
            keep_with_next: false,
            break_before: false,
            min_first: usize::MAX,
            min_last: 1,
            id: None,
        }
    }

    pub(crate) fn head(&self, first: bool) -> f32 {
        match &self.kind {
            Kind::Table { caption_h, cont_h, head_h, .. } => {
                (if first { *caption_h } else { *cont_h }) + head_h + TABLE_BORDER
            }
            Kind::Code { caption_h, .. } => (if first { *caption_h } else { 0.0 }) + CODE_FRAME,
            _ => 0.0,
        }
    }

    fn render(&self, out: &mut String, from: usize, to: usize, top: bool) {
        if matches!(self.kind, Kind::Anchor) {
            return;
        }
        let first = from == 0;
        let last = to == self.pieces.len();
        out.push_str("<div class=\"frag\"");
        if first && let Some(id) = &self.id {
            let _ = write!(out, " id=\"{id}\"");
        }
        let mt = if top { 0.0 } else { self.space_before };
        let mb = if last { self.space_after } else { 0.0 };
        if mt > 0.0 || mb > 0.0 {
            let _ = write!(out, " style=\"margin:{mt}pt 0 {mb}pt\"");
        }
        out.push('>');
        let pieces = self.pieces[from..to].iter().map(|p| p.html.as_str());
        match &self.kind {
            Kind::Text { open, close } => {
                out.push_str(open);
                pieces.for_each(|p| out.push_str(p));
                out.push_str(close);
            }
            Kind::Table { caption, cont, open, head, .. } => {
                out.push_str(if first { caption } else { cont });
                out.push_str(open);
                out.push_str(head);
                out.push_str("<tbody>");
                pieces.for_each(|p| out.push_str(p));
                out.push_str("</tbody></table>");
            }
            Kind::Code { caption, .. } => {
                if first {
                    out.push_str(caption);
                }
                out.push_str("<div class=\"code\">");
                pieces.for_each(|p| out.push_str(p));
                out.push_str("</div>");
            }
            Kind::Whole => pieces.for_each(|p| out.push_str(p)),
            Kind::Anchor => (),
        }
        out.push_str("</div>");
    }
}

pub(crate) struct Doc<'a> {
    pub(crate) file: Id<FileId>,
    pub(crate) name: &'a str,
    pub(crate) rush: Rush<'a>,
}

fn write_pages(out: &mut String, blocks: &[Block], pages: &[paginate::PageOut], first_num: usize, notes: &[flow::NoteBody]) {
    for (i, page) in pages.iter().enumerate() {
        out.push_str("<section class=\"page\"><div class=\"content\">");
        for f in &page.frags {
            blocks[f.block].render(out, f.from, f.to, f.top);
        }
        if !page.notes.is_empty() {
            out.push_str("<div class=\"notes\"><div class=\"sep\"></div>");
            for &g in &page.notes {
                if let Some(n) = notes.get(g as usize - 1) {
                    out.push_str(&n.html);
                }
            }
            out.push_str("</div>");
        }
        let _ = write!(out, "</div><div class=\"num\">{}</div></section>", first_num + i);
    }
}


fn page_of_ids(blocks: &[Block], pages: &[paginate::PageOut], first_num: usize) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for (i, page) in pages.iter().enumerate() {
        for f in &page.frags {
            if f.from == 0 && let Some(id) = &blocks[f.block].id {
                map.entry(id.clone()).or_insert(first_num + i);
            }
        }
    }
    map
}

pub(crate) fn render_project(docs: Vec<Doc>, res: &Resources) -> (String, Vec<RenderWarning>) {
    let mut warn = Vec::new();
    let title = docs.first().map(|d| d.name.to_string()).unwrap_or_default();
    let flow = flow::build(docs, res, &mut warn);
    let note_h = |g: u16| flow.notes.get(g as usize - 1).map_or(0.0, |n| n.height);

    let draft = toc::blocks(&flow.toc, None);
    let toc_pages = paginate::paginate(&draft, &note_h, &mut Vec::new()).len();
    let body = paginate::paginate(&flow.blocks, &note_h, &mut warn);
    let ids = page_of_ids(&flow.blocks, &body, toc_pages + 1);
    let toc = toc::blocks(&flow.toc, Some(&ids));
    let toc_layout = paginate::paginate(&toc, &note_h, &mut Vec::new());

    let mut out = String::from("<!DOCTYPE html><html lang=\"ru\"><head><meta charset=\"UTF-8\">");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>");
    let _ = escape(&mut out, &title);
    let _ = write!(out, "</title><style>:root{{--size:{SIZE}pt;--lh:{LH}pt;--small:{SMALL}pt;--slh:{SMALL_LH}pt;\
        --text-h:{TEXT_H:.3}pt;--note-sep:{NOTE_SEP}pt;--m-top:{MARGIN_TOP}mm;--m-right:{MARGIN_RIGHT}mm;\
        --m-bottom:{MARGIN_BOTTOM}mm;--m-left:{MARGIN_LEFT}mm;--border:{BORDER}pt;--cell-pad:{CELL_PAD_Y}pt {CELL_PAD_X}pt;\
        --code-pad:{CODE_PAD_Y}pt {CODE_PAD_X}pt}}</style>");
    let _ = write!(out, "<link rel=\"stylesheet\" href=\"{CSS_NAME}\"></head><body>");
    write_pages(&mut out, &toc, &toc_layout, 2, &flow.notes);
    write_pages(&mut out, &flow.blocks, &body, toc_pages + 1, &flow.notes);
    out.push_str("</body></html>");
    (out, warn)
}
