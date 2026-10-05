use std::vec;

use crate::parse::lexemas::{ *};



fn merge_flush<'a>(out: &mut Vec<Block<'a>>, line: &'a str) {
    if let Some(last) = out.last_mut() {
        last.lines.push(line);
        return;
    }
}

fn flush<'a>(out: &mut Vec<Block<'a>>, line: &'a str, kind: BlockKind){
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
    TableOpenOrFormat,
    TableLine,
    TableCloseOrFormat,
    Image(u16),
    Header(u8)
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
            let s = formated[1..].trim_start_matches(SPACE);
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
                        }
                    }
                    return Self {raw, formated, thinned, kind: LineKind::Image(0)} ;
                }
            }
            if !first_w.is_empty() {
                if rest.is_empty() {
                    if first_w.len() == 2 && first_w.chars().all(|c| c == CODE) {
                        return Self { raw, formated, thinned: formated, kind: LineKind::CodeClose };
                    } else if first_w.len() == 1 && first_w.starts_with(COMMENT) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::Comment};
                    } else if first_w.len() == 1 && first_w.starts_with(FORMULA) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::FormulaClose};
                    } else if first_w.len() == 3 && first_w.starts_with(TABLE) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::TableCloseOrFormat};
                    } else {
                        return Self { raw, formated, thinned: rest, kind: LineKind::Regular};
                    }
                } else {
                    println!("first_w: {first_w:?}, rest: {rest:?}, first_w.len(): {:?}", first_w.len());
                    if first_w.chars().all(|c| c == HEADER){
                        let header_level = first_w.chars().count().min(5) as u8;
                        return Self { raw, formated, thinned: rest, kind: LineKind::Header(header_level)};
                    } 
                    else if first_w.len() == 1 && first_w.starts_with(|c| c == BULLET_LIST || c == NUMERIC_LIST) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::List};
                    }
                    else if first_w.len() == 2 && first_w.chars().all(|c| c == CODE) {
                        return Self { raw, formated, thinned: rest, kind: LineKind::CodeOpen};
                    } else if first_w.len() == 1 && first_w.starts_with(FORMULA) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::FormulaOpen};
                    } else if first_w.len() == 3 && first_w.starts_with(TABLE) {
                        return Self {raw, formated, thinned: rest, kind: LineKind::TableOpenOrFormat};
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

    println!("lines {lines:?}");
    let mut current: usize = 0;
    let mut inner_block: Option<BlockKind> = None;

    while current < lines.len() {
        let line = &lines[current];
        match &inner_block {
            None => {
                match line.kind {
                    LineKind::Empty => par_is_open = false,
                    LineKind::Regular => {
                        if par_is_open {
                            merge_flush(&mut out, line.formated);
                        }
                        else {
                            flush(&mut out, line.formated, BlockKind::Paragraph);
                            par_is_open = true;
                        }
                    },
                    LineKind::Header(l) => {
                        flush(&mut out, line.thinned, BlockKind::Header(l));
                        par_is_open = false;
                    },
                    LineKind::CodeOpen => {
                        if search_for_pair(&lines, &current, Some(LineKind::CodeOpen), LineKind::CodeClose) {
                            flush(&mut out, line.thinned, BlockKind::Code);
                            inner_block = Some(BlockKind::Code);
                            par_is_open = false;
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
                            flush(&mut out, line.thinned, BlockKind::Formula);
                            inner_block = Some(BlockKind::Formula);
                            par_is_open = false;
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
                        flush(&mut out, line.formated,BlockKind::List);
                        inner_block = Some(BlockKind::List);
                        par_is_open = false;
                    }, 
                    LineKind::TableOpenOrFormat => {
                        let mut t_pos = current + 1;
                        let mut table_counter: u8 = 1;
                        loop {
                            if t_pos > lines.len() {
                                if par_is_open {
                                    merge_flush(&mut out, line.formated);
                                }
                                else {
                                    flush(&mut out, line.formated, BlockKind::Paragraph);
                                    par_is_open = true;
                                }
                                break;
                            }
                            let t_line = &lines[t_pos];
                            if t_line.kind == LineKind::TableCloseOrFormat || t_line.kind == LineKind::TableOpenOrFormat {
                                if t_pos == current + 1 {
                                    if par_is_open {
                                        merge_flush(&mut out, line.formated);
                                    }
                                    else {
                                        flush(&mut out, line.formated, BlockKind::Paragraph);
                                        par_is_open = true;
                                    }
                                    break;
                                } else {
                                    table_counter += 1;
                                }

                            }
                            t_pos += 1;
                        }
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
                    BlockKind::List => {
                        match line.kind {
                            LineKind::List => merge_flush(&mut out, line.formated),
                            _ => {
                                inner_block = None;
                                current -= 1;
                            }
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
    fn empty_line(){
        let source = "";
        let out = split_blocks(source);
        let expected: Vec<Block<'_>> = Vec::new();
        assert_eq!(out, expected);
    }

    #[test]
    fn only_empty_lines(){
        let source = "\n\n    \t  \t \t \n\n\n \t \t";
        let out = split_blocks(source);
        let expected: Vec<Block<'_>> = Vec::new();
        assert_eq!(out, expected);
    }

    fn last_new_line() {
        let source_a = "aa\n\nbb";
        let source_b = "aa\n\nbb\n";
        let out_a = split_blocks(source_a);
        let out_b = split_blocks(source_b);
        assert_eq!(out_a, out_b);
    }

    fn check_bom() {
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
    fn check_p_one_line_one_par(){
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
    fn check_p_two_lines_one_par(){
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
    fn check_p_two_par(){
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
    fn check_p_empty_countain_tabs_and_spaces(){
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
    fn check_p_multiple_empty_lines(){
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
    fn check_p_trim_tabs_and_spaces(){
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
    fn check_p_trim_card_return(){
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
    fn check_header_levels() {
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
    fn check_header_overload() {
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
    fn check_header_trim() {
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
    fn check_pluses_in_header_text() {
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
    fn header_need_space() {
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
    fn tab_can_be_space() {
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
    fn empty_header() {
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
    fn mark_test() {
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
    fn spaces_before_header() {
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
    fn header_split_to_blocks() {
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
    fn two_headers(){
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
    fn header_and_par() {
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
    fn check_code() {
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
    fn anon_code() {
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
    fn code_save_format() {
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
    fn code_trim_return_card() {
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
    fn check_code_spaces_around_close() {
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
    fn check_code_alon_closer() {
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
    fn check_code_not_closed() {
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
    fn check_blank_in_code() {
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
    fn check_two_open_one_close_code() {
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
    fn code_split_pars() {
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
    fn check_comment() {
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
    fn check_comment_multiline() {
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
    fn check_empty_comment() {
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
    fn check_comment_trim() {
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
    fn check_comment_spaces_around_tags() {
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
    fn comment_trim_return_card() {
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
    fn check_blank_in_comment() {
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
    fn comment_split_pars() {
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
    fn check_comment_not_closed() {
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
    fn comment_not_closed_not_break_par() {
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
    fn comment_not_closed_reparse() {
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
    fn comment_can_not_be_nested() {
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
    fn comment_tag_must_be_alone() {
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
    fn comment_contains_other_tags() {
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
    fn check_formula() {
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
    fn anon_formula() {
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
    fn formula_name_trim() {
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
    fn formula_need_space() {
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
    fn formula_tab_can_be_space() {
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
    fn formula_save_format() {
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
    fn formula_trim_return_card() {
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
    fn check_formula_spaces_around_close() {
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
    fn check_formula_alon_closer() {
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
    fn check_formula_not_closed() {
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
    fn formula_not_closed_not_break_par() {
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
    fn formula_not_closed_reparse() {
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
    fn check_blank_in_formula() {
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
    fn check_two_open_one_close_formula() {
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
    fn formula_split_pars() {
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
    fn formula_contains_other_tags() {
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
    fn formula_and_comment() {
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
    fn check_image(){
        let source = "\\(path, name, 100)";
        let out = split_blocks(source);
        let expected = vec![
            Block {
                kind: BlockKind::Image(100),
                lines: vec!["path, name"]
            }
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn check_list() {
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
    fn check_list_something_after() {
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
    fn check_list_split_par() {
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
    fn check_table() {
        let source = "\\№ tt\n/aa/bb/\n\\№ Л / Л\n/cc/dd/\n\\№";
        let out = split_blocks(source);
        
    }
}