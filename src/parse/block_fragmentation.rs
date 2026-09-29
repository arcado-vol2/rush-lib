use crate::parse::lexemas::{BlockKind::{Header1, Header2, Header3, Header4, Header5}, *};


fn flush(out: &mut Vec<Block>, merge_with_prev: &mut bool, span: Span, kind: BlockKind ) {
    if *merge_with_prev {
        if let Some(last) = out.last_mut() {
            if last.kind == kind {
                last.span.end = span.end;
                return;
            }
        }
    }
    out.push(
        Block { kind, span }
    );
    *merge_with_prev = true;
}

#[derive(PartialEq, Debug)]
enum SpecialIndicator {
    NotYet,
    FoundMark,
    Maybe,
    SpecialEnded,
    NoWay,
    Closed
}


fn split_blocks(source: &str) -> Vec<Block> {
    let mut out = Vec::new();

    let chars: Vec<char> = source.chars().collect();
    let mut pos = 0; 
    let mut merge_with_prev = false; 
    while pos < chars.len() {
        if is_blank(chars[pos]) {
            pos+=1;
            continue;
        } else if chars[pos] == NEW_LINE {
            merge_with_prev = false;
            pos+=1;
        } else {
            let mut line_span = Span { start: pos, end: pos };
            let mut special = SpecialIndicator::NotYet;
            let mut first = true;
            let mut header_count = 0;
            let mut block_kind: Option<BlockKind> = None;
            while pos < chars.len() && chars[pos] != NEW_LINE {
                if !is_blank(chars[pos]){
                    line_span.end = pos;
                    if first {
                        if chars[pos] == MARK {
                            special = SpecialIndicator::FoundMark;
                        } else {
                            special = SpecialIndicator::NoWay;
                        }
                        first = false;
                    } else if special == SpecialIndicator::FoundMark {
                        if is_special(chars[pos])  {
                            special = SpecialIndicator::Maybe;
                        } else {
                            special = SpecialIndicator::NoWay;
                        }
                    }
                    if special == SpecialIndicator::Maybe {
                        if chars[pos] == HEADER {
                            if block_kind.is_none() {
                                block_kind = Some(BlockKind::Header1);
                                header_count = 1;
                            } else {
                                header_count += 1;
                                block_kind = Some(match header_count {
                                    1 => Header1,
                                    2 => Header2,
                                    3 => Header3,
                                    4 => Header4,
                                    _ => Header5,
                                })
                            }
                        }
                        else {
                            special = SpecialIndicator::NoWay;
                        }
                    } else if special == SpecialIndicator::SpecialEnded {
                        special = SpecialIndicator::Closed;
                    }
                } else {
                    if special == SpecialIndicator::Maybe {
                        if block_kind.is_some() {
                            special = SpecialIndicator::SpecialEnded;
                        } else {
                            special = SpecialIndicator::NoWay;
                        }
                    }
                }
                println!("{:?} at {}, SpecialIndicator: {:?}, BlockKind: {:?}", chars[pos], pos, special, block_kind);
                pos+=1;
            }
            println!("While end SpecialIndicator: {:?}, BlockKind: {:?}", special, block_kind);
            let block_kind = match special {
                SpecialIndicator::Closed => {
                    block_kind.unwrap_or(BlockKind::Paragraph)
                }
                _ => BlockKind::Paragraph
            };
            println!("Block kind will be {:?}", block_kind);
            flush(&mut out, &mut merge_with_prev, line_span, block_kind);
            pos+=1;
        }
    }


    out    

}




#[cfg(test)] mod test {
    use crate::parse::block_fragmentation::split_blocks;

    #[test] 
    fn main() {
        println!("---------");
        let text = "\\++ d\n   a     \n\nb";
        let b = split_blocks(text);
        println!("{:?}", b);
        println!("---------");
    }
}