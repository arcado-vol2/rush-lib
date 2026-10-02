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
    Header(u8)
}

#[derive(Debug)]
struct ClassifiedLine<'a> {
    raw: &'a str,
    formated: &'a str,
    kind: LineKind,
}

impl<'a> ClassifiedLine<'a> {
    fn new(inp_line: &'a str) -> Self {
        let line = inp_line.strip_suffix('\n').unwrap_or(inp_line);
        let raw = line.strip_suffix('\r').unwrap_or(line);
        let formated = raw.trim_matches(|c| is_blank(c));
        if formated.is_empty(){
            return Self {raw, formated, kind: LineKind::Empty};
        } else if formated.starts_with(MARK) {
            let s = &formated[1..];
            let end = s.find(|c| is_blank(c)).unwrap_or(s.len());
            let (first_w, tail) = s.split_at(end);
            let rest = tail.trim_start_matches(|c| is_blank(c));
            if !first_w.is_empty() {
                if rest.is_empty() {
                    if first_w.chars().count() == 2 && first_w.chars().all(|c| c == CODE) {
                        return Self { raw, formated, kind: LineKind::CodeClose };
                    }else {
                        return Self { raw, formated, kind: LineKind::Regular};
                    }
                } 
                if first_w.chars().all(|c| c == HEADER){
                    let header_level = first_w.chars().count().min(5) as u8;
                    return Self { raw, formated: rest, kind: LineKind::Header(header_level)};
                } else if first_w.chars().count() == 2 && first_w.chars().all(|c| c == CODE) {
                    return Self { raw, formated: rest, kind: LineKind::CodeOpen};
                }
            }
        
        }
        return Self { raw, formated, kind: LineKind::Regular};
    }
}


fn split_blocks<'a>(source: &'a str) -> Vec<Block<'a>> {
    let mut out: Vec<Block<'a>> = Vec::new();
    let mut par_is_open = false; 
    let lines: Vec<ClassifiedLine> = source.split_inclusive('\n').map(|l| ClassifiedLine::new(l)).collect();
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
                            flush(&mut out, &line.formated, BlockKind::Paragraph);
                            par_is_open = true;
                        }
                    },
                    LineKind::Header(l) => {
                        flush(&mut out, line.formated, BlockKind::Header(l));
                        par_is_open = false;
                    },
                    LineKind::CodeOpen => {
                        let mut t_pos = current+1;
                        loop {
                            if t_pos >= lines.len(){
                                inner_block = None;
                                break;
                            }
                            let t_line = &lines[t_pos];
                            if t_line.kind == LineKind::CodeClose {
                                inner_block = Some(BlockKind::Code);
                                flush(&mut out, line.formated, BlockKind::Code);
                                
                                break;
                            }
                            t_pos+=1;
                        }
                    },
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
    use crate::parse::{block_fragmentation::split_blocks, lexemas::{Block, BlockKind}};

    #[test]
    fn check_p(){
        let source = "aa";
        let out = split_blocks(source);
        let expected = vec![
            Block{
                kind: BlockKind::Paragraph,
                lines: vec!["aa"]
            }
        ];
        assert_eq!(out, expected);


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
    fn main() {

        let text = "";
        let out = split_blocks(text);
        println!("{:?}", out);
    }
}