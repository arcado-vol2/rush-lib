use super::flow::Ctx;
use super::inline::Layout;
use super::{Block, Item, Kind, LineBox, Piece};
use crate::render::ir::{Link, Run, TocEntry};
use std::collections::HashMap;

/// Table of contents blocks; without `pages` - a draft to measure its length
pub(crate) fn blocks(entries: &[TocEntry], pages: Option<&HashMap<String, usize>>, ctx: &Ctx) -> Vec<Block> {
    let Some(toc) = ctx.style.toc else { return Vec::new() };
    let w = ctx.w;
    let title_role = &toc.title_role;
    let mut title = ctx.inline(title_role);
    title.text(toc.title);
    let title_lines = title.lines(Layout::of(title_role, w));
    let mut out = vec![Block {
        pieces: title_lines.into_iter().map(|line| Piece { height: line.lh, items: vec![Item::Line { y: 0.0, line }], notes: vec![] }).collect(),
        space_after: title_role.space_after,
        keep_with_next: title_role.keep_with_next,
        ..Block::new(Kind::Text)
    }];

    let role = &toc.entry;
    let digit = ctx.metrics.width("0", role.font, role.size);
    let tail = digit * 4.0 + 12.0;
    for e in entries {
        let id = e.anchor.id();
        let link = |text: &str| Run { link: Some(Link::Anchor(e.anchor.clone())), ..Run::text(text) };
        let href = &mut |_: &Link| Some(format!("#{id}"));
        let indent = (e.level.saturating_sub(1)) as f32 * 2.0 * digit;
        let mut il = ctx.inline(role);
        il.runs(&[link(&e.text)], href);
        let mut lines = il.lines(Layout { width: w - tail, first: indent, rest: indent, align: crate::style::Align::Left });
        let last = lines.pop().unwrap_or_else(|| LineBox { x: indent, ..LineBox::empty(role.lh, role.size) });
        let mut pieces: Vec<Piece> = lines.into_iter().map(|line| Piece { height: line.lh, items: vec![Item::Line { y: 0.0, line }], notes: vec![] }).collect();

        let page = pages.and_then(|p| p.get(&id)).map(usize::to_string).unwrap_or_default();
        let mut num = ctx.inline(role);
        num.runs(&[link(&page)], href);
        let mut num = num.lines(Layout { width: f32::MAX / 4.0, first: 0.0, rest: 0.0, align: crate::style::Align::Left })
            .pop()
            .unwrap_or_else(|| LineBox::empty(role.lh, role.size));
        num.x = w - num.natural;
        let from = last.x + last.natural + 3.0;
        let to = w - num.natural.max(2.0 * role.size) - 3.0;
        let lh = last.lh;
        let mut items = vec![Item::Line { y: 0.0, line: last }];
        if to - from >= 6.0 {
            items.push(Item::Leader { x: from, y: lh * 0.55, w: to - from });
        }
        items.push(Item::Line { y: 0.0, line: num });
        pieces.push(Piece { items, height: lh, notes: vec![] });
        out.push(Block { pieces, ..Block::new(Kind::Text) });
    }
    out
}
