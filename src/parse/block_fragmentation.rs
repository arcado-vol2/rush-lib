use std::vec;

use crate::parse::{block_fragmentation::LineKind::TableLine, lexemas::*};



fn merge_flush<'a>(out: &mut Vec<Block<'a>>, line: &'a str) {
    if let Some(last) = out.last_mut() {
        last.lines.push(line);
        return;
    }
}

fn flush<'a>(out: &mut Vec<Block<'a>>, line: &'a str, kind: BlockKind, par_is_open: &mut bool){
    *par_is_open = false;
    out.push(
        Block { kind, lines: vec![line]}
    );
}

#[derive(PartialEq, Debug)]
enum LineKind {
    Empty,
    Regular,
    CodeOpen,
    CodeClose,
    Comment,
    FormulaOpen,
    FormulaClose,
    List,
    TableOpen,
    TableLine,
    TableFormat,
    TableClose,
    Image(u16),
    Header(u8),
    Note(u16),
}

#[derive(Debug)]
struct ClassifiedLine<'a> {
    raw: &'a str,
    formated: &'a str,
    thinned: &'a str,
    kind: LineKind,
}

impl<'a> ClassifiedLine<'a> {
    fn new(inp_line: &'a str) -> Self {
        let line = inp_line.strip_suffix('\n').unwrap_or(inp_line);
        let raw = line.strip_suffix('\r').unwrap_or(line);
        let formated = raw.trim_matches(|c| is_blank(c));
        if formated.is_empty(){
            return Self {raw, formated, thinned: formated, kind: LineKind::Empty};
        } else if formated.starts_with(TABLE_SEP){
            return Self {raw, formated, thinned: formated, kind: LineKind::TableLine};
        } else if formated.starts_with(MARK) {
            let mut s = &formated[1..];
            if s.starts_with(SPACE) {
                s = s.trim_start_matches(SPACE);
                if !(s.starts_with(NUMERIC_LIST) || s.starts_with(BULLET_LIST)){
                    return Self { raw, formated, thinned: formated, kind: LineKind::Regular };
                }
            }
            let end = s.find(|c| is_blank(c)).unwrap_or(s.len());
            let (first_w, tail) = s.split_at(end);
            let rest = tail.trim_start_matches(|c| is_blank(c));
            if s.starts_with(IMG_LBRACE) && s.ends_with(IMG_RBRACE){
                let sep_count = s.matches(IMG_SEP).count();
                
                if sep_count == 1 || sep_count == 2 {
                    let thinned = 'get_first_and_last: {
                        let mut t = s.char_indices();
                        let first_end = match t.next(){
                            Some((_, ch)) => ch.len_utf8(),
                            None => break 'get_first_and_last s
                        };
                        let Some((last_start,_)) = t.next_back() else {
                            break  'get_first_and_last s;
                        };
                        
                        &s[first_end..last_start]
                    };
                    let content: Vec<&str> = thinned.split(IMG_SEP).collect();
                    if content.len() == 3 {
                        let (head, last) = thinned.rsplit_once(IMG_SEP).unwrap();
                        if let Ok(scale) = last.trim().parse::<u16>() && scale > 0{
                            return  Self {raw, formated, thinned: head, kind: LineKind::Image(scale)};
                        } else {
                            return Self {raw, formated, thinned: head, kind: LineKind::Image(0)} ;
                        }
                    }
                    return Self {raw, formated, thinned, kind: LineKind::Image(0)} ;
                }
            } else if !first_w.is_empty() {
                if rest.is_empty() {
                    if CODE_TAG.is(first_w) {
                        return Self { raw, formated, thinned: formated, kind: LineKind::CodeClose };
                    } else if COMMENT_TAG.is(first_w) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::Comment};
                    } else if FORMUAL_TAG.is(first_w) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::FormulaClose};
                    } else if TABLE_TAG.is(first_w) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::TableClose};
                    } else if first_w.len() == 4 && first_w.starts_with(TABLE) && first_w.ends_with(TABLE_FORMAT) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::TableFormat};
                    } 
                    else {
                        return Self { raw, formated, thinned: rest, kind: LineKind::Regular};
                    }
                } else {
                    if first_w.chars().all(|c| c == HEADER){
                        let header_level = first_w.chars().count().min(5) as u8;
                        return Self { raw, formated, thinned: rest, kind: LineKind::Header(header_level)};
                    } else if let Some(num) = NOTE_TAG.strip(first_w) {
                        let num = num.parse::<u16>();
                        if let Ok(link) = num{
                            return Self {raw, formated, thinned: rest, kind: LineKind::Note(link)};
                        } else {
                            return Self {raw, formated, thinned: formated, kind: LineKind::Regular};
                        }
                    }
                    else if CODE_TAG.is(first_w) {
                        return Self { raw, formated, thinned: rest, kind: LineKind::CodeOpen};
                    } else if FORMUAL_TAG.is(first_w) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::FormulaOpen};
                    } else if TABLE_TAG.is(first_w) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::TableOpen};
                    } else if NUMERIC_LIST_TAG.is(first_w) || BULLET_LIST_TAG.is(first_w) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::List};
                    }
                    else if first_w.len() == 4 && first_w.starts_with(TABLE) && first_w.ends_with(TABLE_FORMAT) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::TableFormat};
                    } 
                }
                
            }
        
        }
        return Self { raw, formated, thinned:formated, kind: LineKind::Regular};
    }
}


