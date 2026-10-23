use super::flow::TocEntry;
use super::inline::{Align, Base, Inline, Layout};
use super::metrics::{width, Font};
use super::{Block, Kind, Piece, LH, SIZE, TEXT_W};
use std::collections::HashMap;


pub(crate) fn blocks(entries: &[TocEntry], pages: Option<&HashMap<String, usize>>) -> Vec<Block> {
    let mut title = Inline::new(Base { size: SIZE, font: Font { bold: true, ..Font::default() }, upper: true });
    title.text("Содержание");
    let title_lines = title.lines(Layout { width: TEXT_W, first: 0.0, rest: 0.0, align: Align::Center });
    let mut out = vec![Block {
        pieces: title_lines.into_iter().map(|l| Piece { html: l.html(), height: LH, notes: vec![] }).collect(),
        space_after: LH,
        keep_with_next: true,
        ..Block::new(Kind::Text { open: "<h1 class=\"toc-title\">".into(), close: "</h1>" })
    }];

    let digit = width("0", Font::default(), SIZE);
    let tail = digit * 4.0 + 12.0;
    for e in entries {
        let indent = (e.level.saturating_sub(1)) as f32 * 2.0 * digit;
        let mut il = Inline::new(Base::new(SIZE));
        il.text(&e.text);
        let mut lines = il.lines(Layout { width: TEXT_W - tail, first: indent, rest: indent, align: Align::Left });
        let page = pages.and_then(|p| p.get(&e.id)).map(usize::to_string).unwrap_or_default();
        let last = lines.pop().unwrap_or_else(|| super::inline::Line { style: String::new(), body: String::new(), notes: vec![] });
        let mut pieces: Vec<Piece> = lines.into_iter().map(|l| Piece { html: l.html(), height: LH, notes: vec![] }).collect();
        let style = if indent > 0.0 { format!(" style=\"padding-left:{indent:.2}pt\"") } else { String::new() };
        pieces.push(Piece {
            html: format!("<span class=\"l toc-end\"{style}><span class=\"tt\">{}</span><span class=\"dots\"></span><span class=\"pg\">{page}</span></span>", last.body),
            height: LH,
            notes: vec![],
        });
        out.push(Block {
            pieces,
            ..Block::new(Kind::Text { open: format!("<a class=\"toc\" href=\"#{}\">", e.id), close: "</a>" })
        });
    }
    out
}
