use std::borrow::Cow;

use crate::parse::lexemas::{BOLD, BYTE_SHIELD, CODE, COLOR, ITALIC, LBRACE, MARK, MONO, RBRACE, SHIELD, SMALL_E, SMALL_YO, STRIKE, UNDER};


fn is_should_be_changed(ch: char) -> bool {
    ch == SHIELD || ch ==MARK || ch == SMALL_YO || ch.is_uppercase()
}

fn is_shealdable(ch: char) -> bool {
    matches!(
        ch,
        MARK | SHIELD | BOLD | ITALIC | STRIKE | UNDER |MONO | COLOR | LBRACE | RBRACE | CODE
    )
}


#[derive(PartialEq, Debug, Clone, Copy)]
enum InlineElement {
    Bold,
    Italic,
    Strike,
    Under,
    Mono,
    Color
}

pub(crate) fn clear_name_attribute<'a>(line: &'a str) -> Cow<'a, str> {
    let Some(first) = line.find(is_should_be_changed) else {
        return Cow::Borrowed(line);
    };

    let mut out = String::with_capacity(line.len());
    out.push_str(&line[..first]);
    
    let mut chars = line[first..].chars();
    let mut braces_stack: Vec<InlineElement> = Vec::with_capacity(line.len());
    let mut in_color = false;

    fn push_closer(braces_stack: &mut Vec<InlineElement>, out: &mut String, kind: InlineElement, ch: &char){
        if let Some(pos) = *&braces_stack.iter().rposition(|&open| open == kind) {
            braces_stack.truncate(pos);
        } else {
            out.push(*ch);
        }
    }


    while let Some(ch) = chars.next() {
        match ch {
            SHIELD => {
                println!(" found SHIELD");
                if let Some(next) = chars.next() {
                    if is_shealdable(next) {
                        out.push(next);
                    } else {
                        out.push(ch);
                        out.push(next);
                    }
                } else {
                    out.push(ch);
                }
            },
            MARK => {
                if let Some(next) = chars.next() {
                    match next {
                        BOLD => braces_stack.push(InlineElement::Bold),
                        ITALIC => braces_stack.push(InlineElement::Italic),
                        STRIKE => braces_stack.push(InlineElement::Strike),
                        UNDER => braces_stack.push(InlineElement::Under),
                        MONO => braces_stack.push(InlineElement::Mono),
                        COLOR => {
                            braces_stack.push(InlineElement::Color);
                            in_color = true
                        },
                        _ => ()
                    }
                }

            },
            _ => {
                if in_color { 
                    if ch == LBRACE{
                        in_color = false
                    } else {
                    }
                }
                else {
                    let ch =ch.to_lowercase().next().unwrap();
                    match ch {
                        LBRACE => in_color = false,
                        BOLD => push_closer(&mut braces_stack, &mut out, InlineElement::Bold, &ch),
                        ITALIC => push_closer(&mut braces_stack, &mut out, InlineElement::Italic, &ch),
                        MONO => push_closer(&mut braces_stack, &mut out, InlineElement::Mono, &ch),
                        STRIKE => push_closer(&mut braces_stack, &mut out, InlineElement::Strike, &ch),
                        UNDER => push_closer(&mut braces_stack, &mut out, InlineElement::Under, &ch),
                        RBRACE => push_closer(&mut braces_stack, &mut out, InlineElement::Color, &ch),
                        SMALL_YO => out.push(SMALL_E),
                        _ => out.push(ch)
                    }
                }
            }

        }
        
    }
    return Cow::Owned(out);

}


fn clear_name_link<'a>(line: &'a str) -> Cow<'a, str> {
    let Some(first) = line.bytes().position(|b| b == BYTE_SHIELD) else {
        return Cow::Borrowed(line);
    };
    let mut out = String::with_capacity(line.len());
    out.push_str(&line[..first]);
    
    let mut chars = line[first..].chars();
    while let Some(ch) = chars.next() {
        match ch {
            SHIELD => {
                if let Some(next) = chars.next() && next == RBRACE {
                    out.push(next);
                }
            }
            _ => {
                out.push(ch);
            }
        }
    }
    return Cow::Owned(out);
}