/// Search for pair line kind to decide is pair block are closed or not
/// 
/// `true` - find match
/// `false` - did not
/// 
fn search_for_pair(lines: &Vec<ClassifiedLine>, current: &usize, open_kind_opt: Option<LineKind>, close_kind: LineKind) -> bool 
{
    let mut t_pos = *current + 1;
    
    loop {
        if t_pos >= lines.len(){
            return false;
        }
        let t_line = &lines[t_pos];
        if Some(&t_line.kind) == open_kind_opt.as_ref()  {
            return false;
        } else if t_line.kind == close_kind { 
            return true;
        }
        t_pos+=1;
    }
}

fn split_blocks<'a>(source: &'a str) -> Vec<Block<'a>> {
    let source = source.strip_prefix('\u{FEFF}').unwrap_or(source);
    let mut out: Vec<Block<'a>> = Vec::new();
    let mut par_is_open = false; 
    let lines: Vec<ClassifiedLine> = source.split_inclusive('\n').map(|l| ClassifiedLine::new(l)).collect();

    let mut current: usize = 0;
    let mut inner_block: Option<BlockKind> = None;

    fn flush_par<'a>(out: &mut Vec<Block<'a>>, line: &'a str, par_is_open: &mut bool) {
         if *par_is_open {
            merge_flush(out, line);
        }
        else {
            out.push(
                Block { kind: BlockKind::Paragraph, lines: vec![line]}
            );
            *par_is_open = true;
        }
    }

    println!("Input");
    for line in &lines {
        println!("{line:?}");
    }
    println!("----");
    while current < lines.len() {
        let line = &lines[current];
        match &inner_block {
            None => {
                match line.kind {
                    LineKind::Empty => par_is_open = false,
                    LineKind::Regular | LineKind::TableFormat | LineKind::TableLine | LineKind::TableClose => {
                       flush_par(&mut out, line.formated, &mut par_is_open);
                    },
                    LineKind::Header(l) => {
                        flush(&mut out, line.thinned, BlockKind::Header(l), &mut par_is_open);
                    },
                    LineKind::CodeOpen => {
                        if search_for_pair(&lines, &current, Some(LineKind::CodeOpen), LineKind::CodeClose) {
                            flush(&mut out, line.thinned, BlockKind::Code, &mut par_is_open);
                            inner_block = Some(BlockKind::Code);
                        } else {
                            inner_block = None;
                        }
                    },
                    LineKind::Comment => {
                        if search_for_pair(&lines, &current, None, LineKind::Comment) {
                            out.push(Block { kind: BlockKind::Comment, lines: Vec::new() });
                            inner_block = Some(BlockKind::Comment);
                            par_is_open = false;
                        } else {
                            inner_block = None;
                        }
                    },
                    LineKind::FormulaOpen => {
                        if search_for_pair(&lines, &current, Some(LineKind::FormulaOpen), LineKind::FormulaClose) {
                            flush(&mut out, line.thinned, BlockKind::Formula, &mut par_is_open);
                            inner_block = Some(BlockKind::Formula);
                        } else {
                            inner_block = None;
                        }
                    }, 
                    LineKind::Image(scale) => {
                        out.push(
                            Block { kind: BlockKind::Image(scale), lines: vec![line.thinned]}
                        );
                        par_is_open = false;
                    },
                    LineKind::List => {
                        flush(&mut out, line.formated,BlockKind::List, &mut par_is_open);
                        for t_pos in current+1..lines.len(){
                            let t_line = &lines[t_pos];
                            match t_line.kind {
                                LineKind::List => {
                                    current = t_pos;
                                    merge_flush(&mut out, t_line.formated);
                                },
                                _ => {
                                    par_is_open = false;
                                    break;
                                }
                            }
                        }
                    }, 
                    LineKind::TableOpen => {
                        let mut t_pos = current + 1;
                        let mut found_format = false;
                        loop {
                            if t_pos >= lines.len() { 
                                flush_par(&mut out, line.formated, &mut par_is_open);
                                inner_block = None;
                                break;
                            }
                            let t_line = &lines[t_pos];
                            match t_line.kind {
                                LineKind::TableFormat => {
                                    if found_format {
                                        flush_par(&mut out, line.formated, &mut par_is_open);
                                        inner_block = None;
                                        break;
                                    }
                                    if t_pos == current + 1 { 
                                        flush_par(&mut out, line.formated, &mut par_is_open);
                                        inner_block = None;
                                        break;
                                    } else {
                                        found_format = true; 
                                    }
                                },
                                LineKind::TableClose => {
                                    if !found_format {
                                        flush_par(&mut out, line.formated, &mut par_is_open);
                                        inner_block = None;
                                        break;
                                    } else {
                                        flush(&mut out, line.formated, BlockKind::Table, &mut par_is_open);
                                        inner_block = Some(BlockKind::Table);
                                        break;
                                    }
                                },
                                LineKind::TableLine => { },
                                _ => { 
                                    flush_par(&mut out, line.formated, &mut par_is_open);
                                    inner_block = None;
                                    break;
                                }
                            }
                            t_pos += 1;
                        }
                    },
                    LineKind::Note(n) => {
                        flush(&mut out, line.thinned, BlockKind::Note(n), &mut par_is_open);
                    }
                    _ => {}
                }
            }
            Some(inner_block_kind) => {
                match inner_block_kind {
                    BlockKind::Code => {
                        match line.kind {
                            LineKind::CodeClose => inner_block = None,
                            _ => merge_flush(&mut out, line.raw)
                        }
                        
                    },
                    BlockKind::Comment => {
                        match line.kind {
                            LineKind::Comment => inner_block = None,
                            _ => merge_flush(&mut out, line.formated),
                        }
                    },
                    BlockKind::Formula => {
                        match line.kind {
                            LineKind::FormulaClose => inner_block = None,
                            _ => merge_flush(&mut out, line.raw),
                        }
                    },
                    BlockKind::Table => {
                        match line.kind {
                            LineKind::TableClose => {
                                inner_block = None
                            },
                            LineKind::TableFormat | LineKind::TableLine => { 
                                merge_flush(&mut out, line.formated);
                            },
                            _ => ()
                            
                        }
                    }
                    _ => {}
                }
            }
        }
        current+=1;
    }   
    out
}


