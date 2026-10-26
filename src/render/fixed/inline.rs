use super::metrics::Metrics;
use super::{LineBox, PlacedRun};
use crate::parse::common::Color;
use crate::render::ir::{Link, Run, RunText};
use crate::style::{Align, Font, Role};
use std::collections::VecDeque;

const WIDTH_SAFETY: f32 = 0.5;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    pub(crate) width: f32,
    pub(crate) first: f32,
    pub(crate) rest: f32,
    pub(crate) align: Align,
}

impl Layout {
    pub(crate) fn of(role: &Role, width: f32) -> Self {
        Self { width, first: role.first, rest: role.rest, align: role.align }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Attrs {
    under: bool,
    strike: bool,
    color: Option<Color>,
    link: Option<String>,
    error: bool,
}

#[derive(Debug, Clone)]
struct Seg {
    text: String,
    /// unbreakable piece: reference number, note, error
    atomic: bool,
    sup: bool,
    font: Font,
    size: f32,
    attrs: usize,
    note: Option<u16>,
}

type Word = Vec<Seg>;

pub(crate) struct Inline<'m> {
    metrics: &'m Metrics,
    size: f32,
    lh: f32,
    font: Font,
    upper: bool,
    sup_scale: f32,
    attrs: Vec<Attrs>,
    words: Vec<Word>,
    cur: Word,
}

impl<'m> Inline<'m> {
    pub(crate) fn new(metrics: &'m Metrics, role: &Role, sup_scale: f32) -> Self {
        Self {
            metrics,
            size: role.size,
            lh: role.lh,
            font: role.font,
            upper: role.upper,
            sup_scale,
            attrs: vec![Attrs::default()],
            words: Vec::new(),
            cur: Vec::new(),
        }
    }

    fn seg_width(&self, seg: &Seg) -> f32 {
        self.metrics.width(&seg.text, seg.font, seg.size)
    }

    fn word_width(&self, word: &Word) -> f32 {
        word.iter().map(|s| self.seg_width(s)).sum()
    }

    fn space_width(&self, word: &Word) -> f32 {
        let last = word.last().expect("пустых слов нет");
        self.metrics.width(" ", last.font, last.size)
    }

    pub(crate) fn text(&mut self, s: &str) {
        self.push_str(s, 0, self.font);
    }

    pub(crate) fn space(&mut self) {
        self.brk();
    }

    pub(crate) fn marker(&mut self, text: String) {
        let seg = Seg { text, atomic: true, sup: true, font: self.font, size: self.size * self.sup_scale, attrs: 0, note: None };
        self.cur.push(seg);
    }

    fn brk(&mut self) {
        if !self.cur.is_empty() {
            self.words.push(std::mem::take(&mut self.cur));
        }
    }

    fn push_seg(&mut self, seg: Seg) {
        if let Some(last) = self.cur.last_mut()
            && !last.atomic && !seg.atomic
            && last.attrs == seg.attrs && last.font == seg.font && last.size == seg.size
        {
            last.text.push_str(&seg.text);
            return;
        }
        self.cur.push(seg);
    }

    fn push_str(&mut self, s: &str, attrs: usize, font: Font) {
        for (i, part) in s.split(' ').enumerate() {
            if i > 0 {
                self.brk();
            }
            if !part.is_empty() {
                let text = if self.upper { part.to_uppercase() } else { part.to_string() };
                self.push_seg(Seg { text, atomic: false, sup: false, font, size: self.size, attrs, note: None });
            }
        }
    }

    fn attrs_of(&mut self, a: Attrs) -> usize {
        match self.attrs.iter().position(|x| *x == a) {
            Some(i) => i,
            None => {
                self.attrs.push(a);
                self.attrs.len() - 1
            }
        }
    }

    /// `href` - link address in the output file, `None` if the link has no target
    pub(crate) fn runs(&mut self, runs: &[Run], href: &mut impl FnMut(&Link) -> Option<String>) {
        for run in runs {
            let font = self.font.with(run.font);
            let link = run.link.as_ref().and_then(&mut *href);
            let base = Attrs { under: run.under, strike: run.strike, color: run.color.clone(), link, error: false };
            let size = self.size;
            match &run.text {
                RunText::Text(s) => {
                    let a = self.attrs_of(base);
                    self.push_str(s, a, font);
                }
                RunText::Space => self.brk(),
                RunText::Ref(i) => {
                    let a = self.attrs_of(base);
                    self.cur.push(Seg { text: i.to_string(), atomic: true, sup: false, font, size, attrs: a, note: None });
                }
                RunText::Note(g) => {
                    let a = self.attrs_of(base);
                    self.cur.push(Seg { text: g.to_string(), atomic: true, sup: true, font, size: size * self.sup_scale, attrs: a, note: Some(*g) });
                    self.brk();
                }
                RunText::Error => {
                    let a = self.attrs_of(Attrs { error: true, ..base });
                    self.cur.push(Seg { text: "??".into(), atomic: true, sup: false, font: Font { mono: true, ..font }, size, attrs: a, note: None });
                }
            }
        }
    }

