use crate::parse::common::{Color, Named, RefKind};
use crate::parse::rush::{Aligment, InlineEl, ListEl, ListElKind, Node, OuterLink, Rush, Span, TableEl};
use crate::render::warning::{Feature, RenderWarning, RenderWarningKind};
use crate::resources::{FileId, Id, ImageId, Resources};
use crate::style::{fill, Align, Font, Style};
use latex2mathml::{latex_to_mathml, DisplayStyle};
use std::collections::{BTreeSet, HashMap, HashSet};
use url::{ParseError, Url};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Anchor {
    Doc(usize),
    Header(Vec<u16>),
    Table(u16),
    Image(u16),
    Code(u16),
    Formula(u16),
    Note(u16),
}

impl Anchor {
    pub(crate) fn id(&self) -> String {
        match self {
            Anchor::Doc(k) => format!("d-{k}"),
            Anchor::Header(index) => format!("h-{}", index.iter().map(u16::to_string).collect::<Vec<_>>().join("-")),
            Anchor::Table(i) => format!("t-{i}"),
            Anchor::Image(i) => format!("i-{i}"),
            Anchor::Code(i) => format!("c-{i}"),
            Anchor::Formula(i) => format!("f-{i}"),
            Anchor::Note(i) => format!("n-{i}"),
        }
    }