#[cfg(test)] mod test {
    use std::borrow::Cow;

    use crate::parse::line_parse::*;

    fn check(input: &str, expected: &str) {
        assert_eq!(clear_name_attribute(input), expected);
    }


    #[test]
    fn check_line_clear_name_plain_borrowed() {
        let out = clear_name_attribute("простое имя");
        assert!(matches!(out, Cow::Borrowed(_)));
        assert_eq!(out, "простое имя");
    }

    #[test]
    fn check_line_clear_name_empty() {
        check("", "");
    }

    #[test]
    fn check_line_clear_name_format_simple() {
        check("\\:жирный:", "жирный");
        check("\\;курсив;", "курсив");
        check("\\-зачёркнутый-", "зачеркнутый");
        check("\\=подчёркнутый=", "подчеркнутый");
        check("\\!моно!", "моно");
    }

    #[test]
    fn check_line_clear_name_format_inside_text() {
        check("Схема \\:главного: узла", "схема главного узла");
    }

    #[test]
    fn check_line_clear_name_format_closer_without_opener() {
        check("Время 10:30; итог - 5 = 5!", "время 10:30; итог - 5 = 5!");
    }

    #[test]
    fn check_line_clear_name_color_hex() {
        check("\\%#1b2b20(текст)", "текст");
    }

    #[test]
    fn check_line_clear_name_color_constant() {
        check("до \\%КРАС(красный) после", "до красный после");
    }

    #[test]
    fn check_line_clear_name_color_rgb() {
        check("\\%255103020(текст)", "текст");
    }

    // ===== Escaping =====

    #[test]
    fn check_line_clear_name_shield_special_char() {
        check("\\:10_:30:", "10:30");
    }

    #[test]
    fn check_line_clear_name_shield_double_underscore() {
        check("a__:", "a_:");
    }

    #[test]
    fn check_line_clear_name_shield_backslash() {
        check("_\\:", "\\:");
    }

    #[test]
    fn check_line_clear_name_shield_regular_char_kept() {
        check("snake_case", "snake_case");
    }

    #[test]
    fn check_line_clear_name_shield_end_of_line_kept() {
        check("имя_", "имя_");
    }

    #[test]
    fn check_line_clear_name_shield_regular_char_inside_format() {
        check("\\:my_var:", "my_var");
    }

    // ===== Unclosed elements =====

    #[test]
    fn check_line_clear_name_format_not_closed() {
        check("\\:незакрытый", "незакрытый");
    }


    // ===== Nesting =====

    #[test]
    fn check_line_clear_name_nested_different() {
        check("\\:a \\;b; c:", "a b c");
    }

    #[test]
    fn check_line_clear_name_nested_same() {
        check("\\:a \\:b: c:", "a b c");
    }

    #[test]
    fn check_line_clear_name_nested_color() {
        check("\\%КРАС(a \\%СИН(b) c)", "a b c");
    }

    #[test]
    fn check_line_clear_name_nested_outer_invalidates_inner() {
        check("\\:a \\;b: c;", "a b c;");
    }

    // ===== ё normalization =====

    #[test]
    fn check_line_clear_name_yo_replaced() {
        check("Ёмкость ёлки", "емкость елки");
    }

    #[test]
    fn check_line_clear_name_yo_inside_format() {
        check("\\:Схема ёмкости:", "схема емкости");
    }

    // ===== Key property =====

    #[test]
    fn check_line_clear_name_formatted_matches_plain() {
        let plain = "схема узла";
        for formatted in [
            "\\:Схема узла:",
            "\\:Схема: узла",
            "Схема \\;узла;",
            "\\%КРАС(Схема) узла",
            "\\=\\:Схема: узла=",
        ] {
            assert_eq!(clear_name_attribute(formatted), plain, "вход: {formatted:?}");
        }
    }

    #[test]
    fn check_line_clear_name_link(){
        assert_eq!(clear_name_link("aa"), "aa");
    }

    #[test]
    fn check_line_clear_name_link_shield_rbrace(){
        assert_eq!(clear_name_link("(a_)a"), "(a)a");
    }
}
