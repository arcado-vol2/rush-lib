use std::{borrow::Cow, vec};
use std::fmt;

use crate::parse::lexemas::{TABLE_CENTER_ALT, TABLE_FUL_SEP_ALT, TABLE_LEFT_ALT, TABLE_RIGHT_ALT};
use crate::parse::{block_fragmentation::{Block, BlockKind, Counters, InnerProjectLinks, split_blocks_from, unescape_image_path}, common::*, lexemas::{BULLET_LIST, MARK, NUMERIC_LIST, TAB, TAB_WIDTH, TABLE_CENTER, TABLE_FUL_SEP, TABLE_LEFT, TABLE_MERGE_LEFT_TAG, TABLE_MERGE_UP_LEFT_TAG, TABLE_MERGE_UP_TAG, TABLE_RIGHT, TABLE_SEP, is_blank}, line_parse::{InlineContext, InlinePreEl, PreSpan, PreStyle, PreparsedLines, generate_pre_spans, parse_note_line}};
use crate::resources::{Resources, Id, ImageId, FileId};
use crate::color::Color;

#[derive(Debug)]
pub(crate) enum OuterLink<'a> {
    URL(&'a str),
    File(Id<FileId>)
}
#[derive(Debug)]
pub(crate) struct InnerLink {
    pub(crate) index: u16,
    pub(crate) kind: RefKind
}
#[derive(Debug)]
pub(crate) enum InlineEl<'a> {
    String(&'a str),
    /// `Err` - name of the element that could not be found
    InnerLink(Result<InnerLink, (Cow<'a, str>, RefKind)>),
    /// `Err` - number of a missing note
    Note(Result<u16, u16>),
    Space,
    LongDash
}
pub(crate) struct Style<'a> {
    pub(crate) is_bold: bool,
    pub(crate) is_italic: bool,
    pub(crate) is_mono: bool,
    pub(crate) is_strike: bool,
    pub(crate) is_under: bool,
    pub(crate) color: Option<Color>,
    pub(crate) link: Option<OuterLink<'a>>,
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
pub(crate) struct Span<'a> {
    pub(crate) text: Vec<InlineEl<'a>>,
    pub(crate) style: Style<'a>
}

impl<'a> Span<'a> {
    /// `refs_allowed` - `false` inside an element name, where element references are invalid
    fn from_pre_span(pre: PreSpan<'a>, refs_allowed: bool, outer_links: &Resources, inner_links: &InnerProjectLinks) -> Self {
        let PreStyle {is_bold, is_italic, is_mono, is_strike, is_under, color, link} = pre.style;
        let link: Option<OuterLink> = match link {
            Some(pre_link) => {
                match outer_links.get_file_id(pre_link) {
                    Some(id) => Some(OuterLink::File(id)),
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
                match inner_links.resolve(k, &s).filter(|_| refs_allowed) {
                    Some(i) => Ok(InnerLink { index: i, kind: k }),
                    None => Err((s, k))
                }
            ),
            InlinePreEl::LongDash => InlineEl::LongDash,
            InlinePreEl::Note(n) => InlineEl::Note(if inner_links.has_note(n) {Ok(n)} else {Err(n)})
        }).collect();
        Self { text, style }
    }
}



fn parse_generic_block<'a> (lines: Vec<&'a str>, ctx: InlineContext, outer_links: &Resources, inner_links: &InnerProjectLinks) -> Vec<Span<'a>>{
    let preparsed = PreparsedLines::new(lines, ctx);
    let pre_spans = generate_pre_spans(preparsed);
    let mut content = Vec::new();
    for pre in pre_spans {
        content.push(Span::from_pre_span(pre, !ctx.is_name(), outer_links, inner_links));
    }
    content
}


#[derive(Debug)]
pub(crate) struct Paragraph<'a> {
    pub(crate) content: Vec<Span<'a>>
}


#[derive(Debug)]
pub(crate) struct Header<'a> {
    pub(crate) level: u8,
    pub(crate) index: [u16; 5],
    pub(crate) content: Vec<Span<'a>>
}

