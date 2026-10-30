use super::{Block, Kind};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Frag {
    pub(crate) block: usize,
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) top: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct PageOut {
    pub(crate) frags: Vec<Frag>,
    pub(crate) notes: Vec<u16>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Room {
    /// text area height minus the reserve
    pub(crate) height: f32,
    pub(crate) note_sep: f32,
}

struct State<'n> {
    pages: Vec<PageOut>,
    used: f32,
    notes_h: f32,
    content: bool,
    placed: HashSet<u16>,
    room: Room,
    note_h: &'n dyn Fn(u16) -> f32,
}

impl State<'_> {
    fn new_page(&mut self) {
        self.pages.push(PageOut::default());
        self.used = 0.0;
        self.notes_h = 0.0;
        self.content = false;
    }

    fn avail(&self) -> f32 {
        self.room.height - self.used - self.notes_h
    }

    fn extra(&self, notes: &[u16], pending: &[u16]) -> f32 {
        let mut h = 0.0;
        let mut seen: Vec<u16> = Vec::new();
        for &g in notes {
            if self.placed.contains(&g) || pending.contains(&g) || seen.contains(&g) {
                continue;
            }
            if self.pages.last().unwrap().notes.is_empty() && pending.is_empty() && seen.is_empty() {
                h += self.room.note_sep;
            }
            seen.push(g);
            h += (self.note_h)(g);
        }
        h
    }

    fn take_notes(&mut self, notes: &[u16]) {
        for &g in notes {
            if self.placed.insert(g) {
                if self.pages.last().unwrap().notes.is_empty() {
                    self.notes_h += self.room.note_sep;
                }
                self.notes_h += (self.note_h)(g);
                self.pages.last_mut().unwrap().notes.push(g);
            }
        }
    }
}

fn lead(blocks: &[Block], j: usize) -> f32 {
    let Some(b) = blocks.get(j) else { return 0.0 };
    if b.break_before {
        return 0.0;
    }
    let n = b.min_first.min(b.pieces.len());
    let tail = if n == b.pieces.len() { b.tail() } else { 0.0 };
    let own = b.space_before + b.head(true) + b.pieces[..n].iter().map(|p| p.height).sum::<f32>() + tail;
    if b.keep_with_next && n == b.pieces.len() { own + lead(blocks, j + 1) } else { own }
}