    fn all_words(&self) -> impl Iterator<Item = &Word> {
        self.words.iter().chain((!self.cur.is_empty()).then_some(&self.cur))
    }

    pub(crate) fn natural_width(&self) -> f32 {
        let mut w = 0.0;
        let mut prev: Option<&Word> = None;
        for word in self.all_words() {
            if let Some(p) = prev {
                w += self.space_width(p);
            }
            w += self.word_width(word);
            prev = Some(word);
        }
        w
    }

    pub(crate) fn min_width(&self) -> f32 {
        self.all_words().map(|w| self.word_width(w)).fold(0.0, f32::max)
    }

    fn split_word(&self, word: Word, avail: f32) -> (Word, Word) {
        let mut head = Vec::new();
        let mut used = 0.0;
        let mut segs = word.into_iter();
        while let Some(mut seg) = segs.next() {
            let w = self.seg_width(&seg);
            if used + w <= avail || (head.is_empty() && seg.atomic) {
                used += w;
                head.push(seg);
                continue;
            }
            if !seg.atomic {
                let mut cut = 0;
                for (i, c) in seg.text.char_indices() {
                    let cw = self.metrics.width(&seg.text[i..i + c.len_utf8()], seg.font, seg.size);
                    if used + cw > avail && !(head.is_empty() && cut == 0) {
                        break;
                    }
                    used += cw;
                    cut = i + c.len_utf8();
                }
                if cut > 0 {
                    let tail = seg.text.split_off(cut);
                    let mut rest = seg.clone();
                    rest.text = tail;
                    head.push(seg);
                    let mut tail_word = vec![rest];
                    tail_word.extend(segs);
                    tail_word.retain(|s| !s.text.is_empty() || s.atomic);
                    return (head, tail_word);
                }
            }
            let mut tail_word = vec![seg];
            tail_word.extend(segs);
            return (head, tail_word);
        }
        (head, Vec::new())
    }

    pub(crate) fn lines(&self, lay: Layout) -> Vec<LineBox> {
        let mut queue: VecDeque<Word> = self.all_words().cloned().collect();
        let mut out: Vec<(Vec<Word>, f32, f32)> = Vec::new();
        let mut cur: Vec<Word> = Vec::new();
        let mut cur_w = 0.0;
        let avail_for = |n: usize| (lay.width - if n == 0 { lay.first } else { lay.rest } - WIDTH_SAFETY).max(1.0);
        while let Some(word) = queue.pop_front() {
            let avail = avail_for(out.len());
            let ww = self.word_width(&word);
            let sp = cur.last().map_or(0.0, |w| self.space_width(w));
            if cur_w + sp + ww <= avail {
                cur.push(word);
                cur_w += sp + ww;
                continue;
            }
            if cur.is_empty() {
                let (head, tail) = self.split_word(word, avail);
                let hw = self.word_width(&head);
                out.push((vec![head], hw, avail));
                if !tail.is_empty() {
                    queue.push_front(tail);
                }
                continue;
            }
            out.push((std::mem::take(&mut cur), cur_w, avail));
            cur_w = 0.0;
            queue.push_front(word);
        }
        if !cur.is_empty() {
            out.push((cur, cur_w, avail_for(out.len())));
        }

        let count = out.len();
        out.into_iter().enumerate().map(|(n, (words, natural, avail))| {
            let indent = if n == 0 { lay.first } else { lay.rest };
            let x = match lay.align {
                Align::Justify | Align::Left => indent,
                Align::Center => indent + ((lay.width - indent - natural) / 2.0).max(0.0),
                Align::Right => (lay.width - natural).max(indent),
            };
            let justify = lay.align == Align::Justify && n + 1 < count;
            self.line(&words, natural, avail, x, justify)
        }).collect()
    }

