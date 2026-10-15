use std::path::PathBuf;

use crate::parse::{block_fragmentation::FlatLexedProject, common::*, line_parse::{InlinePreEl, PreSpan, generate_spans}, link_project::GlobalProject};

 

enum OuterLink<'a> {
    URL(&'a str),
    File(PathBuf)
}

enum InlineEl<'a> {
    String(&'a str),
    InnerLink(Option<u16>),
    Note(Option<u16>),
    Space
}


struct Style<'a> {
    is_bold: bool,
    is_italic: bool,
    is_mono: bool,
    is_strike: bool,
    is_under: bool,
    color: Option<Color>,
    link: Option<OuterLink<'a>>,
}


struct Span<'a> {
    text: Vec<InlineEl<'a>>,
    style: Style<'a>
}

impl<'a> Span<'a> {
    fn from_pre_span(pre: PreSpan<'a>, outer_links: &GlobalProject, inner_links: &FlatLexedProject) -> Self {
        
        let link: Option<OuterLink> = match pre.style.link {
            Some(pre_link) => {
                match outer_links.files.get(pre_link) {
                    Some(path) => Some(OuterLink::File(path.clone())),
                    None => Some(OuterLink::URL(pre_link))
                }
            }
            None => None
        };
        let style = Style {
            is_bold: pre.style.is_bold,
            is_italic: pre.style.is_italic,
            is_mono: pre.style.is_mono,
            is_strike: pre.style.is_strike,
            is_under: pre.style.is_under,
            color: pre.style.color,
            link: link
        };

        let text = pre.text.into_iter()
        .map(|el| match el {
            InlinePreEl::String(s) => InlineEl::String(s),
            InlinePreEl::Space => InlineEl::Space,
            InlinePreEl::InnerLink(s, k) => {
                match k {
                    RefKind::Code => InlineEl::InnerLink(inner_links.code.data.get(&s).copied()),
                    RefKind::Formula =>  InlineEl::InnerLink(inner_links.formulas.data.get(&s).copied()),
                    RefKind::Table =>  InlineEl::InnerLink(inner_links.tables.data.get(&s).copied()),
                    RefKind::Image =>  InlineEl::InnerLink(inner_links.images.data.get(&s).copied()),
                }
            },
            InlinePreEl::Note(n) => InlineEl::Note(inner_links.notes.get(&n).copied())
        }).collect();
        Self { text, style }
    }
}



enum ListElKind<'a> {
    Ordered(Vec<Span<'a>>),
    Unordered(Vec<Span<'a>>)
}



struct ListEl<'a> {
    this: Option<ListElKind<'a>>,
    children: Vec<ListEl<'a>>
}


enum Aligment {
    Left,
    Center,
    Right
}
impl Default for Aligment {
    fn default() -> Self {
        Self::Center
    }
}

enum TableEl<'a> {
    Cell(Span<'a>),
    MergeLeft,
    MergeUp,
    MergeUpLeft
}


struct Paragraph<'a> {
    content: Vec<Span<'a>>
}
struct Header<'a> {
    level: u8,
    content: Vec<Span<'a>>
}

struct Comment<'a> {
    content: Vec<Span<'a>>
}

struct Formula<'a> {
    named: Named<Vec<Span<'a>>>,
    content: Vec<&'a str>
}

struct Code<'a> {
    named: Named<Vec<Span<'a>>>,
    content: Vec<&'a str>
}

struct Image<'a> {
    meta: ImageKind<Vec<Span<'a>>>,
    path: Option<PathBuf>
}

struct Note<'a> {
    index: u16,
    content: Vec<Span<'a>>
}
 
struct List<'a> {
    content: Vec<ListEl<'a>>
}
struct Table<'a> {
    named: Named<Vec<Span<'a>>>,
    headers: Vec<Vec<TableEl<'a>>>,
    aligments: Vec<Aligment>,
    cells: Vec<Vec<TableEl<'a>>>,
}

enum Node<'a> {
    Paragraph(Paragraph<'a>),
    Header(Header<'a>),
    Comment(Comment<'a>),
    Formula(Formula<'a>),
    Code(Code<'a>),
    Image(Image<'a>),
    Note(Note<'a>),
    List(List<'a>),
    Table(Table<'a>)
}


trait Backend {
    fn paragraph(&mut self, paragraph: &Paragraph);
    fn header(&mut self, header: &Header);
    fn comment(&mut self, comment: &Comment);
    fn formula(&mut self, formula: &Formula);
    fn code(&mut self, code: &Code);
    fn image(&mut self, image: &Image);
    fn note(&mut self, note: &Note);
    fn list(&mut self, list: &List);
    fn table(&mut self, table: &Table);

    fn block(&mut self, node: &Node) {
        match node {
            Node::Paragraph(b) => self.paragraph(b),
            Node::Header(b)    => self.header(b),
            Node::Comment(b)   => self.comment(b),
            Node::Formula(b)   => self.formula(b),
            Node::Code(b)      => self.code(b),
            Node::Image(b)     => self.image(b),
            Node::Note(b)      => self.note(b),
            Node::List(b)      => self.list(b),
            Node::Table(b)     => self.table(b),
        }
    }
}

