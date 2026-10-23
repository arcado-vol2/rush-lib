use super::metrics::{width, Font};
use crate::parse::common::RefKind;
use crate::parse::rush::{InlineEl, OuterLink, Span};
use crate::render::common::RenderWarning;
use crate::render::html::{escape, parse_url};
use crate::resources::Resources;
use std::collections::{HashMap, VecDeque};
use std::fmt::Write;


const SUP_SCALE: f32 = 0.7;
const WIDTH_SAFETY: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    Justify,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Base {
    pub(crate) size: f32,
    pub(crate) font: Font,
    pub(crate) upper: bool,
}

impl Base {
    pub(crate) fn new(size: f32) -> Self {
        Self { size, font: Font::default(), upper: false }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    pub(crate) width: f32,
    pub(crate) first: f32,
    pub(crate) rest: f32,
    pub(crate) align: Align,
}

#[derive(Debug, Default)]
pub(crate) struct Notes {
    pub(crate) local: HashMap<u16, u16>,
    pub(crate) last: u16,
}

impl Notes {
    pub(crate) fn global(&mut self, local: u16) -> u16 {
        if let Some(&g) = self.local.get(&local) {
            return g;
        }
        let g = self.fresh();
        self.local.insert(local, g);
        g
    }

    pub(crate) fn fresh(&mut self) -> u16 {
        self.last = self.last.saturating_add(1);
        self.last
    }
}

pub(crate) struct Ctx<'c> {
    pub(crate) res: &'c Resources,
    pub(crate) docs: &'c HashMap<usize, usize>,
    pub(crate) notes: Notes,
    pub(crate) warn: &'c mut Vec<RenderWarning>,
}

#[derive(Debug, Clone)]
struct Seg {
    text: String,
    html: Option<String>,
    font: Font,
    size: f32,
    style: usize,
    note: Option<u16>,
}

type Word = Vec<Seg>;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Line {
    pub(crate) style: String,
    pub(crate) body: String,
    pub(crate) notes: Vec<u16>,
}

impl Line {
    pub(crate) fn html(&self) -> String {
        if self.style.is_empty() {
            format!("<span class=\"l\">{}</span>", self.body)
        } else {
            format!("<span class=\"l\" style=\"{}\">{}</span>", self.style, self.body)
        }
    }
}

pub(crate) struct Inline {
    base: Base,
    styles: Vec<(String, String)>,
    words: Vec<Word>,
    cur: Word,
}

fn seg_width(seg: &Seg) -> f32 {
    width(&seg.text, seg.font, seg.size)
}

fn word_width(word: &Word) -> f32 {
    word.iter().map(seg_width).sum()
}

fn space_width(word: &Word) -> f32 {
    let last = word.last().expect("пустых слов нет");
    width(" ", last.font, last.size)
}

fn split_word(word: Word, avail: f32) -> (Word, Word) {
    let mut head = Vec::new();
    let mut used = 0.0;
    let mut segs = word.into_iter();
    while let Some(mut seg) = segs.next() {
        let w = seg_width(&seg);
        if used + w <= avail || (head.is_empty() && seg.html.is_some()) {
            used += w;
            head.push(seg);
            continue;
        }
        if seg.html.is_none() {
            let mut cut = 0;
            for (i, c) in seg.text.char_indices() {
                let cw = width(&seg.text[i..i + c.len_utf8()], seg.font, seg.size);
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
                tail_word.retain(|s| !s.text.is_empty() || s.html.is_some());
                return (head, tail_word);
            }
        }
        let mut tail_word = vec![seg];
        tail_word.extend(segs);
        return (head, tail_word);
    }
    (head, Vec::new())
}

impl Inline {
    pub(crate) fn new(base: Base) -> Self {
        Self { base, styles: vec![(String::new(), String::new())], words: Vec::new(), cur: Vec::new() }
    }

    pub(crate) fn text(&mut self, s: &str) {
        self.push_str(s, 0, self.base.font, self.base.size);
    }

    pub(crate) fn space(&mut self) {
        self.brk();
    }

    pub(crate) fn raw(&mut self, html: String, text: String, scale: f32) {
        let seg = Seg { text, html: Some(html), font: self.base.font, size: self.base.size * scale, style: 0, note: None };
        self.cur.push(seg);
    }

    fn brk(&mut self) {
        if !self.cur.is_empty() {
            self.words.push(std::mem::take(&mut self.cur));
        }
    }

    fn push_seg(&mut self, seg: Seg) {
        if let Some(last) = self.cur.last_mut()
            && last.html.is_none() && seg.html.is_none()
            && last.style == seg.style && last.font == seg.font && last.size == seg.size
        {
            last.text.push_str(&seg.text);
            return;
        }
        self.cur.push(seg);
    }

