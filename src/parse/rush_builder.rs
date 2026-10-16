use std::{path::PathBuf, vec};
use std::fmt;

use crate::parse::{block_fragmentation::{Block, BlockKind, InnerProjectLinks, split_blocks}, common::*, lexemas::{BULLET_LIST, MARK, NUMERIC_LIST, TAB, TAB_WIDTH, TABLE_FUL_SEP, TABLE_MERGE_LEFT_TAG, TABLE_MERGE_UP_LEFT_TAG, TABLE_MERGE_UP_TAG, TABLE_SEP, is_blank}, line_parse::{InlinePreEl, PreSpan, PreStyle, PreparsedLines, generate_pre_spans}, link_project::GlobalProject};
#[derive(Debug)]
enum OuterLink<'a> {
    URL(&'a str),
    File(PathBuf)
}
#[derive(Debug)]
struct InnerLink {
    index: u16,
    kind: RefKind
}
#[derive(Debug)]
enum InlineEl<'a> {
    String(&'a str),
    InnerLink(Option<InnerLink>),
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
impl<'a> fmt::Debug for Style<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bools = [
            self.is_bold,
            self.is_italic,
            self.is_mono,
            self.is_strike,
            self.is_under,
        ]
        .iter()
        .map(|&b| if b { '1' } else { '0' })
        .collect::<String>();

        f.debug_struct("Style")
            .field("bools", &bools)
            .field("color", &self.color)
            .field("link", &self.link)
            .finish()
    }
}



#[derive(Debug)]
struct Span<'a> {
    text: Vec<InlineEl<'a>>,
    style: Style<'a>
}

impl<'a> Span<'a> {
    fn from_pre_span(pre: PreSpan<'a>, outer_links: &GlobalProject, inner_links: &InnerProjectLinks) -> Self {
        let PreStyle {is_bold, is_italic, is_mono, is_strike, is_under, color, link} = pre.style;
        println!("Color {color:?}");
        let link: Option<OuterLink> = match link {
            Some(pre_link) => {
                match outer_links.get_file(pre_link) {
                    Some(path) => Some(OuterLink::File(path)),
                    None => Some(OuterLink::URL(pre_link))
                }
            }
            None => None
        };
        let style = Style { is_bold, is_italic, is_mono, is_strike, is_under, color, link};
        let text = pre.text.into_iter()
        .map(|el| match el {
            InlinePreEl::String(s) => InlineEl::String(s),
            InlinePreEl::Space => InlineEl::Space,
            InlinePreEl::InnerLink(s, k) => InlineEl::InnerLink(
                inner_links.resolve(k, &s).map(|i| InnerLink { index: i, kind: k })
            ),
            InlinePreEl::Note(n) => InlineEl::Note(inner_links.has_note(n).then_some(n))
        }).collect();
        Self { text, style }
    }
}



fn parse_generic_block<'a> (lines: Vec<&'a str>, outer_links: &GlobalProject, inner_links: &InnerProjectLinks) -> Vec<Span<'a>>{
    let preparsed = PreparsedLines::new(lines);
    let pre_spans = generate_pre_spans(preparsed);
    let mut content = Vec::new();
    for pre in pre_spans {
        content.push(Span::from_pre_span(pre, outer_links, inner_links));
    }
    content
}


#[derive(Debug)]
struct Paragraph<'a> {
    content: Vec<Span<'a>>
}


#[derive(Debug)]
struct Header<'a> {
    level: u8,
    content: Vec<Span<'a>>
}

#[derive(Debug)]
struct Comment<'a> {
    content: Vec<Span<'a>>
}
#[derive(Debug)]
struct Formula<'a> {
    named: Named<Vec<Span<'a>>>,
    content: Vec<&'a str>
}
#[derive(Debug)]
struct Code<'a> {
    named: Named<Vec<Span<'a>>>,
    content: Vec<&'a str>
}
#[derive(Debug)]
struct Image<'a> {
    meta: ImageKind<Vec<Span<'a>>>,
    path: Option<PathBuf>
}
#[derive(Debug)]
struct Note<'a> {
    index: u16,
    content: Vec<Span<'a>>
}
#[derive(Debug)]
enum ListElKind<'a> {
    Ordered(Vec<Span<'a>>),
    Unordered(Vec<Span<'a>>)
}
#[derive(Debug)]
struct ListEl<'a> {
    this: Option<ListElKind<'a>>,
    children: Vec<ListEl<'a>>
}
 #[derive(Debug)]
