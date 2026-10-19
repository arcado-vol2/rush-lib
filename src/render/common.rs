
use crate::parse::rush::{Code, Comment, Formula, Header, Image, List, Node, Note, Paragraph, Rush, Span, Table};
use std::fmt::{self, Write};

pub(crate) trait Backend {
    fn span(out: &mut impl Write, inp: Vec<Span>) -> fmt::Result;

    fn paragraph(out: &mut impl Write, paragraph: Paragraph) -> fmt::Result;
    fn header(out: &mut impl Write, header: Header) -> fmt::Result;
    fn comment(out: &mut impl Write, comment: Comment) -> fmt::Result;
    fn formula(out: &mut impl Write, formula: Formula) -> fmt::Result;
    fn code(out: &mut impl Write, code: Code) -> fmt::Result;
    fn image(out: &mut impl Write, image: Image) -> fmt::Result;
    fn note(out: &mut impl Write, note: Note) -> fmt::Result;
    fn list(out: &mut impl Write, list: List) -> fmt::Result;
    fn table(out: &mut impl Write, table: Table) -> fmt::Result;

    fn block(out: &mut impl Write, node: Node) -> fmt::Result {
        match node {
            Node::Paragraph(b) => Self::paragraph(out, b),
            Node::Header(b) => Self::header(out, b),
            Node::Comment(b) => Self::comment(out, b),
            Node::Formula(b) => Self::formula(out, b),
            Node::Code(b) => Self::code(out, b),
            Node::Image(b) => Self::image(out, b),
            Node::Note(b) => Self::note(out, b),
            Node::List(b) => Self::list(out, b),
            Node::Table(b) => Self::table(out, b),
        }
    }

    fn render(file: Rush) -> Result<String, fmt::Error>;
}