    fn of_ref(kind: RefKind, index: u16) -> Self {
        match kind {
            RefKind::Table => Anchor::Table(index),
            RefKind::Code => Anchor::Code(index),
            RefKind::Formula => Anchor::Formula(index),
            RefKind::Image => Anchor::Image(index),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Link {
    Url(String),
    /// another project file: the format decides whether it is a page or an anchor
    Doc(Id<FileId>),
    Anchor(Anchor),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RunText {
    Text(String),
    /// place where the line can wrap
    Space,
    Ref(u16),
    /// global note number
    Note(u16),
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Run {
    pub(crate) text: RunText,
    pub(crate) font: Font,
    pub(crate) under: bool,
    pub(crate) strike: bool,
    pub(crate) color: Option<Color>,
    pub(crate) link: Option<Link>,
}

impl Run {
    pub(crate) fn plain(text: RunText) -> Self {
        Self { text, font: Font::default(), under: false, strike: false, color: None, link: None }
    }

    pub(crate) fn text(s: &str) -> Self {
        Self::plain(RunText::Text(s.to_string()))
    }

    /// Run text for places where markup is not needed
    pub(crate) fn label(&self) -> String {
        match &self.text {
            RunText::Text(s) => s.clone(),
            RunText::Space => " ".into(),
            RunText::Ref(i) | RunText::Note(i) => i.to_string(),
            RunText::Error => "??".into(),
        }
    }
}

pub(crate) fn plain(runs: &[Run]) -> String {
    let s: String = runs.iter().filter(|r| !matches!(r.text, RunText::Note(_))).map(Run::label).collect();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CaptionKind {
    Table,
    Image,
    Code,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Caption {
    pub(crate) kind: CaptionKind,
    /// "Таблица 1 - ", the name follows right after it
    pub(crate) label: String,
    pub(crate) name: Option<Vec<Run>>,
}

impl Caption {
    pub(crate) fn runs(&self) -> Vec<Run> {
        let mut out = vec![Run::text(&self.label)];
        if let Some(name) = &self.name {
            out.extend(name.iter().cloned());
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParaKind {
    Body,
    Comment,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ListItem {
    pub(crate) level: usize,
    pub(crate) marker: String,
    pub(crate) runs: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Cell {
    Text { col: usize, w: usize, h: usize, runs: Vec<Run> },
    /// part of a merged area without its own text
    Empty { col: usize, join_top: bool, join_left: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NoteDef {
    pub(crate) g: u16,
    pub(crate) runs: Vec<Run>,
    /// nothing refers to the note: it stays where it is defined
    pub(crate) anchored: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Block {
    Para { kind: ParaKind, runs: Vec<Run> },
    Header { level: u8, number: Option<String>, runs: Vec<Run>, anchor: Anchor },
    List(Vec<ListItem>),
    Figure { caption: Caption, image: Option<Id<ImageId>>, scale: u16, anchor: Anchor },
    Formula { name: Option<Vec<Run>>, mathml: Option<String>, src: String, number: String, anchor: Anchor },
    Code { caption: Caption, lines: Vec<String>, anchor: Anchor },
    Table { caption: Caption, cont: String, aligns: Vec<Align>, head: Vec<Vec<Cell>>, body: Vec<Vec<Cell>>, anchor: Anchor },
    Notes(Vec<NoteDef>),
}

#[derive(Debug, PartialEq)]
pub(crate) struct Doc {
    pub(crate) file: Id<FileId>,
    pub(crate) title: String,
    pub(crate) anchor: Anchor,
    pub(crate) blocks: Vec<Block>,
    pub(crate) used: BTreeSet<Feature>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TocEntry {
    pub(crate) level: u8,
    pub(crate) text: String,
    pub(crate) anchor: Anchor,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct Project {
    pub(crate) docs: Vec<Doc>,
    pub(crate) toc: Vec<TocEntry>,
}

pub(crate) struct Source<'a> {
    pub(crate) file: Id<FileId>,
    pub(crate) title: &'a str,
    pub(crate) rush: Rush<'a>,
}

#[derive(Debug, Default)]
struct Notes {
    local: HashMap<u16, u16>,
    last: u16,
}

impl Notes {
    fn global(&mut self, local: u16) -> u16 {
        if let Some(&g) = self.local.get(&local) {
            return g;
        }
        let g = self.fresh();
        self.local.insert(local, g);
        g
    }

    fn fresh(&mut self) -> u16 {
        self.last = self.last.saturating_add(1);
        self.last
    }
}

pub(crate) fn parse_url(link: &str) -> Option<Url> {
    match Url::parse(link) {
        Ok(url) => Some(url),
        Err(ParseError::RelativeUrlWithoutBase) => Url::parse(&format!("https://{link}")).ok(),
        Err(_) => None,
    }
}

fn align(a: Aligment) -> Align {
    match a {
        Aligment::Left => Align::Left,
        Aligment::Center => Align::Center,
        Aligment::Right => Align::Right,
    }
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

enum Slot<'a> {
    Cell { col: usize, w: usize, h: usize, content: Vec<Span<'a>> },
    Empty { col: usize, join_top: bool, join_left: bool },
}

/// Combines cells marked for merging into rectangular areas
fn merge_cells(mut rows: Vec<Vec<TableEl>>, cols: usize) -> Vec<Vec<Slot>> {
    let n = rows.len() * cols;

    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let mut parent: Vec<usize> = (0..n).collect();
    for (r, row) in rows.iter().enumerate() {
        for (c, el) in row.iter().enumerate() {
            let (up, left) = match el {
                TableEl::MergeUp => (true, false),
                TableEl::MergeLeft => (false, true),
                TableEl::MergeUpLeft => (true, true),
                TableEl::Cell(_) => (false, false),
            };
            let i = r * cols + c;
            if up {
                let (a, b) = (find(&mut parent, i), find(&mut parent, i - cols));
                parent[a] = b;
            }
            if left {
                let (a, b) = (find(&mut parent, i), find(&mut parent, i - 1));
                parent[a] = b;
            }
        }
    }
    let region: Vec<usize> = (0..n).map(|i| find(&mut parent, i)).collect();

    let mut taken = vec![false; n];
    let joinable = |taken: &[bool], rows: &[Vec<TableEl>], r: usize, c: usize, anchor: usize| {
        let i = r * cols + c;
        !taken[i] && region[i] == region[anchor] && !matches!(rows[r][c], TableEl::Cell(_))
    };

    let mut out = Vec::with_capacity(rows.len());
    for r in 0..rows.len() {
        let mut row = Vec::new();
        for c in 0..cols {
            let i = r * cols + c;
            if taken[i] {
                continue;
            }
            if let TableEl::Cell(content) = &mut rows[r][c] {
                let content = std::mem::take(content);
                let mut w = 1;
                while c + w < cols && joinable(&taken, &rows, r, c + w, i) {
                    w += 1;
                }
                let mut h = 1;
                while r + h < rows.len() && (0..w).all(|k| joinable(&taken, &rows, r + h, c + k, i)) {
                    h += 1;
                }
                for dr in 0..h {
                    for dc in 0..w {
                        taken[(r + dr) * cols + c + dc] = true;
                    }
                }
                row.push(Slot::Cell { col: c, w, h, content });
            } else {
                taken[i] = true;
                row.push(Slot::Empty {
                    col: c,
                    join_top: r > 0 && region[i - cols] == region[i],
                    join_left: c > 0 && region[i - 1] == region[i],
                });
            }
        }
        out.push(row);
    }
    out
}

struct Ctx<'c> {
    style: &'c Style,
    file: String,
    notes: Notes,
    used: BTreeSet<Feature>,
    warn: &'c mut Vec<RenderWarning>,
}

impl Ctx<'_> {
    fn warn(&mut self, kind: RenderWarningKind) {
        self.warn.push(RenderWarning::content(&self.file, kind));
    }

    fn runs(&mut self, spans: Vec<Span>) -> Vec<Run> {
        let mut out = Vec::new();
        for span in spans {
            let st = span.style;
            let font = Font { mono: st.is_mono, bold: st.is_bold, italic: st.is_italic };
            for (on, feature) in [(st.color.is_some(), Feature::Color), (st.is_under, Feature::Underline), (st.is_strike, Feature::Strike)] {
                if on {
                    self.used.insert(feature);
                }
            }
            let link = match st.link {
                Some(OuterLink::URL(raw)) => match parse_url(raw) {
                    Some(url) => Some(Link::Url(url.to_string())),
                    None => {
                        self.warn(RenderWarningKind::InvalidUrl(raw.to_string()));
                        None
                    }
                },
                Some(OuterLink::File(id)) => Some(Link::Doc(id)),
                None => None,
            };
            let run = |text: RunText, link: Option<Link>| Run { text, font, under: st.is_under, strike: st.is_strike, color: st.color.clone(), link };
            for el in span.text {
                let r = match el {
                    InlineEl::String(s) => run(RunText::Text(s.to_string()), link.clone()),
                    InlineEl::Space => run(RunText::Space, link.clone()),
                    InlineEl::LongDash => run(RunText::Text("—".into()), link.clone()),
                    InlineEl::InnerLink(Ok(l)) => {
                        let own = Link::Anchor(Anchor::of_ref(l.kind, l.index));
                        run(RunText::Ref(l.index), link.clone().or(Some(own)))
                    }
                    InlineEl::InnerLink(Err((name, kind))) => {
                        self.warn(RenderWarningKind::UnresolvedRef { kind, name: name.into_owned() });
                        run(RunText::Error, link.clone())
                    }
                    InlineEl::Note(Ok(i)) => {
                        self.used.insert(Feature::Notes);
                        let g = self.notes.global(i);
                        run(RunText::Note(g), link.clone().or(Some(Link::Anchor(Anchor::Note(g)))))
                    }
                    InlineEl::Note(Err(i)) => {
                        self.warn(RenderWarningKind::UnresolvedNote(i));
                        run(RunText::Error, link.clone())
                    }
                };
                out.push(r);
            }
        }
        out
    }

    fn caption(&mut self, kind: CaptionKind, named: Named<Vec<Span>>) -> (Caption, u16) {
        let tpl = match kind {
            CaptionKind::Table => &self.style.captions.table,
            CaptionKind::Image => &self.style.captions.image,
            CaptionKind::Code => &self.style.captions.code,
        };
        match named {
            Named::WithName { name, index } => {
                let label = fill(tpl.named, index);
                (Caption { kind, label, name: Some(self.runs(name)) }, index)
            }
            Named::Anonymous { index } => (Caption { kind, label: fill(tpl.anonymous, index), name: None }, index),
        }
    }

    fn list(&mut self, items: Vec<ListEl>, level: usize, out: &mut Vec<ListItem>) {
        let mut num: u16 = 0;
        for el in items {
            let item = match el.this {
                Some(ListElKind::Ordered(spans)) => {
                    num += 1;
                    Some((fill(self.style.markers.ordered, num), spans))
                }
                Some(ListElKind::Unordered(spans)) => {
                    num = 0;
                    Some((self.style.markers.unordered.to_string(), spans))
                }
                None => None,
            };
            if let Some((marker, spans)) = item {
                let runs = self.runs(spans);
                out.push(ListItem { level, marker, runs });
            }
            self.list(el.children, level + 1, out);
        }
    }

    fn table(&mut self, rows: Vec<Vec<TableEl>>, cols: usize) -> Vec<Vec<Cell>> {
        merge_cells(rows, cols).into_iter().map(|row| row.into_iter().map(|slot| match slot {
            Slot::Cell { col, w, h, content } => {
                if w > 1 || h > 1 {
                    self.used.insert(Feature::MergedCells);
                }
                Cell::Text { col, w, h, runs: self.runs(content) }
            }
            Slot::Empty { col, join_top, join_left } => {
                self.used.insert(Feature::MergedCells);
                Cell::Empty { col, join_top, join_left }
            }
        }).collect()).collect()
    }
}

/// Deferred note: its text is parsed after the whole document, when its global number is known
enum PendingNote {
    Done(NoteDef),
    Later(u16),
}

enum Pending {
    Block(Block),
    Notes(Vec<PendingNote>),
}

fn doc(k: usize, src: Source, ctx: &mut Ctx, toc: &mut Vec<TocEntry>) -> Doc {
    let style = ctx.style;
    let toc_level = style.toc.map(|t| t.max_level);
    let anchor = Anchor::Doc(k);
    if toc_level.is_some() {
        toc.push(TocEntry { level: 1, text: src.title.to_string(), anchor: anchor.clone() });
    }

    let referenced = note_refs(&src.rush.nodes);
    let mut defs: HashMap<u16, Vec<Span>> = HashMap::new();
    let mut blocks = Vec::new();
    for node in src.rush.nodes {
        let block = match node {
            Node::Paragraph(p) => Block::Para { kind: ParaKind::Body, runs: ctx.runs(p.content) },
            Node::Comment(c) => {
                let mut runs = Vec::new();
                if let Some(prefix) = style.comment_prefix {
                    runs.push(Run::text(prefix));
                    runs.push(Run::plain(RunText::Space));
                }
                runs.extend(ctx.runs(c.content));
                Block::Para { kind: ParaKind::Comment, runs }
            }
            Node::Header(h) => {
                let level = h.level.clamp(1, 5);
                let index = &h.index[..level as usize];
                let number = style.numbering.number(level, index);
                let anchor = Anchor::Header(index.to_vec());
                let runs = ctx.runs(h.content);
                if toc_level.is_some_and(|max| level <= max) {
                    let text = match &number {
                        Some(n) => format!("{n} {}", plain(&runs)),
                        None => plain(&runs),
                    };
                    toc.push(TocEntry { level, text, anchor: anchor.clone() });
                }
                Block::Header { level, number, runs, anchor }
            }
            Node::List(l) => {
                let mut items = Vec::new();
                ctx.list(l.content, 0, &mut items);
                Block::List(items)
            }
            Node::Image(i) => {
                let scale = i.meta.scale;
                if scale != 100 {
                    ctx.used.insert(Feature::ImageScale);
                }
                let (caption, index) = ctx.caption(CaptionKind::Image, i.meta.named);
                let image = match i.path {
                    Ok(id) => Some(id),
                    Err(path) => {
                        ctx.warn(RenderWarningKind::UnresolvedImage(path.into_owned()));
                        None
                    }
                };
                Block::Figure { caption, image, scale, anchor: Anchor::Image(index) }
            }
            Node::Formula(f) => {
                ctx.used.insert(Feature::Formula);
                let (index, name) = match f.named {
                    Named::WithName { name, index } => (index, Some(ctx.runs(name))),
                    Named::Anonymous { index } => (index, None),
                };
                let src = f.content.join("\n");
                let mathml = match latex_to_mathml(&src, DisplayStyle::Block) {
                    Ok(m) => Some(m),
                    Err(e) => {
                        ctx.warn(RenderWarningKind::InvalidFormula(e.to_string()));
                        None
                    }
                };
                let number = fill(style.captions.formula, index);
                Block::Formula { name, mathml, src, number, anchor: Anchor::Formula(index) }
            }
            Node::Code(c) => {
                let (caption, index) = ctx.caption(CaptionKind::Code, c.named);
                let lines = c.content.iter().map(|l| l.to_string()).collect();
                Block::Code { caption, lines, anchor: Anchor::Code(index) }
            }
            Node::Table(t) => {
                let cols = t.aligments.len();
                if cols == 0 {
                    continue;
                }
                let (caption, index) = ctx.caption(CaptionKind::Table, t.named);
                let head = ctx.table(t.headers, cols);
                let body = ctx.table(t.cells, cols);
                Block::Table {
                    caption,
                    cont: fill(style.captions.table_cont, index),
                    aligns: t.aligments.into_iter().map(align).collect(),
                    head,
                    body,
                    anchor: Anchor::Table(index),
                }
            }
            Node::Note(n) => {
                ctx.used.insert(Feature::Notes);
                let mut items = Vec::new();
                for item in n.content {
                    if item.index != 0 && referenced.contains(&item.index) {
                        defs.insert(item.index, item.content);
                        items.push(PendingNote::Later(item.index));
                        continue;
                    }
                    let g = if item.index == 0 { ctx.notes.fresh() } else { ctx.notes.global(item.index) };
                    items.push(PendingNote::Done(NoteDef { g, runs: ctx.runs(item.content), anchored: true }));
                }
                blocks.push(Pending::Notes(items));
                continue;
            }
        };
        blocks.push(Pending::Block(block));
    }

    let mut resolved: HashMap<u16, (u16, Vec<Run>)> = HashMap::new();
    loop {
        let ready: Vec<(u16, u16)> = ctx.notes.local.iter()
            .filter(|(l, _)| defs.contains_key(l))
            .map(|(&l, &g)| (l, g))
            .collect();
        if ready.is_empty() {
            break;
        }
        for (l, g) in ready {
            let spans = defs.remove(&l).unwrap();
            let runs = ctx.runs(spans);
            resolved.insert(l, (g, runs));
        }
    }

    let blocks = blocks.into_iter().filter_map(|p| match p {
        Pending::Block(b) => Some(b),
        Pending::Notes(items) => {
            let defs: Vec<NoteDef> = items.into_iter().filter_map(|n| match n {
                PendingNote::Done(def) => Some(def),
                PendingNote::Later(l) => resolved.remove(&l).map(|(g, runs)| NoteDef { g, runs, anchored: false }),
            }).collect();
            (!defs.is_empty()).then_some(Block::Notes(defs))
        }
    }).collect();

    Doc { file: src.file, title: src.title.to_string(), anchor, blocks, used: std::mem::take(&mut ctx.used) }
}

/// Documents go one after another: notes are numbered across all of them
pub(crate) fn build(sources: Vec<Source>, style: &Style, res: &Resources, warn: &mut Vec<RenderWarning>) -> Project {
    let mut project = Project::default();
    let mut last_note = 0;
    for (k, src) in sources.into_iter().enumerate() {
        let file = res.files.local_path(src.file).to_string_lossy().into_owned();
        let mut ctx = Ctx { style, file, notes: Notes { local: HashMap::new(), last: last_note }, used: BTreeSet::new(), warn: &mut *warn };
        let d = doc(k + 1, src, &mut ctx, &mut project.toc);
        last_note = ctx.notes.last;
        project.docs.push(d);
    }
    project
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::config::DocStyle;
    use crate::project::Project as Proj;

    pub(crate) fn project_of(src: &str) -> (Proj, Project, Vec<RenderWarning>) {
        let mut p = Proj::new();
        p.add_file("doc", "doc.rush", src.to_string()).unwrap();
        let res = &p.resources;
        let style = Style::of(DocStyle::Gost);
        let rush = Rush::new(res.files.data(Id::new(0)), res);
        let mut warn = Vec::new();
        let project = build(vec![Source { file: Id::new(0), title: "Документ", rush }], &style, res, &mut warn);
        (p, project, warn)
    }

    #[test]
    fn header_number_and_toc() {
        let (_, p, _) = project_of("\\++ Раздел\n\nтекст\n\n\\+++ Подраздел\n\n\\++++ Пункт");
        let toc: Vec<(u8, &str)> = p.toc.iter().map(|e| (e.level, e.text.as_str())).collect();
        assert_eq!(toc, vec![(1, "Документ"), (2, "1 Раздел"), (3, "1.1 Подраздел")]);
        let numbers: Vec<Option<&str>> = p.docs[0].blocks.iter().filter_map(|b| match b {
            Block::Header { number, .. } => Some(number.as_deref()),
            _ => None,
        }).collect();
        assert_eq!(numbers, vec![Some("1"), Some("1.1"), Some("1.1.1")]);
    }

    #[test]
    fn comment_gets_prefix() {
        let (_, p, _) = project_of("\\\"\nтекст\n\\\"");
        let Block::Para { kind: ParaKind::Comment, runs } = &p.docs[0].blocks[0] else { panic!() };
        assert_eq!(plain(runs), "Комментарий: текст");
    }

    #[test]
    fn notes_numbered_by_reference() {
        let (_, p, _) = project_of("а\\*5 б\n\n\\**5 сноска\n\\**0 без ссылки");
        let b = &p.docs[0].blocks;
        let Block::Para { runs, .. } = &b[0] else { panic!() };
        assert!(runs.iter().any(|r| r.text == RunText::Note(1)));
        let Block::Notes(defs) = &b[1] else { panic!() };
        let got: Vec<(u16, bool)> = defs.iter().map(|d| (d.g, d.anchored)).collect();
        assert_eq!(got, vec![(1, false), (2, true)]);
        assert_eq!(plain(&defs[0].runs), "сноска");
    }

    #[test]
    fn captions_and_continuation() {
        let (_, p, _) = project_of("\\№ Т\n/ a / b /\n\\№-\n/ 1 / 2 /\n/ \\= / 3 /\n\\№");
        let Block::Table { caption, cont, body, .. } = &p.docs[0].blocks[0] else { panic!() };
        assert_eq!(caption.label, "Таблица\u{a0}1\u{a0}—\u{a0}");
        assert_eq!(cont, "Продолжение таблицы 1");
        assert!(matches!(body[0][0], Cell::Text { h: 2, .. }));
        assert!(p.docs[0].used.contains(&Feature::MergedCells));
    }

    #[test]
    fn unresolved_note_warns_with_file() {
        let (_, _, warn) = project_of("см.\\*9 текст");
        assert_eq!(warn, vec![RenderWarning::content("doc.rush", RenderWarningKind::UnresolvedNote(9))]);
    }
}