    fn line(&self, words: &[Word], natural: f32, avail: f32, x: f32, justify: bool) -> LineBox {
        let mut word_spacing = 0.0;
        if justify && natural < avail {
            let gaps = words.len() - 1 + words.iter()
                .flat_map(|w| w.iter())
                .filter(|s| !s.atomic)
                .map(|s| s.text.matches('\u{a0}').count())
                .sum::<usize>();
            if gaps > 0 {
                word_spacing = (avail - natural) / gaps as f32;
            }
        }
        let mut runs: Vec<PlacedRun> = Vec::new();
        let mut notes = Vec::new();
        let mut pos = 0.0;
        let push = |runs: &mut Vec<PlacedRun>, seg: &Seg, text: &str, pos: &mut f32| {
            let a = &self.attrs[seg.attrs];
            let gaps = text.chars().filter(|&c| c == ' ' || c == '\u{a0}').count() as f32;
            let w = self.metrics.width(text, seg.font, seg.size) + gaps * word_spacing;
            if let Some(last) = runs.last_mut()
                && !seg.sup && !last.sup && last.font == seg.font && last.size == seg.size
                && last.under == a.under && last.strike == a.strike && last.color == a.color
                && last.link == a.link && last.error == a.error
            {
                last.text.push_str(text);
            } else {
                runs.push(PlacedRun {
                    x: *pos,
                    text: text.to_string(),
                    font: seg.font,
                    size: seg.size,
                    sup: seg.sup,
                    under: a.under,
                    strike: a.strike,
                    color: a.color.clone(),
                    link: a.link.clone(),
                    error: a.error,
                });
            }
            *pos += w;
        };
        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                let last = words[i - 1].last().expect("пустых слов нет");
                push(&mut runs, last, " ", &mut pos);
            }
            for seg in word {
                push(&mut runs, seg, &seg.text, &mut pos);
                if let Some(g) = seg.note {
                    notes.push(g);
                }
            }
        }
        LineBox { x, size: self.size, lh: self.lh, word_spacing, natural, runs, notes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DocStyle;
    use crate::style::Style;

    fn text_of(l: &LineBox) -> String {
        l.runs.iter().map(|r| r.text.as_str()).collect()
    }

    #[test]
    fn breaks_by_width_and_justifies_all_but_last() {
        let style = Style::of(DocStyle::Gost);
        let m = Metrics::new(&style.fonts);
        let role = Role::text(14.0, 21.0);
        let one = m.width("слово", Font::default(), 14.0);
        let space = m.width(" ", Font::default(), 14.0);
        let w = one * 2.0 + space + WIDTH_SAFETY + 1.0;
        let mut i = Inline::new(&m, &role, 0.7);
        i.text("слово слово слово слово слово");
        let out = i.lines(Layout { width: w, first: 0.0, rest: 0.0, align: Align::Justify });
        let bodies: Vec<String> = out.iter().map(text_of).collect();
        assert_eq!(bodies, vec!["слово слово", "слово слово", "слово"]);
        assert!(out[0].word_spacing > 0.0);
        assert_eq!(out[2].word_spacing, 0.0);
    }

    #[test]
    fn long_word_is_cut_by_chars() {
        let style = Style::of(DocStyle::Gost);
        let m = Metrics::new(&style.fonts);
        let role = Role::text(14.0, 21.0);
        let w = m.width("abcd", Font::default(), 14.0) + WIDTH_SAFETY + 0.1;
        let mut i = Inline::new(&m, &role, 0.7);
        i.text("abcdabcdab");
        let out = i.lines(Layout { width: w, first: 0.0, rest: 0.0, align: Align::Left });
        let bodies: Vec<String> = out.iter().map(text_of).collect();
        assert_eq!(bodies, vec!["abcd", "abcd", "ab"]);
    }

    #[test]
    fn indent_center_and_run_positions() {
        let style = Style::of(DocStyle::Gost);
        let m = Metrics::new(&style.fonts);
        let role = Role::text(14.0, 21.0);
        let mut i = Inline::new(&m, &role, 0.7);
        i.text("a b");
        let l = &i.lines(Layout { width: 400.0, first: 35.0, rest: 0.0, align: Align::Left })[0];
        assert_eq!(l.x, 35.0);
        let c = &i.lines(Layout { width: 400.0, first: 0.0, rest: 0.0, align: Align::Center })[0];
        assert!((c.x - (400.0 - c.natural) / 2.0).abs() < 1e-3);
        assert_eq!(c.runs.len(), 1);
    }
}
