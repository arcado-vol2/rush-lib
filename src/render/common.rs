
use crate::parse::rush::{Code, Comment, Formula, Header, Image, List, Node, Note, Paragraph, Rush, Span, Table};
use crate::resources::{FileId, Id, Resources};
use std::fmt::{self, Write};

#[derive(Debug, Clone, PartialEq)]
pub enum RenderWarning {
    UnresolvedRef,
    UnresolvedNote,
    UnresolvedImage,
    InvalidUrl(String),
    InvalidFormula(String),
}

pub(crate) trait Backend {
    fn span(out: &mut impl Write, inp: Vec<Span>, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;

    fn paragraph(out: &mut impl Write, paragraph: Paragraph, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn header(out: &mut impl Write, header: Header, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn comment(out: &mut impl Write, comment: Comment, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn formula(out: &mut impl Write, formula: Formula, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn code(out: &mut impl Write, code: Code, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn image(out: &mut impl Write, image: Image, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn note(out: &mut impl Write, note: Note, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn list(out: &mut impl Write, list: List, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn table(out: &mut impl Write, table: Table, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;

    fn block(out: &mut impl Write, node: Node, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        match node {
            Node::Paragraph(b) => Self::paragraph(out, b, res, file, warn),
            Node::Header(b) => Self::header(out, b, res, file, warn),
            Node::Comment(b) => Self::comment(out, b, res, file, warn),
            Node::Formula(b) => Self::formula(out, b, res, file, warn),
            Node::Code(b) => Self::code(out, b, res, file, warn),
            Node::Image(b) => Self::image(out, b, res, file, warn),
            Node::Note(b) => Self::note(out, b, res, file, warn),
            Node::List(b) => Self::list(out, b, res, file, warn),
            Node::Table(b) => Self::table(out, b, res, file, warn),
        }
    }

    fn render(rush: Rush, res: &Resources, file: Id<FileId>, css_href: &str) -> Result<(String, Vec<RenderWarning>), fmt::Error>;
}