/// Lays blocks out on pages; second - blocks taller than a page
pub(crate) fn paginate(blocks: &[Block], room: Room, note_h: &dyn Fn(u16) -> f32) -> (Vec<PageOut>, Vec<usize>) {
    let mut st = State { pages: vec![PageOut::default()], used: 0.0, notes_h: 0.0, content: false, placed: HashSet::new(), room, note_h };
    let mut too_tall = Vec::new();

    for (i, b) in blocks.iter().enumerate() {
        if b.break_before && st.content {
            st.new_page();
        }
        let n = b.pieces.len();
        let mut start = 0;
        while start < n {
            let top = !st.content;
            let first = start == 0;
            let before = if top { 0.0 } else { b.space_before } + b.head(first);

            let mut h = before;
            let mut pending: Vec<u16> = Vec::new();
            let mut notes_h = 0.0;
            let mut k = start;
            while k < n {
                let piece = &b.pieces[k];
                let add = st.extra(&piece.notes, &pending);
                let tail = if k + 1 == n { b.tail() } else { 0.0 };
                if h + piece.height + tail + notes_h + add > st.avail() {
                    break;
                }
                h += piece.height;
                notes_h += add;
                for &g in &piece.notes {
                    if !pending.contains(&g) {
                        pending.push(g);
                    }
                }
                k += 1;
            }

            let mut end = k;
            if k < n {
                let min_head = if first { b.min_first } else { 1 };
                end = k.min(n.saturating_sub(b.min_last));
                if end < start.saturating_add(min_head) || end <= start {
                    if !top {
                        st.new_page();
                        continue;
                    }
                    if b.min_first >= n {
                        too_tall.push(i);
                        end = n;
                    } else {
                        end = k.max(start + 1);
                    }
                }
            } else if b.keep_with_next && !top && h + b.tail() + notes_h + lead(blocks, i + 1) > st.avail() {
                st.new_page();
                continue;
            }

            let visible = b.pieces[start..end].iter().any(|p| p.height > 0.0) || !matches!(b.kind, Kind::Anchor);
            let used: f32 = if visible { before } else { 0.0 } + b.pieces[start..end].iter().map(|p| p.height).sum::<f32>();
            for p in &b.pieces[start..end] {
                st.take_notes(&p.notes);
            }
            st.used += used;
            st.pages.last_mut().unwrap().frags.push(Frag { block: i, from: start, to: end, top });
            if visible {
                st.content = true;
            }
            if end == n {
                st.used += b.tail() + b.space_after;
                break;
            }
            start = end;
            st.new_page();
        }
    }
    (st.pages, too_tall)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Piece;

    const LH: f32 = 21.0;
    const ROOM: Room = Room { height: 700.0, note_sep: 10.0 };

    fn piece(height: f32) -> Piece {
        Piece { items: Vec::new(), height, notes: vec![] }
    }

    fn text(lines: usize) -> Block {
        Block {
            pieces: (0..lines).map(|_| piece(LH)).collect(),
            min_first: 2,
            min_last: 2,
            ..Block::new(Kind::Text)
        }
    }

    fn whole(height: f32) -> Block {
        Block { pieces: vec![piece(height)], ..Block::new(Kind::Whole) }
    }

    fn per_page() -> usize {
        (ROOM.height / LH) as usize
    }

    fn run(blocks: &[Block]) -> Vec<PageOut> {
        paginate(blocks, ROOM, &|_| LH).0
    }

    fn shape(pages: &[PageOut]) -> Vec<Vec<(usize, usize, usize)>> {
        pages.iter().map(|p| p.frags.iter().map(|f| (f.block, f.from, f.to)).collect()).collect()
    }

    #[test]
    fn paragraph_splits_with_orphans_and_widows() {
        let n = per_page();
        let pages = run(&[text(n - 1), text(3)]);
        assert_eq!(shape(&pages), vec![vec![(0, 0, n - 1)], vec![(1, 0, 3)]]);
        let pages = run(&[text(n + 1)]);
        assert_eq!(shape(&pages), vec![vec![(0, 0, n - 1)], vec![(0, n - 1, n + 1)]]);
    }

    #[test]
    fn whole_block_moves_to_next_page() {
        let n = per_page();
        let pages = run(&[text(n - 3), whole(LH * 4.0)]);
        assert_eq!(shape(&pages), vec![vec![(0, 0, n - 3)], vec![(1, 0, 1)]]);
    }

    #[test]
    fn header_keeps_with_next() {
        let n = per_page();
        let header = Block { keep_with_next: true, min_first: 1, ..text(1) };
        let pages = run(&[text(n - 2), header, text(4)]);
        assert_eq!(shape(&pages), vec![vec![(0, 0, n - 2)], vec![(1, 0, 1), (2, 0, 4)]]);
    }

    #[test]
    fn note_pushes_line_to_next_page() {
        let n = per_page();
        let mut b = text(n);
        b.pieces[n - 3].notes = vec![1];
        let (pages, _) = paginate(&[b], ROOM, &|_| LH * 3.0);
        assert_eq!(pages[0].notes, Vec::<u16>::new());
        assert_eq!(pages[1].notes, vec![1]);
        assert_eq!(shape(&pages)[0], vec![(0, 0, n - 3)]);
    }

    #[test]
    fn break_before_and_too_tall() {
        let doc = Block { break_before: true, ..text(2) };
        let (pages, tall) = paginate(&[text(2), doc, whole(ROOM.height * 2.0)], ROOM, &|_| LH);
        assert_eq!(shape(&pages), vec![vec![(0, 0, 2)], vec![(1, 0, 2)], vec![(2, 0, 1)]]);
        assert_eq!(tall, vec![2]);
    }
}