#[derive(Debug)]
pub(crate) struct Comment<'a> {
    pub(crate) content: Vec<Span<'a>>
}
#[derive(Debug)]
pub(crate) struct Formula<'a> {
    pub(crate) named: Named<Vec<Span<'a>>>,
    pub(crate) content: Vec<&'a str>
}
#[derive(Debug)]
pub(crate) struct Code<'a> {
    pub(crate) named: Named<Vec<Span<'a>>>,
    pub(crate) lang: Option<&'a str>,
    pub(crate) content: Vec<&'a str>
}
#[derive(Debug)]
pub(crate) struct Image<'a> {
    pub(crate) meta: ImageKind<Vec<Span<'a>>>,
    /// `Err` - path where no image was found
    pub(crate) path: Result<Id<ImageId>, Cow<'a, str>>
}
#[derive(Debug)]
pub(crate) struct NoteItem<'a> {
    pub(crate) index: u16,
    pub(crate) content: Vec<Span<'a>>
}
#[derive(Debug)]
pub(crate) struct Note<'a> {
    pub(crate) content: Vec<NoteItem<'a>>
}
#[derive(Debug)]
pub(crate) enum ListElKind<'a> {
    Ordered(Vec<Span<'a>>),
    Unordered(Vec<Span<'a>>)
}
#[derive(Debug)]
pub(crate) struct ListEl<'a> {
    pub(crate) this: Option<ListElKind<'a>>,
    pub(crate) children: Vec<ListEl<'a>>
}
 #[derive(Debug)]