    fn push_str(&mut self, s: &str, style: usize, font: Font, size: f32) {
        for (i, part) in s.split(' ').enumerate() {
            if i > 0 {
                self.brk();
            }
            if !part.is_empty() {
                let text = if self.base.upper { part.to_uppercase() } else { part.to_string() };
                self.push_seg(Seg { text, html: None, font, size, style, note: None });
            }
        }
    }

    pub(crate) fn spans(&mut self, spans: Vec<Span>, ctx: &mut Ctx) {
        for span in spans {
            let st = &span.style;
            let tags = [
                (st.is_bold, "<b>", "</b>"),
                (st.is_italic, "<i>", "</i>"),
                (st.is_mono, "<span class=\"mono\">", "</span>"),
                (st.is_strike, "<s>", "</s>"),
                (st.is_under, "<u>", "</u>"),
            ];
            let mut open = String::new();
            let mut close = String::new();
            for &(on, o, _) in &tags {
                if on {
                    open.push_str(o);
                }
            }
            if let Some(col) = &st.color {
                let _ = write!(open, "<span style=\"color:{}\">", col.to_hex_tag());
            }
            let mut link = false;
            match &st.link {
                Some(OuterLink::URL(raw)) => match parse_url(raw) {
                    Some(url) => {
                        open.push_str("<a href=\"");
                        let _ = escape(&mut open, url.as_str());
                        open.push_str("\">");
                        link = true;
                    }
                    None => ctx.warn.push(RenderWarning::InvalidUrl(raw.to_string())),
                },
                Some(OuterLink::File(id)) => match ctx.docs.get(&id.idx) {
                    Some(k) => {
                        let _ = write!(open, "<a href=\"#d-{k}\">");
                        link = true;
                    }
                    None => ctx.warn.push(RenderWarning::FileNotInContent(
                        ctx.res.files.local_path(*id).to_string_lossy().into_owned(),
                    )),
                },
                None => (),
            }
            if link {
                close.push_str("</a>");
            }
            if st.color.is_some() {
                close.push_str("</span>");
            }
            for &(on, _, c) in tags.iter().rev() {
                if on {
                    close.push_str(c);
                }
            }
            let style = if open.is_empty() {
                0
            } else {
                self.styles.push((open, close));
                self.styles.len() - 1
            };
            let font = Font {
                mono: st.is_mono || self.base.font.mono,
                bold: st.is_bold || self.base.font.bold,
                italic: st.is_italic || self.base.font.italic,
            };
            let size = self.base.size;

            for el in span.text {
                match el {
                    InlineEl::String(s) => self.push_str(s, style, font, size),
                    InlineEl::Space => self.brk(),
                    InlineEl::LongDash => self.push_str("—", style, font, size),
                    InlineEl::InnerLink(Ok(l)) => {
                        let prefix = match l.kind {
                            RefKind::Table => 't',
                            RefKind::Code => 'c',
                            RefKind::Formula => 'f',
                            RefKind::Image => 'i',
                        };
                        let text = l.index.to_string();
                        let html = if link { text.clone() } else { format!("<a href=\"#{prefix}-{text}\">{text}</a>") };
                        self.cur.push(Seg { text, html: Some(html), font, size, style, note: None });
                    }
                    InlineEl::InnerLink(Err((name, kind))) => {
                        ctx.warn.push(RenderWarning::UnresolvedRef { kind, name: name.into_owned() });
                        self.error(font, size, style);
                    }
                    InlineEl::Note(Ok(i)) => {
                        let g = ctx.notes.global(i);
                        let text = g.to_string();
                        let html = if link {
                            format!("<sup class=\"fr\">{text}</sup>")
                        } else {
                            format!("<sup class=\"fr\"><a href=\"#n-{text}\">{text}</a></sup>")
                        };
                        self.cur.push(Seg { text, html: Some(html), font, size: size * SUP_SCALE, style, note: Some(g) });
                        self.brk();
                    }
                    InlineEl::Note(Err(i)) => {
                        ctx.warn.push(RenderWarning::UnresolvedNote(i));
                        self.error(font, size, style);
                    }
                }
            }
        }
    }

    fn error(&mut self, font: Font, size: f32, style: usize) {
        let html = "<span class=\"error\">??</span>".to_string();
        self.cur.push(Seg { text: "??".into(), html: Some(html), font: Font { mono: true, ..font }, size, style, note: None });
    }

