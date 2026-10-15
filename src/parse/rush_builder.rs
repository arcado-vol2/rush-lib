use std::path::PathBuf;

use crate::parse::{common::*, line_parse::{Span, generate_spans}};


enum ListElKind {
    Ordered,
    Unordered
}
struct ListEl<'a> {
    kind: ListElKind,
    indent: u16,
    text: Span<'a>
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
    content: Span<'a>
}

struct Comment<'a> {
    content: Vec<Span<'a>>
}

struct Formula<'a> {
    named: Named<'a>,
    content: Vec<&'a str>
}

struct Code<'a> {
    named: Named<'a>,
    content: Vec<&'a str>
}

struct Image<'a> {
    meta: ImageKind<'a>,
    path: PathBuf
}

struct Note<'a> {
    index: u16,
    content: Span<'a>
}
 
struct List<'a> {
    content: Vec<ListEl<'a>>
}
struct Table<'a> {
    named: Named<'a>,
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