pub(crate) struct List<'a> {
    pub(crate) content: Vec<ListEl<'a>>
}
impl<'a> List<'a> {
    fn build(out: &mut Vec<ListEl<'a>>,  lines: &[&'a str], outer_links: &Resources, inner_links: &InnerProjectLinks, level: u16) -> usize {
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
                let spans = parse_generic_block(vec![l_text], InlineContext::Text, outer_links, inner_links);
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

#[derive(Debug, Clone, Copy)]
pub(crate) enum Aligment {
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
pub(crate) enum TableEl<'a> {
    Cell(Vec<Span<'a>>),
    MergeLeft,
    MergeUp,
    MergeUpLeft
}



fn split_table_cells(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut prev: Option<char> = None;
    let mut chars = line.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        let next = chars.peek().map(|&(_, c)| c);
        if ch == TABLE_SEP && prev.is_none_or(is_blank) && next.is_none_or(is_blank) {
            out.push(line[start..i].trim_matches(is_blank));
            start = i + ch.len_utf8();
        }
        prev = Some(ch);
    }
    out.push(line[start..].trim_matches(is_blank));
    out
}

#[derive(Debug)]
pub(crate) struct Table<'a> {
    pub(crate) named: Named<Vec<Span<'a>>>,
    pub(crate) headers: Vec<Vec<TableEl<'a>>>,
    pub(crate) aligments: Vec<Aligment>,
    pub(crate) widths: Option<Vec<f32>>,
    pub(crate) cells: Vec<Vec<TableEl<'a>>>,
}
impl<'a> Table<'a> {
    fn new(named: Named<Vec<Span<'a>>>, lines: Vec<&'a str>, outer_links: &Resources, inner_links: &InnerProjectLinks) -> Self {
        let sep = lines.iter()
            .position(|l| {
                TABLE_FUL_SEP.strip(l).is_some() || TABLE_FUL_SEP_ALT.strip(l).is_some()
            }).expect("table block without separator");
        let format = TABLE_FUL_SEP.strip(lines[sep])
            .or_else(|| TABLE_FUL_SEP_ALT.strip(lines[sep]))
            .expect("table block without separator");
        let parse_rows = |rows: &[&'a str]| -> Vec<Vec<TableEl<'a>>> {
            rows.iter().map(|l| Self::parse_table_line(l, outer_links, inner_links)).collect()
        };
        let mut headers = parse_rows(&lines[..sep]);
        let mut cells = parse_rows(&lines[sep + 1..]);
        let columns = headers.iter().map(Vec::len).max().unwrap_or(0);
        Self::normalize(&mut headers, columns);
        Self::normalize(&mut cells, columns);
        let (aligments, widths) = Self::parse_format(format, columns);
        Self { named, headers, aligments, widths, cells }
    }

    /// Pads/truncates rows to `columns` and replaces impossible merges with empty cells
    fn normalize(rows: &mut Vec<Vec<TableEl<'a>>>, columns: usize) {
        for (r, row) in rows.iter_mut().enumerate() {
            row.resize_with(columns, || TableEl::Cell(Vec::new()));
            for (c, el) in row.iter_mut().enumerate() {
                let impossible = match el {
                    TableEl::MergeLeft => c == 0,
                    TableEl::MergeUp => r == 0,
                    TableEl::MergeUpLeft => c == 0 || r == 0,
                    TableEl::Cell(_) => false
                };
                if impossible {
                    *el = TableEl::Cell(Vec::new());
                }
            }
        }
    }

    /// `rest` - everything after `\№-`, outer `/` are optional
    fn parse_format(rest: &str, columns: usize) -> (Vec<Aligment>, Option<Vec<f32>>) {
        let rest = rest.trim_matches(is_blank);
        let mut pieces = split_table_cells(rest);
        if rest.ends_with(TABLE_SEP) {
            pieces.pop();
        }
        if rest.starts_with(TABLE_SEP) && !pieces.is_empty() {
            pieces.remove(0);
        }
        let mut pieces = pieces.into_iter();
        let (aligments, weights): (Vec<Aligment>, Vec<Option<u32>>) = (0..columns)
            .map(|_| Self::parse_format_piece(pieces.next().unwrap_or("")))
            .unzip();
        (aligments, Self::normalize_weights(&weights))
    }

    fn parse_format_piece(piece: &str) -> (Aligment, Option<u32>) {
        let digit = |c: char| c.is_ascii_digit();
        let head = piece.trim_start_matches(digit);
        let (letter, number) = if head.len() != piece.len() {
            (head, &piece[..piece.len() - head.len()])
        } else {
            let tail = piece.trim_end_matches(digit);
            (tail, &piece[tail.len()..])
        };
        let weight = if number.is_empty() {
            None
        } else {
            match number.parse::<u32>() {
                Ok(n) if n > 0 => Some(n),
                _ => return (Aligment::default(), None)
            }
        };
        let mut chars = letter.chars();
        let aligment = match (chars.next().and_then(|c| c.to_uppercase().next()), chars.next()) {
            (None, _) => Aligment::default(),
            (Some(TABLE_LEFT), None) | (Some(TABLE_LEFT_ALT), None) => Aligment::Left,
            (Some(TABLE_RIGHT), None) | (Some(TABLE_RIGHT_ALT), None) => Aligment::Right,
            (Some(TABLE_CENTER), None) | (Some(TABLE_CENTER_ALT), None)=> Aligment::Center,
            _ => return (Aligment::default(), None)
        };
        (aligment, weight)
    }

    fn normalize_weights(weights: &[Option<u32>]) -> Option<Vec<f32>> {
        let given: Vec<f64> = weights.iter().flatten().map(|&w| w as f64).collect();
        if given.is_empty() {
            return None;
        }
        let avg = given.iter().sum::<f64>() / given.len() as f64;
        let full: Vec<f64> = weights.iter().map(|w| w.map_or(avg, |w| w as f64)).collect();
        let total: f64 = full.iter().sum();
        Some(full.into_iter().map(|w| (w / total) as f32).collect())
    }

    fn parse_table_line(line: &'a str, outer_links: &Resources, inner_links: &InnerProjectLinks) -> Vec<TableEl<'a>> {
        let pieces = split_table_cells(line);

        pieces[1..pieces.len() - 1].iter().map(
            |&raw| {
                if TABLE_MERGE_LEFT_TAG.is(raw) {
                    TableEl::MergeLeft
                } else if TABLE_MERGE_UP_TAG.is(raw) {
                    TableEl::MergeUp
                } else if TABLE_MERGE_UP_LEFT_TAG.is(raw) {
                    TableEl::MergeUpLeft
                } else if raw.is_empty() {
                    TableEl::Cell(Vec::new())
                } else {
                    TableEl::Cell(parse_generic_block(vec![raw], InlineContext::TableCell, outer_links, inner_links))
                }
            }
        ).collect()
    }
}




pub(crate) enum Node<'a> {
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
    fn build_from_block(block: Block<'a>, outer_links: &Resources, inner_links: &InnerProjectLinks) -> Self {
        match block.kind {
            BlockKind::Paragraph => {
                let spans = parse_generic_block(block.lines, InlineContext::Text, outer_links, inner_links);
                Self::Paragraph(Paragraph {content: spans})
            },
            BlockKind::Comment => {
                let spans = parse_generic_block(block.lines, InlineContext::Text, outer_links, inner_links);
                Self::Comment(Comment {content: spans})
            },
            BlockKind::Header{level, counter} => {
                let spans = parse_generic_block(block.lines, InlineContext::Text, outer_links, inner_links);
                Self::Header( Header{content: spans, level, index: counter.0})
            },
            BlockKind::Note => {
                let content = block.lines.into_iter().map(|l| {
                    let (index, text) = parse_note_line(l).expect("not note line in note block");
                    NoteItem { index, content: parse_generic_block(vec![text], InlineContext::Text, outer_links, inner_links) }
                }).collect();
                Self::Note(Note { content })
            },
            BlockKind::Code { named, lang } => {
                Self::Code(Code {
                    named: named.map(|s| parse_generic_block(vec![s], InlineContext::Name, outer_links, inner_links)),
                    lang,
                    content: block.lines
                })
            },
            BlockKind::Formula(name) => {
                Self::Formula(Formula {
                    named: name.map(|s| parse_generic_block(vec![s], InlineContext::Name, outer_links, inner_links)),
                    content: block.lines
                })
            },
            BlockKind::Image(img) => {
                Self::Image(Image {
                     meta: img.map(|s| parse_generic_block(vec![s], InlineContext::ImageName, outer_links, inner_links)),
                     path: {
                        let path = block.lines.first().map_or(Cow::Borrowed(""), |s| unescape_image_path(s));
                        outer_links.get_image_id(&path).ok_or(path)
                     }
                })
            },
            BlockKind::List => {
                let mut out = Vec::new();
                List::build(&mut out, &block.lines, outer_links, inner_links, 0);
                Self::List(List { content: out })
            },
            BlockKind::Table(name) => {
                Self::Table(Table::new(
                    name.map(|s| parse_generic_block(vec![s], InlineContext::Name, outer_links, inner_links)),
                    block.lines,
                    outer_links,
                    inner_links
                ))
            }
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
                .field("lang", &v.lang)
                .field("content", &Lines(&v.content))
                .finish(),

            Node::Image(v) => f.debug_tuple("Image")
                .field(v)
                .finish(),

            Node::Note(v) => f.debug_struct("Note")
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


pub(crate) struct Rush<'a> {
    pub(crate) nodes: Vec<Node<'a>>,
    /// numbering counters after the last element
    pub(crate) counters: Counters,
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
    pub(crate) fn new(source: &'a str,  global_project: &Resources) -> Self {
        Self::new_from(source, global_project, Counters::default())
    }

    /// Numbering continues from `start`; the totals go to `counters`, the next file starts from them
    pub(crate) fn new_from(source: &'a str, global_project: &Resources, start: Counters) -> Self {
        let (lexed_project, counters) = split_blocks_from(source, start);
        let mut out = Self {nodes: Vec::new(), counters};

        for block in lexed_project.blocks {
            out.nodes.push(Node::build_from_block(block, global_project, &lexed_project.links));
        }


        out
    }
}


#[cfg(test)] mod test {
    use crate::parse::{rush::*};
    use crate::resources::*;

    fn empty_gp() -> Resources {
        Resources {
            images: ResTable::default(),
            files: ResTable::default()
        }
    }

    fn spans_text(spans: &[Span]) -> String {
        spans.iter()
            .flat_map(|s| s.text.iter())
            .map(|el| match el {
                InlineEl::String(s) => s,
                InlineEl::Space => " ",
                _ => "?"
            })
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    fn row_text(row: &[TableEl]) -> Vec<String> {
        row.iter().map(|el| match el {
            TableEl::Cell(spans) => spans_text(spans),
            TableEl::MergeLeft => "\\+".to_string(),
            TableEl::MergeUp => "\\=".to_string(),
            TableEl::MergeUpLeft => "\\+=".to_string(),
        }).collect()
    }

    fn rows_text(rows: &[Vec<TableEl>]) -> Vec<Vec<String>> {
        rows.iter().map(|r| row_text(r)).collect()
    }

    fn table(source: &str) -> (Vec<Vec<String>>, Vec<Vec<String>>, Vec<String>) {
        let gp = empty_gp();
        let out = Rush::new(source, &gp);
        match &out.nodes[..] {
            [Node::Table(t)] => (
                rows_text(&t.headers),
                rows_text(&t.cells),
                t.aligments.iter().map(|a| format!("{a:?}")).collect()
            ),
            other => panic!("ожидалась одна таблица: {other:?}")
        }
    }

    fn v(row: &[&str]) -> Vec<String> {
        row.iter().map(|s| s.to_string()).collect()
    }


    #[test]
    fn split_cells() {
        assert_eq!(split_table_cells("/ a / b /"), vec!["", "a", "b", ""]);
        assert_eq!(split_table_cells("/ км/ч / 5 /"), vec!["", "км/ч", "5", ""]);
        assert_eq!(split_table_cells("/ a _/ b /"), vec!["", "a _/ b", ""]);
        assert_eq!(split_table_cells("/ a//b /"), vec!["", "a//b", ""]);
        assert_eq!(split_table_cells("/ / /"), vec!["", "", "", ""]);
        assert_eq!(split_table_cells("/\ta\t/\tb /"), vec!["", "a", "b", ""]);
        assert_eq!(split_table_cells("Л / Ц /"), vec!["Л", "Ц", ""]);
    }

    #[test]
    fn table_escape_and_text_slash() {
        let (h, b, _) = table("\\№ 0\n/ км/ч / a _/ b /\n\\№-\n/ x__/y / \\:ж_/: /\n\\№");
        assert_eq!(h, vec![v(&["км/ч", "a / b"])]);
        assert_eq!(b, vec![v(&["x_/y", "ж/"])]);
    }

    #[test]
    fn slash_escape_only_in_table() {
        let out = Rush::new("a _/ b", &empty_gp());
        match &out.nodes[..] {
            [Node::Paragraph(p)] => assert_eq!(spans_text(&p.content), "a _/ b"),
            other => panic!("{other:?}")
        }
    }

    #[test]
    fn table_empty_cells_pad_and_cut() {
        let (h, b, a) = table("\\№ 0\n/ a / / c /\n\\№-\n/ / /\n/ 1 / 2 / 3 / 4 /\n\\№");
        assert_eq!(h, vec![v(&["a", "", "c"])]);
        assert_eq!(b, vec![v(&["", "", ""]), v(&["1", "2", "3"])]);
        assert_eq!(a, vec!["Center", "Center", "Center"]);
    }

    #[test]
    fn table_merges_on_edges() {
        let (h, b, _) = table(concat!(
            "\\№ 0\n",
            "/ \\+ / \\= / \\+= /\n",
            "/ \\=+ / \\+ / \\= /\n",
            "\\№-\n",
            "/ \\= / \\+= / a /\n",
            "/ \\+= / \\=+ / \\+ /\n",
            "\\№"
        ));
        assert_eq!(h, vec![v(&["", "", ""]), v(&["", "\\+", "\\="])]);
        assert_eq!(b, vec![v(&["", "", "a"]), v(&["", "\\+=", "\\+"])]);
    }

    #[test]
    fn table_aligments() {
        assert_eq!(table("\\№ 0\n/ a / b / c / d /\n\\№- Л / ц / X /\n\\№").2, vec!["Left", "Center", "Center", "Center"]);
        assert_eq!(table("\\№ 0\n/ a / b /\n\\№- / П / Л / Л /\n\\№").2, vec!["Right", "Left"]);
        assert_eq!(table("\\№ 0\n/ a / b /\n\\№- п\n\\№").2, vec!["Right", "Center"]);
    }

    fn table_widths(source: &str) -> Option<Vec<f32>> {
        let gp = empty_gp();
        let out = Rush::new(source, &gp);
        match &out.nodes[..] {
            [Node::Table(t)] => t.widths.clone(),
            other => panic!("{other:?}")
        }
    }

    fn assert_widths(source: &str, expected: &[f32]) {
        let got = table_widths(source).unwrap_or_else(|| panic!("{source}"));
        assert_eq!(got.len(), expected.len(), "{source}: {got:?}");
        for (g, e) in got.iter().zip(expected) {
            assert!((g - e).abs() < 1e-4, "{source}: {got:?} != {expected:?}");
        }
    }

    const T2: &str = "\\№ 0\n/ a / b /\n";
    const T3: &str = "\\№ 0\n/ a / b / c /\n";
    const T4: &str = "\\№ 0\n/ a / b / c / d /\n";

    #[test]
    fn table_widths_user_example() {
        let src = format!("{T4}\\№- / Ц100 / 200 / Ц / 50Ц /\n\\№");
        let avg = 350.0 / 3.0;
        let total = 350.0 + avg;
        assert_widths(&src, &[100.0 / total, 200.0 / total, avg / total, 50.0 / total]);
        assert_eq!(table(&src).2, vec!["Center"; 4]);
    }

    #[test]
    fn table_widths_number_on_both_sides() {
        let src = format!("{T2}\\№- 30Л / П70\n\\№");
        assert_widths(&src, &[0.3, 0.7]);
        assert_eq!(table(&src).2, vec!["Left", "Right"]);
    }

    #[test]
    fn table_widths_case_insensitive() {
        let src = format!("{T2}\\№- л25 / 75п\n\\№");
        assert_widths(&src, &[0.25, 0.75]);
        assert_eq!(table(&src).2, vec!["Left", "Right"]);
    }

    #[test]
    fn table_widths_only_numbers() {
        let src = format!("{T2}\\№- 1 / 3\n\\№");
        assert_widths(&src, &[0.25, 0.75]);
        assert_eq!(table(&src).2, vec!["Center", "Center"]);
    }

    #[test]
    fn table_widths_absent() {
        assert_eq!(table_widths(&format!("{T2}\\№- Л / Ц\n\\№")), None);
        assert_eq!(table_widths(&format!("{T2}\\№-\n\\№")), None);
        assert_eq!(table(&format!("{T2}\\№- Л / Ц\n\\№")).2, vec!["Left", "Center"]);
    }

    #[test]
    fn table_widths_fewer_pieces_than_columns() {
        assert_widths(&format!("{T3}\\№- 100\n\\№"), &[1.0 / 3.0; 3]);
    }

    #[test]
    fn table_widths_extra_pieces_ignored() {
        assert_widths(&format!("{T2}\\№- 10 / 30 / 1000\n\\№"), &[0.25, 0.75]);
    }

    #[test]
    fn table_widths_invalid_pieces() {
        for bad in ["0Ц", "Ц 10", "10Ц20", "Ц10Л", "X5", "ЦЛ10"] {
            let src = format!("{T2}\\№- {bad} / 40\n\\№");
            assert_widths(&src, &[0.5, 0.5]);
            assert_eq!(table(&src).2, vec!["Center", "Center"], "{bad}");
        }
        assert_eq!(table_widths(&format!("{T2}\\№- 0 / Л0\n\\№")), None);
    }

    #[test]
    fn code_lang_in_node() {
        let out = Rush::new("\\//py Пример\nprint(1)\n\\//", &empty_gp());
        match &out.nodes[..] {
            [Node::Code(c)] => {
                assert_eq!(c.lang, Some("py"));
                assert_eq!(c.content, vec!["print(1)"]);
                assert!(matches!(c.named, Named::WithName { .. }));
            }
            other => panic!("{other:?}")
        }
        let out = Rush::new("\\// Пример\nx\n\\//", &empty_gp());
        assert!(matches!(&out.nodes[..], [Node::Code(Code { lang: None, .. })]), "{out:?}");
    }

    #[test]
    fn table_row_without_spaces_is_invalid() {
        let out = Rush::new("\\№ 0\n/a / b /\n\\№-\n/ c /\n\\№", &empty_gp());
        assert!(!out.nodes.iter().any(|n| matches!(n, Node::Table(_))), "{out:?}");
    }

    fn table_name<'a>(node: &'a Node<'a>) -> &'a [Span<'a>] {
        match node {
            Node::Table(Table { named: Named::WithName { name, .. }, .. }) => name,
            other => panic!("{other:?}")
        }
    }

    #[test]
    fn element_ref_in_name_is_invalid() {
        let gp = empty_gp();
        let out = Rush::new("\\№ Т1\n/ a /\n\\№-\n\\№\n\\№ см. \\*т(Т1) и\\*1\n/ a /\n\\№-\n\\№\n\\**1 сноска", &gp);
        let refs: Vec<_> = table_name(&out.nodes[1]).iter()
            .flat_map(|s| s.text.iter())
            .filter_map(|el| match el {
                InlineEl::InnerLink(l) => Some(format!("link {}", l.is_ok())),
                InlineEl::Note(n) => Some(format!("note {n:?}")),
                _ => None
            })
            .collect();
        assert_eq!(refs, vec!["link false", "note Ok(1)"]);
    }

    #[test]
    fn forward_ref_is_valid() {
        let gp = empty_gp();
        let out = Rush::new("см. \\*т(Позже)\n\n\\№ Позже\n/ a /\n\\№-\n\\№", &gp);
        match &out.nodes[0] {
            Node::Paragraph(p) => assert!(p.content.iter().flat_map(|s| s.text.iter())
                .any(|el| matches!(el, InlineEl::InnerLink(Ok(InnerLink { index: 1, .. }))))),
            other => panic!("{other:?}")
        }
    }

    #[test]
    fn notes_grouped_into_one_node() {
        let gp = empty_gp();
        let out = Rush::new("\\**1 первая\n\\**2 вторая", &gp);
        match &out.nodes[..] {
            [Node::Note(n)] => {
                let items: Vec<_> = n.content.iter().map(|i| (i.index, spans_text(&i.content))).collect();
                assert_eq!(items, vec![(1, "первая".to_string()), (2, "вторая".to_string())]);
            },
            other => panic!("{other:?}")
        }
    }

    #[test]
    fn counters_continue_between_files() {
        let gp = empty_gp();
        let src = "\\++ Раздел\n\n\\№ Т\n/ a /\n\\№-\n\\№\n\n\\(a.png, Р)\n\n\\// Л\nx\n\\//\n\n\\$ Ф\nx\n\\$";
        let first = Rush::new(src, &gp);
        assert_eq!(first.counters.tables, 1);
        let second = Rush::new_from(src, &gp, first.counters);
        let indexes: Vec<String> = second.nodes.iter().map(|n| match n {
            Node::Header(h) => format!("h{:?}", &h.index[..2]),
            Node::Table(Table { named: Named::WithName { index, .. }, .. }) => format!("t{index}"),
            Node::Image(Image { meta: ImageKind { named: Named::WithName { index, .. }, .. }, .. }) => format!("i{index}"),
            Node::Code(Code { named: Named::WithName { index, .. }, .. }) => format!("c{index}"),
            Node::Formula(Formula { named: Named::WithName { index, .. }, .. }) => format!("f{index}"),
            other => panic!("{other:?}")
        }).collect();
        assert_eq!(indexes, vec!["h[1, 2]", "t2", "i2", "c2", "f2"]);
        assert_eq!(second.counters.formulas, 2);
    }

    #[test]
    fn image_caption_escaped_comma() {
        let gp = empty_gp();
        let out = Rush::new("\\(a.png, Схема_, вариант)", &gp);
        match &out.nodes[..] {
            [Node::Image(Image { meta: ImageKind { named: Named::WithName { name, .. }, .. }, path: Err(_) })] =>
                assert_eq!(spans_text(name), "Схема, вариант"),
            other => panic!("{other:?}")
        }
    }
}
