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


// TODO: Switch to &str, use it instead of `span`, toss it as block, and then as token
fn split_blocks<'a>(source: &'a str) -> Vec<Block<'a>> {
    let mut out: Vec<Block<'a>> = Vec::new();
    let mut merge_with_prev = false; 

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
            if line.starts_with(MARK){
                let words: Vec<&str> = line[1..].split(|c| is_blank(c)).filter(|w| !w.is_empty()).collect();
                println!("META: {words:?}");
            }
            else {
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
        let text = "aaa\n\\+ bbbb\nccc\n";
        let out = split_blocks(text);
        println!("{:?}", out);
    }
}