struct List<'a> {
    content: Vec<ListEl<'a>>
}
impl<'a> List<'a> {
    fn build(out: &mut Vec<ListEl<'a>>,  lines: &[&'a str], outer_links: &GlobalProject, inner_links: &InnerProjectLinks, level: u16) -> usize {
        let mut i =0;
        while i < lines.len() {
            let (l_text, l_level, l_is_bullet) = Self::parse_line(lines[i]);
            if l_level > level {
                if out.is_empty() {
                    out.push(ListEl { this: None, children: Vec::new() });
                } else {
                    let j = Self::build(&mut out.last_mut().unwrap().children , &lines[i..], outer_links, inner_links, level + 1);
                    i += j;
                }
            } else if l_level == level {
                let spans = parse_generic_block(vec![l_text], outer_links, inner_links);
                if l_is_bullet {
                    out.push(
                    ListEl {
                         this: Some(ListElKind::Unordered(spans)), 
                         children: Vec::new() 
                    });
                } else {
                    out.push(
                    ListEl {
                         this: Some(ListElKind::Ordered(spans)), 
                         children: Vec::new() 
                    });
                }
                i+=1;
            } else {
                return i;
            }
        }
        i
    }

    /// true - Bullet
    /// false - Numeric
    fn parse_line(line: &'a str) -> (&'a str, u16, bool) {
        let rest = &line[MARK.len_utf8()..];
        let body = rest.trim_start_matches(is_blank);
        let level = rest[..rest.len() - body.len()]
            .chars()
            .map(|c| if c == TAB { TAB_WIDTH } else { 1 })
            .sum::<u16>();
        let (kind, text) = if let Some(s) = body.strip_prefix(BULLET_LIST) {
            (true, s.trim_start_matches(is_blank))
        } else if let Some(s) = body.strip_prefix(NUMERIC_LIST) {
            (false, s.trim_start_matches(is_blank))
        } else {
            unreachable!("not list element in list block")
        };
        (text, level, kind)
    }
}

#[derive(Debug)]
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
#[derive(Debug)]
enum TableEl<'a> {
    Cell(Vec<Span<'a>>),
    MergeLeft,
    MergeUp,
    MergeUpLeft
}


#[derive(Debug)]
struct Table<'a> {
    named: Named<Vec<Span<'a>>>,
    headers: Vec<Vec<TableEl<'a>>>,
    aligments: Vec<Aligment>,
    cells: Vec<Vec<TableEl<'a>>>,
}
impl<'a> Table<'a> {
    fn new(lines: Vec<&'a str>, outer_links: &GlobalProject, inner_links: &InnerProjectLinks) -> Self {
        let mut lines = lines.iter().peekable();
        while let Some(line) = lines.peek() && line.strip_prefix(|s| TABLE_FUL_SEP.is(s)).is_some() {
            let line = lines.next().unwrap();
            let line = Self::parse_table_line(line, outer_links, inner_links);
            println!("head {line:?}");
        }
        println!("Sep: {:?}", lines.next());
        while let Some(line) = lines.next(){
            println!("bode: {line:?}");
        }
        unimplemented!("no");
    }