    fn all_words(&self) -> impl Iterator<Item = &Word> {
        self.words.iter().chain((!self.cur.is_empty()).then_some(&self.cur))
    }
    pub(crate) fn natural_width(&self) -> f32 {
        let mut w = 0.0;
        let mut prev: Option<&Word> = None;
        for word in self.all_words() {
            if let Some(p) = prev {
                w += space_width(p);
            }
            w += word_width(word);
            prev = Some(word);
        }
        w
    }

    pub(crate) fn min_width(&self) -> f32 {
        self.all_words().map(word_width).fold(0.0, f32::max)
    }

    pub(crate) fn lines(&self, lay: Layout) -> Vec<Line> {
        let mut queue: VecDeque<Word> = self.all_words().cloned().collect();
        let mut out: Vec<(Vec<Word>, f32, f32)> = Vec::new();
        let mut cur: Vec<Word> = Vec::new();
        let mut cur_w = 0.0;
        while let Some(word) = queue.pop_front() {
            let indent = if out.is_empty() { lay.first } else { lay.rest };
            let avail = (lay.width - indent - WIDTH_SAFETY).max(1.0);
            let ww = word_width(&word);
            let sp = cur.last().map_or(0.0, space_width);
            if cur_w + sp + ww <= avail {
                cur.push(word);
                cur_w += sp + ww;
                continue;
            }
            if cur.is_empty() {
                let (head, tail) = split_word(word, avail);
                let hw = word_width(&head);
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
            let indent = if out.is_empty() { lay.first } else { lay.rest };
            out.push((cur, cur_w, (lay.width - indent - WIDTH_SAFETY).max(1.0)));
        }

        let count = out.len();
        out.into_iter().enumerate().map(|(n, (words, natural, avail))| {
            let indent = if n == 0 { lay.first } else { lay.rest };
            self.render_line(&words, natural, avail, indent, lay.align == Align::Justify && n + 1 < count)
        }).collect()
    }

    fn render_line(&self, words: &[Word], natural: f32, avail: f32, indent: f32, justify: bool) -> Line {
        let mut style = String::new();
        if indent > 0.0 {
            let _ = write!(style, "padding-left:{indent:.2}pt;");
        }
        if justify && natural < avail {
            let gaps = words.len() - 1 + words.iter()
                .flat_map(|w| w.iter())
                .filter(|s| s.html.is_none())
                .map(|s| s.text.matches('\u{a0}').count())
                .sum::<usize>();
            if gaps > 0 {
                let _ = write!(style, "word-spacing:{:.3}pt;", (avail - natural) / gaps as f32);
            }
        }
        let mut body = String::new();
        let mut notes = Vec::new();
        let mut cur = 0;
        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                body.push(' ');
            }
            for seg in word {
                if seg.style != cur {
                    body.push_str(&self.styles[cur].1);
                    body.push_str(&self.styles[seg.style].0);
                    cur = seg.style;
                }
                match &seg.html {
                    Some(h) => body.push_str(h),
                    None => {
                        let _ = escape(&mut body, &seg.text);
                    }
                }
                if let Some(g) = seg.note {
                    notes.push(g);
                }
            }
        }
        body.push_str(&self.styles[cur].1);
        Line { style, body, notes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str, w: f32, align: Align) -> Vec<Line> {
        let mut i = Inline::new(Base::new(14.0));
        i.text(text);
        i.lines(Layout { width: w, first: 0.0, rest: 0.0, align })
    }

    #[test]
    fn breaks_by_width_and_justifies_all_but_last() {
        let one = width("слово", Font::default(), 14.0);
        let space = width(" ", Font::default(), 14.0);
        let w = one * 2.0 + space + WIDTH_SAFETY + 1.0;
        let out = lines("слово слово слово слово слово", w, Align::Justify);
        let bodies: Vec<&str> = out.iter().map(|l| l.body.as_str()).collect();
        assert_eq!(bodies, vec!["слово слово", "слово слово", "слово"]);
        assert!(out[0].style.contains("word-spacing"));
        assert!(!out[2].style.contains("word-spacing"));
    }

    #[test]
    fn long_word_is_cut_by_chars() {
        let w = width("abcd", Font::default(), 14.0) + WIDTH_SAFETY + 0.1;
        let out = lines("abcdabcdab", w, Align::Left);
        let bodies: Vec<&str> = out.iter().map(|l| l.body.as_str()).collect();
        assert_eq!(bodies, vec!["abcd", "abcd", "ab"]);
    }

    #[test]
    fn escapes_and_indents() {
        let mut i = Inline::new(Base::new(14.0));
        i.text("a<b");
        let out = i.lines(Layout { width: 400.0, first: 35.0, rest: 0.0, align: Align::Justify });
        assert_eq!(out[0].html(), "<span class=\"l\" style=\"padding-left:35.00pt;\">a&lt;b</span>");
    }
}