#[cfg(test)] mod test {
    use std::vec;

use crate::parse::{block_fragmentation::split_blocks, lexemas::{Block, BlockKind::{self, Header}}};

    #[test]
    fn block_check_empty_source(){
        let source = "";
        let out = split_blocks(source);
        let expected: Vec<Block<'_>> = Vec::new();
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_empty_only_blank_lines(){
        let source = "\n\n    \t  \t \t \n\n\n \t \t";
        let out = split_blocks(source);
        let expected: Vec<Block<'_>> = Vec::new();
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_last_new_line() {
        let source_a = "aa\n\nbb";
        let source_b = "aa\n\nbb\n";
        let out_a = split_blocks(source_a);
        let out_b = split_blocks(source_b);
        assert_eq!(out_a, out_b);
    }

    #[test]
    fn block_check_p_bom() {
        let source = "\u{FEFF}aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_one_line_one_par(){
        let source = "aa";
        let out = split_blocks(source);
        let expected = vec![
            Block{
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_two_lines_one_par(){
        let source = "aa\nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_two_par(){
        let source = "aa\n\nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            },
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_empty_contain_tabs_and_spaces(){
        let source = "aa\n      \t\t   \t  \t \nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            },
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_multiple_empty_lines(){
        let source = "aa\n      \t\t  \n \n    \n \n    \n   \t  \t \nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            },
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_trim_tabs_and_spaces(){
        let source = "    \t\t \taa    \t \t\n\n                 bb\t\t\t";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            },
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_trim_card_return(){
        let source = "aa\r\nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);

        let source = "aa\r\n\r\nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            },
        ];
        assert_eq!(out, expected);
    }


    #[test] 
    fn block_check_header_levels() {
        let source = "\\+ 1\n\\++ 2\n\\+++ 3\n\\++++ 4\n\\+++++ 5";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: Header(1),
                lines: vec!["1"]
            },
            Block {
                kind: Header(2),
                lines: vec!["2"]
            },
            Block {
                kind: Header(3),
                lines: vec!["3"]
            },
            Block {
                kind: Header(4),
                lines: vec!["4"]
            },
            Block {
                kind: Header(5),
                lines: vec!["5"]
            }

        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_header_overload() {
        let source ="\\+++++++++++++++++++++++++++++ h";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: Header(5),
                lines: vec!["h"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_header_trim() {
        let source = "\\++     \t \taa    \t\t\t\n";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: Header(2),
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_header_pluses_in_text() {
        let source = "\\++ C++";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: Header(2),
                lines: vec!["C++"]
            }
        ];
        assert_eq!(out, expected);
    }

     #[test]
    fn block_check_header_p_need_space() {
        let source = "\\+aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\+aa"]
            }
        ];
        assert_eq!(out, expected);
    }

     #[test]
    fn block_check_header_tab_can_be_space() {
        let source = "\\++\taa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: Header(2),
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_header_p_empty() {
        let source = "\\+";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\+"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_mark_with_space() {
        let source = "\\ aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\ aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_header_spaces_before() {
        let source = "     \\+ aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Header(1),
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_header_p_split() {
        let source = "aa\n\\+ bb\ncc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Header(1),
                lines: vec!["bb"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_header_two_in_row(){
        let source = "\\++ aa\n\\++ bb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Header(2),
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Header(2),
                lines: vec!["bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_header_p_after() {
        let source = "\\+ aa\nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Header(1),
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_basic() {
        let source = "\\// aa\nbb\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_anon() {
        let source = "\\// 0\naa\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["0", "aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_save_format() {
        let source = "\\// aa\n\t   bb\n   cc\n\n\tdd\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["aa", "\t   bb", "   cc", "", "\tdd"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_trim_card_return() {
        let source = "\\// aa\nbb\r\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_spaces_around_close() {
        let source = "\\// aa\nbb\n\t\t   \\//\t   ";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_p_alone_closer() {
        let source = "\\//\naa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_p_not_closed() {
        let source = "    \\// aa\nbb\ncc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb", "cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_blank_inside() {
        let source = "\\// aa\n  \nbb\ncc\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["aa", "  ", "bb", "cc"]
            }
        ];
        assert_eq!(out, expected);
    }


    #[test]
    fn block_check_code_p_two_open_one_close() {
        let source = "\\// aa\nbb\ncc\n\\// dd\nee\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec![ "bb", "cc"]
            },
            Block {
                kind: BlockKind::Code,
                lines: vec!["dd", "ee"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_p_split() {
        let source = "aa\n\\// bb\ncc\n\\//\ndd";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec![ "aa"]
            },
            Block {
                kind: BlockKind::Code,
                lines: vec!["bb", "cc"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec![ "dd"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_basic() {
        let source = "\\\"\naa\n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec![ "aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_multiline() {
        let source = "\\\"\naa\nbb\ncc\n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec!["aa", "bb", "cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_empty() {
        let source = "\\\"\n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec![]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_trim() {
        let source = "\\\"\n   \taa \t\n\tbb   \n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_spaces_around_tags() {
        let source = "  \t \\\"\t  \naa\n\t\t   \\\"\t   ";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_trim_card_return() {
        let source = "\\\"\r\naa\r\nbb\r\n\\\"\r\n";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_blank_inside() {
        let source = "\\\"\naa\n\n   \t\nbb\n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec!["aa", "", "", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_p_split() {
        let source = "aa\n\\\"\nbb\n\\\"\ncc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Comment,
                lines: vec!["bb"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_p_not_closed() {
        let source = "\\\"\naa\nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_p_not_closed_not_break_par() {
        let source = "aa\n\\\"\nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_header_p_not_closed_reparse() {
        let source = "\\\"\naa\n\\+ bb\ncc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Header(1),
                lines: vec!["bb"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_p_can_not_be_nested() {
        let source = "\\\"\naa\n\\\"\nbb\n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_p_tag_must_be_alone() {
        let source = "\\\" aa\nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\\" aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_contains_other_tags() {
        let source = "\\\"\n\\+ aa\n\\// bb\n\\//\n\\$ cc\n\\$\n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec!["\\+ aa", "\\// bb", "\\//", "\\$ cc", "\\$"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_basic() {
        let source = "\\$ aa\nbb\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_anon() {
        let source = "\\$ 0\naa\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["0", "aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_name_trim() {
        let source = "\\$  \t aa \t\nbb\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_p_need_space() {
        let source = "\\$aa\nbb\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\$aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_tab_can_be_space() {
        let source = "\\$\taa\nbb\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_save_format() {
        let source = "\\$ aa\n  \\frac{a}{b}\t\n\t= c\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa", "  \\frac{a}{b}\t", "\t= c"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_trim_card_return() {
        let source = "\\$ aa\r\nbb\r\n\\$\r\n";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_spaces_around_close() {
        let source = "\\$ aa\nbb\n\t\t   \\$\t   ";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_p_alone_closer() {
        let source = "\\$\naa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_p_not_closed() {
        let source = "    \\$ aa\nbb\ncc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb", "cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_p_not_closed_not_break_par() {
        let source = "aa\n\\$ bb\ncc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa", "cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_header_p_not_closed_reparse() {
        let source = "\\$ aa\n\\+ bb\ncc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Header(1),
                lines: vec!["bb"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_blank_inside() {
        let source = "\\$ aa\n  \nbb\n\ncc\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa", "  ", "bb", "", "cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_p_two_open_one_close() {
        let source = "\\$ aa\nbb\ncc\n\\$ dd\nee\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb", "cc"]
            },
            Block {
                kind: BlockKind::Formula,
                lines: vec!["dd", "ee"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_p_split() {
        let source = "aa\n\\$ bb\ncc\n\\$\ndd";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Formula,
                lines: vec!["bb", "cc"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["dd"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_contains_other_tags() {
        let source = "\\$ aa\n\\+ bb\n\\\"\n\\// cc\n\\//\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa", "\\+ bb", "\\\"", "\\// cc", "\\//"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_comment_in_row() {
        let source = "\\$ aa\nbb\n\\$\n\\\"\ncc\n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa", "bb"]
            },
            Block {
                kind: BlockKind::Comment,
                lines: vec!["cc"]
            }
        ];
        assert_eq!(out, expected);
    }


    #[test]
    fn block_check_image_basic(){
        let source = "\\(aa, bb, 100)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(100),
                lines: vec!["aa, bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_invalid_scale(){
        let source = "\\(aa, bb, cc)";
        let out = split_blocks(source);
        let expected: Vec<Block> = vec![
            Block { kind: BlockKind::Image(0), lines: vec!["aa, bb"] }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_list_levels() {
        let source = "\\1 aa\n\\ 1 bb\n\\ . cc\n";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::List,
                lines: vec!["\\1 aa", "\\ 1 bb", "\\ . cc"]
            }
        ];
        assert_eq!(out, expected);
    }
    
    #[test]
    fn block_check_list_header_after() {
        let source = "\\1 aa\n\\1 bb\n\\+ cc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::List,
                lines: vec!["\\1 aa", "\\1 bb"]
            },
            Block {
                kind: BlockKind::Header(1),
                lines: vec!["cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_list_p_split() {
        let source = "aa\n\\. bb\ncc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. bb"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_basic() {
        let source = "\\№ tt\n/aa/bb/\n\\№- Л / Л\n/cc/dd/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ tt", "/aa/bb/", "\\№- Л / Л", "/cc/dd/"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_note_basic() {
        let source = "\\**1 aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Note(1),
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    // ===== General =====

    #[test]
    fn block_check_empty_bom_and_blank_lines() {
        let source = "\u{FEFF}\n  \n\t";
        let out = split_blocks(source);
        let expected: Vec<Block<'_>> = Vec::new();
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_bom_not_at_start() {
        let source = "aa\n\u{FEFF}bb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa", "\u{FEFF}bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_mixed_line_endings() {
        let source = "aa\r\nbb\n\r\ncc\n";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa", "bb"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_only_tabs_separator() {
        let source = "aa\n\t\t\nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_p_unknown_tag() {
        let source = "\\я aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\я aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_header_p_shielded_tag() {
        let source = "_\\+ aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["_\\+ aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    // ===== Space between \ and tag =====

    #[test]
    fn block_check_header_p_space_after_mark() {
        let source = "\\ + aa\n\\   ++ aa\n\\\t+ aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\ + aa", "\\   ++ aa", "\\\t+ aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_p_space_after_mark() {
        let source = "\\ // aa\nbb\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\ // aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_p_space_after_mark() {
        let source = "\\ $ aa\nbb\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\ $ aa", "bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_p_space_after_mark() {
        let source = "\\ \"\naa\n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\ \"", "aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_space_after_mark() {
        let source = "\\ № tt\n/aa/\n\\№-\n/bb/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\ № tt", "/aa/", "\\№-", "/bb/", "\\№"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_p_space_after_mark() {
        let source = "\\ (aa, bb)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\ (aa, bb)"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_note_p_space_after_mark() {
        let source = "\\ **1 aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\ **1 aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_space_after_mark_not_close() {
        let source = "\\// aa\nbb\n\\ //\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["aa", "bb", "\\ //"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_space_after_mark_not_close() {
        let source = "\\$ aa\nbb\n\\ $\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa", "bb", "\\ $"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_space_after_mark_not_close() {
        let source = "\\\"\naa\n\\ \"\n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec!["aa", "\\ \""]
            }
        ];
        assert_eq!(out, expected);
    }

    // ===== Paragraph =====

    #[test]
    fn block_check_header_p_empty_parts_dropped() {
        let source = "\n\\+ aa\nbb\n\\+ cc\n\n";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Header(1),
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            },
            Block {
                kind: BlockKind::Header(1),
                lines: vec!["cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    // ===== Header =====

    #[test]
    fn block_check_header_p_only_spaces_after_tag() {
        let source = "\\++   ";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\++"]
            }
        ];
        assert_eq!(out, expected);

        let source = "\\+\t";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\+"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_header_p_mixed_tag() {
        let source = "\\+-+ aa\n\\+a aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\+-+ aa", "\\+a aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_header_trim_card_return() {
        let source = "\\+ aa\r\nbb\r\n";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Header(1),
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_list_header_break_list() {
        let source = "\\. aa\n\\+ bb\n\\. cc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. aa"]
            },
            Block {
                kind: BlockKind::Header(1),
                lines: vec!["bb"]
            },
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    // ===== List =====

    #[test]
    fn block_check_list_deep_levels() {
        let source = "\\. aa\n\\   . bb\n\\1 cc\n\\1 dd";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. aa", "\\   . bb", "\\1 cc", "\\1 dd"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_list_tab_after_marker() {
        let source = "\\.\taa\n\\1\tbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::List,
                lines: vec!["\\.\taa", "\\1\tbb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_list_p_need_space() {
        let source = "\\.aa\n\\1aa\n\\12 aa\n\\2 aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\.aa", "\\1aa", "\\12 aa", "\\2 aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_list_split_by_blank() {
        let source = "\\. aa\n\n\\. bb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. aa"]
            },
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_list_trim() {
        let source = "  \t\\. aa \t\n   \\ 1 bb  ";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. aa", "\\ 1 bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_list_image_after() {
        let source = "\\. aa\n\\(bb, cc)\n\\. dd";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. aa"]
            },
            Block {
                kind: BlockKind::Image(0),
                lines: vec!["bb, cc"]
            },
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. dd"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_list_code_after() {
        let source = "\\. aa\n\\// bb\ncc\n\\//\n\\. dd";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. aa"]
            },
            Block {
                kind: BlockKind::Code,
                lines: vec!["bb", "cc"]
            },
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. dd"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_list_table_after() {
        let source = "\\. aa\n\\№ tt\n/bb/\n\\№-\n/cc/\n\\№\n\\. dd";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. aa"]
            },
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ tt", "/bb/", "\\№-", "/cc/"]
            },
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. dd"]
            }
        ];
        assert_eq!(out, expected);
    }

    // ===== Image =====

    #[test]
    fn block_check_image_no_scale() {
        let source = "\\(aa, bb)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(0),
                lines: vec!["aa, bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_anon() {
        let source = "\\(aa, 0)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(0),
                lines: vec!["aa, 0"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_scale_bounds() {
        let source = "\\(aa, bb, 1)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(1),
                lines: vec!["aa, bb"]
            }
        ];
        assert_eq!(out, expected);

        let source = "\\(aa, bb, 65535)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(65535),
                lines: vec!["aa, bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_invalid_scale_values() {
        let source = "\\(aa, bb, 0)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(0),
                lines: vec!["aa, bb"]
            }
        ];
        assert_eq!(out, expected);

        let source = "\\(aa, bb, -5)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(0),
                lines: vec!["aa, bb"]
            }
        ];
        assert_eq!(out, expected);

        let source = "\\(aa, bb, 65536)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(0),
                lines: vec!["aa, bb"]
            }
        ];
        assert_eq!(out, expected);

        let source = "\\(aa, bb, )";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(0),
                lines: vec!["aa, bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_p_one_param() {
        let source = "\\(aa)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\(aa)"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_p_too_many_params() {
        let source = "\\(aa, bb, 100, dd)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\(aa, bb, 100, dd)"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_p_not_closed() {
        let source = "\\(aa, bb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\(aa, bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_p_text_after() {
        let source = "\\(aa, bb) cc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\(aa, bb) cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_spaces_around() {
        let source = "  \t\\(aa, bb, 100)\t  ";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(100),
                lines: vec!["aa, bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_p_split() {
        let source = "aa\n\\(bb, cc)\ndd";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Image(0),
                lines: vec!["bb, cc"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["dd"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_image_two_in_row() {
        let source = "\\(aa, bb)\n\\(cc, dd, 50)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(0),
                lines: vec!["aa, bb"]
            },
            Block {
                kind: BlockKind::Image(50),
                lines: vec!["cc, dd"]
            }
        ];
        assert_eq!(out, expected);
    }

    // ===== Note =====

    #[test]
    fn block_check_note_anon() {
        let source = "\\**0 aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Note(0),
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_note_tab_and_trim() {
        let source = "   \\**1\t  aa bb \t";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Note(1),
                lines: vec!["aa bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_note_p_split() {
        let source = "aa\n\\**1 bb\ncc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Note(1),
                lines: vec!["bb"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_note_many_anon() {
        let source = "\\**0 aa\n\\**0 bb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Note(0),
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Note(0),
                lines: vec!["bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_note_p_invalid() {
        let source = "\\**\n\\**a aa\n\\**-1 aa\n\\**1aa\n\\* 1 aa\n\\*1 aa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\**", "\\**a aa", "\\**-1 aa", "\\**1aa", "\\* 1 aa", "\\*1 aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    // ===== Comment =====

    #[test]
    fn block_check_comment_tag_with_text_inside() {
        let source = "\\\"\naa\n\\\" bb\n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec!["aa", "\\\" bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_two_in_row() {
        let source = "\\\"\naa\n\\\"\n\\\"\nbb\n\\\"";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Comment,
                lines: vec!["bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_p_text_after_close() {
        let source = "\\\"\naa\n\\\"\nbb";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Comment,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_comment_code_p_not_closed_reparse() {
        let source = "\\\"\naa\n\\// bb\ncc\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Code,
                lines: vec!["bb", "cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_list_comment_break_list() {
        let source = "\\. aa\n\\\"\nbb\n\\\"\n\\. cc";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. aa"]
            },
            Block {
                kind: BlockKind::Comment,
                lines: vec!["bb"]
            },
            Block {
                kind: BlockKind::List,
                lines: vec!["\\. cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    // ===== Formula =====

    #[test]
    fn block_check_formula_empty() {
        let source = "\\$ aa\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_name_with_spaces() {
        let source = "\\$ aa bb\ncc\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa bb", "cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_code_open_inside() {
        let source = "\\$ aa\n\\// bb\n\\$";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Formula,
                lines: vec!["aa", "\\// bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_p_closed_by_code() {
        let source = "\\$ aa\nbb\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["bb"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_formula_p_anon_need_space() {
        let source = "\\$0\naa";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\$0", "aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    // ===== Code =====

    #[test]
    fn block_check_code_empty() {
        let source = "\\// aa\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_name_with_spaces() {
        let source = "\\// aa bb\ncc\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["aa bb", "cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_p_invalid_tag() {
        let source = "\\///\n\\//aa\n\\//0";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\///", "\\//aa", "\\//0"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_contains_other_tags() {
        let source = "\\// aa\n\\+ bb\n\\\"\n\\$ cc\n\\$\n\\№ dd\n\\. ee\n\\**1 ff\n\\(gg, hh)\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["aa", "\\+ bb", "\\\"", "\\$ cc", "\\$", "\\№ dd", "\\. ee", "\\**1 ff", "\\(gg, hh)"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_formula_open_inside() {
        let source = "\\// aa\n\\$ bb\ncc\n\\//";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["aa", "\\$ bb", "cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_code_trim_card_return_blank() {
        let source = "\\// aa\r\nbb\r\n\r\ncc\r\n\\//\r\n";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Code,
                lines: vec!["aa", "bb", "", "cc"]
            }
        ];
        assert_eq!(out, expected);
    }

    // ===== Table =====

    #[test]
    fn block_check_table_no_format() {
        let source = "\\№ tt\n/aa/bb/\n\\№-\n/cc/dd/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ tt", "/aa/bb/", "\\№-", "/cc/dd/"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_multiple_header_lines() {
        let source = "\\№ tt\n/aa/bb/\n/cc/dd/\n\\№- Л / П\n/ee/ff/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ tt", "/aa/bb/", "/cc/dd/", "\\№- Л / П", "/ee/ff/"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_empty_body() {
        let source = "\\№ tt\n/aa/bb/\n\\№-\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ tt", "/aa/bb/", "\\№-"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_anon() {
        let source = "\\№ 0\n/aa/\n\\№-\n/bb/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ 0", "/aa/", "\\№-", "/bb/"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_trim() {
        let source = "  \\№ tt \t\n\t/aa/bb/  \n \\№- Л \n  /cc/dd/\t\n  \\№  ";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ tt", "/aa/bb/", "\\№- Л", "/cc/dd/"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_trim_card_return() {
        let source = "\\№ tt\r\n/aa/\r\n\\№-\r\n/bb/\r\n\\№\r\n";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ tt", "/aa/", "\\№-", "/bb/"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_need_space() {
        let source = "\\№tt\n/aa/\n\\№-\n/bb/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№tt", "/aa/", "\\№-", "/bb/", "\\№"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_no_header() {
        let source = "\\№ tt\n\\№-\n/aa/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№ tt", "\\№-", "/aa/", "\\№"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_not_closed() {
        let source = "\\№ tt\n/aa/\n\\№-\n/bb/";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№ tt", "/aa/", "\\№-", "/bb/"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_no_format() {
        let source = "\\№ tt\n/aa/\n/bb/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№ tt", "/aa/", "/bb/", "\\№"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_blank_inside() {
        let source = "\\№ tt\n/aa/\n\n\\№-\n/bb/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№ tt", "/aa/"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№-", "/bb/", "\\№"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_text_inside() {
        let source = "\\№ tt\n/aa/\nbb\n\\№-\n/cc/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№ tt", "/aa/", "bb", "\\№-", "/cc/", "\\№"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_header_p_block_inside() {
        let source = "\\№ tt\n/aa/\n\\+ bb\n\\№-\n/cc/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№ tt", "/aa/"]
            },
            Block {
                kind: BlockKind::Header(1),
                lines: vec!["bb"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№-", "/cc/", "\\№"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_two_formats() {
        let source = "\\№ tt\n/aa/\n\\№-\n/bb/\n\\№-\n/cc/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№ tt", "/aa/", "\\№-", "/bb/", "\\№-", "/cc/", "\\№"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_wrong_order() {
        let source = "\\№ tt\n/aa/\n\\№\n\\№-\n/bb/";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№ tt", "/aa/", "\\№", "\\№-", "/bb/"]
            }
        ];
        assert_eq!(out, expected);
    }


    #[test]
    fn block_check_table_p_invalid_continue_par() {
        let source = "aa\n\\№ tt\n/bb/";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa", "\\№ tt", "/bb/"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_readme_example() {
        let source = "\\№ Неполная таблица\n/ Заголовок / Заголовок /\n\\№ Новая таблица\n/ Заголовок / Заголовок /\n\\№-\n/ ячейка / ячейка /\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["\\№ Неполная таблица", "/ Заголовок / Заголовок /"]
            },
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ Новая таблица", "/ Заголовок / Заголовок /", "\\№-", "/ ячейка / ячейка /"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_split() {
        let source = "aa\n\\№ tt\n/bb/\n\\№-\n/cc/\n\\№\ndd";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            },
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ tt", "/bb/", "\\№-", "/cc/"]
            },
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["dd"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_two_in_row() {
        let source = "\\№ tt\n/aa/\n\\№-\n/bb/\n\\№\n\\№ uu\n/cc/\n\\№-\n/dd/\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ tt", "/aa/", "\\№-", "/bb/"]
            },
            Block {
                kind: BlockKind::Table,
                lines: vec!["\\№ uu", "/cc/", "\\№-", "/dd/"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_lines_outside() {
        let source = "/aa/bb/\n\\№- Л";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["/aa/bb/", "\\№- Л"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn block_check_table_p_alone_close() {
        let source = "aa\n\\№";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Paragraph,
                lines: vec!["aa", "\\№"]
            }
        ];
        assert_eq!(out, expected);
    }
}