
use crate::parse::rush::{Code, Comment, Formula, Header, Image, List, Node, Note, Paragraph, Rush, Span, Table};
use crate::content::Content;
use crate::parse::common::RefKind;
use crate::resources::{FileId, Id, Resources};
use std::fmt::{self, Write};

#[derive(Debug, Clone, PartialEq)]
pub enum RenderWarning {
    UnresolvedRef { kind: RefKind, name: String },
    UnresolvedNote(u16),
    UnresolvedImage(String),
    InvalidUrl(String),
    InvalidFormula(String),
    FileNotInContent(String),
    ImageSizeUnknown(String),
    BlockTallerThanPage,
}

pub(crate) trait Backend {
    fn span(&self, out: &mut impl Write, inp: Vec<Span>, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;

    fn paragraph(&self, out: &mut impl Write, paragraph: Paragraph, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn header(&self, out: &mut impl Write, header: Header, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn comment(&self, out: &mut impl Write, comment: Comment, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn formula(&self, out: &mut impl Write, formula: Formula, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn code(&self, out: &mut impl Write, code: Code, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn image(&self, out: &mut impl Write, image: Image, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn note(&self, out: &mut impl Write, note: Note, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn list(&self, out: &mut impl Write, list: List, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;
    fn table(&self, out: &mut impl Write, table: Table, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result;

    fn block(&self, out: &mut impl Write, node: Node, res: &Resources, file: Id<FileId>, warn: &mut Vec<RenderWarning>) -> fmt::Result {
        match node {
            Node::Paragraph(b) => self.paragraph(out, b, res, file, warn),
            Node::Header(b) => self.header(out, b, res, file, warn),
            Node::Comment(b) => self.comment(out, b, res, file, warn),
            Node::Formula(b) => self.formula(out, b, res, file, warn),
            Node::Code(b) => self.code(out, b, res, file, warn),
            Node::Image(b) => self.image(out, b, res, file, warn),
            Node::Note(b) => self.note(out, b, res, file, warn),
            Node::List(b) => self.list(out, b, res, file, warn),
            Node::Table(b) => self.table(out, b, res, file, warn),
        }
    }

    fn render(&self, rush: Rush, res: &Resources, file: Id<FileId>, content: &Content) -> Result<(String, Vec<RenderWarning>), fmt::Error>;
}