    fn parse_table_line(line: &'a str, outer_links: &GlobalProject, inner_links: &InnerProjectLinks) -> Vec<TableEl<'a>> {
        line.split(TABLE_SEP).map(
            |s| {
                let raw = s.trim_matches(|c| is_blank(c));
                if TABLE_MERGE_LEFT_TAG.is(raw) {
                    TableEl::MergeLeft
                } else if TABLE_MERGE_UP_TAG.is(raw) {
                    TableEl::MergeUp
                } else if TABLE_MERGE_UP_LEFT_TAG.is(raw) {
                    TableEl::MergeUpLeft
                } else {
                    TableEl::Cell(parse_generic_block(vec![raw], outer_links, inner_links))
                }
            }
        ).collect()
    }
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
impl<'a> Node<'a> {
    fn build_from_block(block: Block<'a>, outer_links: &GlobalProject, inner_links: &InnerProjectLinks) -> Self {
        match block.kind {
            BlockKind::Paragraph => {
                let spans = parse_generic_block(block.lines, outer_links, inner_links);
                Self::Paragraph(Paragraph {content: spans})
            },
            BlockKind::Comment => {
                let spans = parse_generic_block(block.lines, outer_links, inner_links);
                Self::Comment(Comment {content: spans})
            },
            BlockKind::Header(l) => {
                let spans = parse_generic_block(block.lines, outer_links, inner_links);
                Self::Header( Header{content: spans, level: l})
            },
            BlockKind::Note(i) => {
                let spans = parse_generic_block(block.lines, outer_links, inner_links);
                Self::Note(Note {index: i, content: spans})
            },
            BlockKind::Code(name) => {
                Self::Code(Code {
                    named: name.map(|s| parse_generic_block(vec![s], outer_links, inner_links)),
                    content: block.lines
                })
            },
            BlockKind::Formula(name) => {
                Self::Formula(Formula {
                    named: name.map(|s| parse_generic_block(vec![s], outer_links, inner_links)),
                    content: block.lines
                })
            },
            BlockKind::Image(img) => {
                Self::Image(Image {
                     meta: img.map(|s| parse_generic_block(vec![s], outer_links, inner_links)),
                     path: block.lines.first().and_then(|s| outer_links.get_image(s))
                })
            },
            BlockKind::List => {
                let mut out = Vec::new();
                List::build(&mut out, &block.lines, outer_links, inner_links, 0);
                Self::List(List { content: out })
            },
            BlockKind::Table(name) => {
                Self::Table(Table::new(block.lines, outer_links, inner_links))
            }

            _ => unimplemented!("not impl")
        }
    }
}

struct Lines<'b, T>(&'b [T]);

impl<'b, T: fmt::Debug> fmt::Debug for Lines<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[\n")?;
        for el in self.0 {
            writeln!(f, "    {:?},", el)?;
        }
        f.write_str("  ]")
    }
}

impl<'a> fmt::Debug for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Paragraph(p) => f.debug_struct("Paragraph")
                .field("content", &Lines(&p.content))
                .finish(),

            Node::Header(h) => f.debug_struct("Header")
                .field("level", &h.level)
                .field("content", &Lines(&h.content))
                .finish(),

            Node::Comment(c) => f.debug_struct("Comment")
                .field("content", &Lines(&c.content))
                .finish(),

            Node::Formula(v) => f.debug_struct("Formula")
                .field("named", &v.named)
                .field("content", &Lines(&v.content))
                .finish(),

            Node::Code(v) => f.debug_struct("Code")
                .field("named", &v.named)
                .field("content", &Lines(&v.content))
                .finish(),

            Node::Image(v) => f.debug_tuple("Image")
                .field(v)
                .finish(),

            Node::Note(v) => f.debug_struct("Note")
                .field("index", &v.index)
                .field("content", &Lines(&v.content))
                .finish(),

            Node::List(v) => f.debug_struct("List")
                .field("content", &Lines(&v.content))
                .finish(),

            Node::Table(v) => f.debug_tuple("Table")
                .field(v)
                .finish(),
        }
    }
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
            Node::Header(b) => self.header(b),
            Node::Comment(b) => self.comment(b),
            Node::Formula(b) => self.formula(b),
            Node::Code(b) => self.code(b),
            Node::Image(b) => self.image(b),
            Node::Note(b) => self.note(b),
            Node::List(b) => self.list(b),
            Node::Table(b) => self.table(b),
        }
    }
}

pub(crate) struct Rush<'a> {
    nodes: Vec<Node<'a>>
}

impl<'a> fmt::Debug for Rush<'a>
where
    Node<'a>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Rush {\n")?;
        for node in &self.nodes {
            writeln!(f, "  {:?}", node)?;
        }
        f.write_str("}")
    }
}

impl<'a> Rush<'a> {
    pub(crate) fn new(source: &'a str,  global_project: &GlobalProject) -> Self {
        let lexed_project = split_blocks(source);
        println!("lexed: {lexed_project:?}");
        println!();
        println!();
        let mut out = Self {nodes: Vec::new()};

        for block in lexed_project.blocks {
            out.nodes.push(Node::build_from_block(block, global_project, &lexed_project.links));
        }


        out
    }
}


#[cfg(test)] mod test {
    use std::collections::HashMap;
    use crate::parse::{rush_builder::Rush, link_project::GlobalProject};

    fn empty_gp() -> GlobalProject {
        GlobalProject {
            images: HashMap::new(),
            files: HashMap::new()
        }
    }

    #[test]
    fn main() {
        let out = Rush::new("\\№ 0\n / Заг 1 / Заг 2 / \n \\№-\n / Яч 11 / Яч 12 /\n / Яч 21 / Яч 22 /\n\\№", &empty_gp());
        println!("{out:?}");
    }
} 