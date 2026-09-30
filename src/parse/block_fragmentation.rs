use std::vec;

use crate::parse::lexemas::*;

#[derive(PartialEq, Debug)]
enum SpecialIndicator {
    NotYet,
    FoundMark,
    Maybe,
    SpecialEnded,
    NoWay,
    Closed
}


fn flush<'a>(out: &mut Vec<Block<'a>>, new_line: &'a str, merge_with_prev: &mut bool, kind: BlockKind){
    if *merge_with_prev {
        if let Some(last) = out.last_mut() {
            if last.kind == kind {
        
                last.lines.push(new_line);
                return;
            }
        }
    }
    out.push(
        Block { kind, lines: vec![new_line]}
    );
    *merge_with_prev = true;
}


fn split_blocks<'a>(source: &'a str) -> Vec<Block<'a>> {
    let mut out: Vec<Block<'a>> = Vec::new();
    let mut merge_with_prev = false; 
    let mut double_block: Option<BlockKind> = None; 
    for raw in source.split_inclusive('\n') {
        println!();
        println!("raw: {:?}", raw);
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let line = line.strip_suffix('\r').unwrap_or(line).trim_matches(|c| is_blank(c));
        println!("result {:?}", line);

        if line.is_empty() {
            merge_with_prev = false;
            println!("line is emtpy skip it");
            continue;
        } else {
            println!("my line: {line}");
            'detect: {
                if line.starts_with(MARK){
                    let s = &line[1..];
                    let end = s.find(|c| is_blank(c)).unwrap_or(s.len());
                    let (first_w, tail) = s.split_at(end);
                    let rest = tail.trim_start_matches(|c| is_blank(c));
                    if rest.is_empty(){
                        //TODO: add here comment handling as second condition
                    }
                    else if !first_w.is_empty() && first_w.chars().all(|c| c == HEADER){
                        let header_level = first_w.chars().count().min(5) as u8;
                        merge_with_prev = false;
                        flush(&mut out, rest, &mut merge_with_prev, BlockKind::Header(header_level));
                        break 'detect;
                    }
                   
                }
                println!("else detect {line}");
                flush(&mut out, line, &mut merge_with_prev, BlockKind::Paragraph);
            }
        
        }
    }   
    out
}


#[cfg(test)] mod test {
    use crate::parse::block_fragmentation::split_blocks;

    #[test] 
    fn main() {
        let text = "\\+\n\\+";
        let out = split_blocks(text);
        println!("{:?}", out);
    }
}