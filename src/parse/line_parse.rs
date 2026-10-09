use std::borrow::Cow;

use crate::parse::lexemas::{BOLD, BYTE_SHIELD, COLOR, ITALIC, LBRACE, MARK, MONO, RBRACE, SHIELD, SMALL_E, SMALL_YO, STRIKE, UNDER};


#[derive(Debug, PartialEq)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}
impl Color {
    const BLACK: Self = Self {r: 0, g: 0, b: 0};
    const WHITE: Self = Self {r: 255, g: 255, b: 255};
    const RED: Self = Self {r: 255, g: 0, b: 0};
    const ORANGE: Self = Self {r: 255, g: 128, b: 0};
    const YELLOW: Self = Self {r: 255, g: 200, b: 0};
    const GREEN: Self = Self {r: 0, g: 255, b: 0};
    const LIGHT_BLUE: Self = Self {r: 0, g: 128, b: 255};
    const DARK_BLUE: Self = Self {r: 0, g: 0, b: 255};
    const MAGENTA: Self = Self {r: 200, g: 0, b: 255};
    const PINK: Self = Self {r: 255, g: 128, b: 128};
    const BROWN: Self = Self {r: 64, g: 32, b: 0};
    const LEMON: Self = Self {r: 255, g: 247, b: 0};
    const CHARTREUSE: Self = Self {r: 127, g: 255, b: 0};
    const CYAN: Self = Self {r: 0, g: 200, b: 255};
    const LAZULI: Self = Self {r: 38, g: 97, b: 156};
    const FUCHSIA: Self = Self {r: 255, g: 0, b: 255};
    

    fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    fn from_hex(col: &str) -> Self{
        let hex = col.strip_prefix('#').unwrap_or(col);
        let mut digits = hex.chars().map(|c| c.to_digit(16).map(|d| d as u8));
        Self::from_iterator(&mut digits).unwrap_or(Self::BLACK)
    }

    fn from_ru_hex(col: &str) -> Self {
        let hex = col.strip_prefix(['ц', 'Ц']).unwrap_or(col);
        let mut digits = hex.chars().map(|c| ru_hex_handler(c));
        Self::from_iterator(&mut digits).unwrap_or(Self::BLACK)
    }

    fn from_digits(col: &str) -> Self {
        let mut digits = col.chars().map(|c| c.to_digit(10));
        let mut next_byte = || {
            let val = digits.next()?? * 100 + digits.next()?? * 10 + digits.next()??;
            u8::try_from(val).ok()
        };
        match (next_byte(), next_byte(), next_byte()) {
            (Some(r), Some(g), Some(b)) => Self {r, g, b},
            _ => Self::BLACK
        }
    }

    fn from_iterator(digits: &mut impl Iterator<Item = Option<u8>>) -> Option<Self> {
        let mut next_byte = || Some(digits.next()?? * 16 + digits.next()??);
        let (r, g, b) = (next_byte()?, next_byte()?, next_byte()?);
        Some(Self::new(r, g, b))
    }

    fn from_unknown_string(line: &str) -> Self {
        if let Some(c) = ru_color_const(line){
            return c;
        } else {
            if line.starts_with('#') {
                return Self::from_hex(line);
            } else if line.starts_with(['ц', 'Ц']) {
                return Self::from_ru_hex(line);
            } else {
                return Self::from_digits(line);
            }
        }
    }

}

fn ru_color_const(line: &str) -> Option<Color> {
    match line {
        "КРАС" => Some(Color::RED),
        "ОРАН" => Some(Color::ORANGE),
        "ЖЁЛ" | "ЖЕЛ" => Some(Color::YELLOW),
        "ЗЕЛ" => Some(Color::GREEN),
        "ГОЛ" => Some(Color::LIGHT_BLUE),
        "СИН" => Some(Color::DARK_BLUE),
        "ФИОЛ" => Some(Color::MAGENTA),
        "РОЗ" => Some(Color::PINK),
        "КОР" => Some(Color::BROWN),
        "ЛИМ" => Some(Color::LEMON),
        "САЛ" => Some(Color::CHARTREUSE),
        "БИР" => Some(Color::CYAN),
        "ЛАЗ" => Some(Color::LAZULI),
        "ФУК" => Some(Color::FUCHSIA),
        "БЕЛ" => Some(Color::WHITE),
        "ЧЕРН" | "ЧЁРН" => Some(Color::BLACK),
        _ => None
    }
}

fn ru_hex_handler(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        'а'|'А' => Some(10),
        'б'|'Б' => Some(11),
        'в'|'В' => Some(12),
        'г'|'Г' => Some(13),
        'д'|'Д' => Some(14),
        'е'|'Е' => Some(15),
         _ => None
    }
}


fn is_should_be_changed(ch: char) -> bool {
    ch == SHIELD || ch ==MARK || ch == SMALL_YO || ch.is_uppercase()
}

fn is_inline_marking(ch: char) -> bool {
    matches!(
        ch,
        MARK | SHIELD | BOLD | ITALIC | STRIKE | UNDER | MONO | COLOR | LBRACE | RBRACE
    )
}




#[derive(PartialEq, Debug, Clone, Copy)]
enum InlineClearKind {
    Bold,
    Italic,
    Strike,
    Under,
    Mono,
    Color(u32), 
}

pub(crate) fn clear_name_attribute<'a>(line: &'a str) -> Cow<'a, str> {
    let Some(first) = line.find(is_should_be_changed) else {
        return Cow::Borrowed(line);
    };

    let mut out = String::with_capacity(line.len());
    out.push_str(&line[..first]);
    
    let mut chars = line[first..].chars().peekable();
    let mut braces_stack: Vec<InlineClearKind> = Vec::new();
    let mut depth: u32 = 0;
    let mut in_color = false;

    fn push_closer(braces_stack: &mut Vec<InlineClearKind>, out: &mut String, kind: InlineClearKind, ch: &char){
        if let Some(pos) = braces_stack.iter().rposition(|&open| open == kind) {
            braces_stack.truncate(pos);
        } else {
            out.push(*ch);
        }
    }


    while let Some(ch) = chars.next() {
        match ch {
            SHIELD => {
                if let Some(next) = chars.next_if(|&c| is_inline_marking(c)) {
                    out.push(next);
                } else {
                    out.push(ch);
                }
            },
            MARK => {
                if let Some(next) = chars.peek() {
                    match *next {
                        BOLD => {
                            braces_stack.push(InlineClearKind::Bold);
                            chars.next();
                        },
                        ITALIC => {
                            braces_stack.push(InlineClearKind::Italic);
                            chars.next();
                        },
                        STRIKE => {
                            braces_stack.push(InlineClearKind::Strike);
                            chars.next();
                        },
                        UNDER => {
                            braces_stack.push(InlineClearKind::Under);
                            chars.next();
                        },
                        MONO => {
                            braces_stack.push(InlineClearKind::Mono);
                            chars.next();
                        },
                        COLOR => {
                            braces_stack.push(InlineClearKind::Color(depth));
                            in_color = true;
                            chars.next();
                        },
                        _ => ()
                    }
                }

            },
            _ => {
                if in_color { 
                    if ch == LBRACE{
                        in_color = false;
                        depth += 1;
                    }
                }
                else {
                    let ch = ch.to_lowercase().next().unwrap();
                    match ch {
                        LBRACE => {
                            out.push(ch);
                            depth += 1;
                        },
                        BOLD => push_closer(&mut braces_stack, &mut out, InlineClearKind::Bold, &ch),
                        ITALIC => push_closer(&mut braces_stack, &mut out, InlineClearKind::Italic, &ch),
                        MONO => push_closer(&mut braces_stack, &mut out, InlineClearKind::Mono, &ch),
                        STRIKE => push_closer(&mut braces_stack, &mut out, InlineClearKind::Strike, &ch),
                        UNDER => push_closer(&mut braces_stack, &mut out, InlineClearKind::Under, &ch),
                        RBRACE => {
                            depth = depth.saturating_sub(1);
                            let color = braces_stack.iter().rposition(|e| matches!(e, InlineClearKind::Color(_)));
                            match color {
                                Some(pos) if braces_stack[pos] == InlineClearKind::Color(depth) => braces_stack.truncate(pos),
                                _ => out.push(ch),
                            }
                        },
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


struct Style{
    bold: bool,
    italic: bool,
    mono: bool,
    under: bool,
    strike: bool,
    color: Color
}


#[derive(Debug, PartialEq)]
enum Mark {
    Bold,
    Italic,
    Strike,
    Under,
    Mono,
    Color(Color),
}


#[derive(Debug)]
struct Opened {
    mark: Mark,
    start: usize,
    depth: u32
}

#[derive(Debug, PartialEq)]
struct Range {
    mark: Mark,
    start: usize,
    end: usize
}

#[derive(Debug, PartialEq)]
enum InlineText<'a> {
    String(&'a str),
    Empty
}

#[derive(Debug, PartialEq)]
struct PreparsedLine<'a> {
    text: Vec<InlineText<'a>>,
    style_ranges: Vec<Range>
}

impl<'a> PreparsedLine<'a> {
    fn try_pop_stack(open_stack: &mut Vec<Opened>, is_target: impl Fn(&Opened) -> bool) -> Option<Opened>{
        let pos = open_stack.iter().rposition(is_target)?;
        let target = open_stack.remove(pos);
        open_stack.truncate(pos);
        return Some(target);
    }

    fn push_range_to_out (out: &mut Self, line: &'a str, start:usize, end: usize) {
        if start < end {
             out.text.push(InlineText::String(&line[start..end])); 
        }
    }

    fn new(lines: Vec<&'a str>) -> Self {
        let mut out = Self {
            text: Vec::new(),
            style_ranges: Vec::new()
        };

        for line in lines{
            let mut open_stack: Vec<Opened> = Vec::new();
            let mut chars = line.char_indices().peekable();
            let mut byte_after_last_tag = 0;
            while let Some((i, ch)) = chars.next(){
                match ch {
                    MARK => {
                        if let Some((next_i, next_ch)) = chars.peek() {
                            match *next_ch {
                                BOLD => {
                                    Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                    byte_after_last_tag = next_i + next_ch.len_utf8();
                                    open_stack.push(Opened { mark: Mark::Bold, start: out.text.len(), depth: 0});
                                    chars.next();
                                },
                                ITALIC => {
                                    Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                    byte_after_last_tag = next_i + next_ch.len_utf8();
                                    open_stack.push(Opened { mark: Mark::Italic, start: out.text.len(), depth: 0 });
                                    chars.next();
                                },
                                STRIKE => {
                                    Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                    byte_after_last_tag = next_i + next_ch.len_utf8();
                                    open_stack.push(Opened { mark: Mark::Strike, start: out.text.len(), depth: 0 });
                                    chars.next();
                                },
                                UNDER => {
                                    Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                    byte_after_last_tag = next_i + next_ch.len_utf8();
                                    open_stack.push(Opened { mark: Mark::Under, start: out.text.len(), depth: 0 });
                                    chars.next();
                                },
                                MONO => {
                                    Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                    byte_after_last_tag = next_i + next_ch.len_utf8();
                                    open_stack.push(Opened { mark: Mark::Mono, start: out.text.len(), depth: 0 });
                                    chars.next();
                                },
                                COLOR => {
                                    let param_start = *next_i + COLOR.len_utf8();
                                    match chars.find(|&(_, c)| c == LBRACE) {
                                        Some((brace_i, _)) => {
                                            let color = Color::from_unknown_string(&line[param_start..brace_i]);
                                            println!("{color:?}");
                                        }
                                        None => ()
                                    }
                                    
                                }
                                _ => {
                                    Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                    byte_after_last_tag = i + ch.len_utf8();
                                }
                            }
                        }
                    },
                    SHIELD => {
                        if chars.next_if(|&(_, c)| is_inline_marking(c)).is_some() {
                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                            byte_after_last_tag = i + SHIELD.len_utf8();
                        }
                    }
                    BOLD => {
                        if let Some(opened) = Self::try_pop_stack(&mut open_stack, |t| t.mark == Mark::Bold){
                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                            byte_after_last_tag = i + ch.len_utf8();
                            out.style_ranges.push(
                                Range { mark: Mark::Bold, start: opened.start, end: out.text.len() }
                            );
                        }
                    },
                    ITALIC => {
                        if let Some(opened) = Self::try_pop_stack(&mut open_stack, |t| t.mark == Mark::Italic){
                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                            byte_after_last_tag = i + ch.len_utf8();
                            out.style_ranges.push(
                                Range { mark: Mark::Italic, start: opened.start, end: out.text.len() }
                            );
                        }
                    },
                    STRIKE => {
                        if let Some(opened) = Self::try_pop_stack(&mut open_stack, |t| t.mark == Mark::Strike){
                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                            byte_after_last_tag = i + ch.len_utf8();
                            out.style_ranges.push(
                                Range { mark: Mark::Strike, start: opened.start, end: out.text.len() }
                            );
                        }
                    },
                    UNDER => {
                        if let Some(opened) = Self::try_pop_stack(&mut open_stack, |t| t.mark == Mark::Under){
                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                            byte_after_last_tag = i + ch.len_utf8();
                            out.style_ranges.push(
                                Range { mark: Mark::Under, start: opened.start, end: out.text.len() }
                            );
                        }
                    },
                    MONO => {
                        if let Some(opened) = Self::try_pop_stack(&mut open_stack, |t| t.mark == Mark::Mono){
                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                            byte_after_last_tag = i + ch.len_utf8();
                            out.style_ranges.push(
                                Range { mark: Mark::Mono, start: opened.start, end: out.text.len() }
                            );
                        }
                    },
                    _ => {
                    }
                }
            }
            Self::push_range_to_out(&mut out, line, byte_after_last_tag, line.len());
            for open in open_stack.into_iter().rev() {
                out.style_ranges.push(Range { mark: open.mark, start: open.start, end: out.text.len()});
            }
        }

        out
    }
}










#[cfg(test)] mod test {
    use std::{borrow::Cow, vec};

    use crate::parse::line_parse::*;

    fn check(input: &str, expected: &str) {
        assert_eq!(clear_name_attribute(input), expected);
    }

    #[test]
    fn main() {
        let out = PreparsedLine::new(vec!["text \\%#ЗАЛЭПА(text)"]);
        println!("{out:?}")
    }


    #[test]
    fn check_line_preparsed_line_common() {
        let out = PreparsedLine::new(vec!["\\:a:"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Bold, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\:a:b\\;c;"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a"), InlineText::String("b"), InlineText::String("c")],
            style_ranges: vec![Range{mark: Mark::Bold, start: 0, end: 1}, Range{mark: Mark::Italic, start: 2, end: 3}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\;:x;"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String(":x")],
            style_ranges: vec![Range{mark: Mark::Italic, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\;\\:a:;"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Bold, start: 0, end: 1}, Range{mark: Mark::Italic, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        // ===== Mono =====

        let out = PreparsedLine::new(vec!["\\!a!"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Mono, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\!a!b\\-c-"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a"), InlineText::String("b"), InlineText::String("c")],
            style_ranges: vec![Range{mark: Mark::Mono, start: 0, end: 1}, Range{mark: Mark::Strike, start: 2, end: 3}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\!-x!"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("-x")],
            style_ranges: vec![Range{mark: Mark::Mono, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\!\\=a=!"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Under, start: 0, end: 1}, Range{mark: Mark::Mono, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        // ===== Strike =====

        let out = PreparsedLine::new(vec!["\\-a-"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Strike, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\-a-b\\=c="]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a"), InlineText::String("b"), InlineText::String("c")],
            style_ranges: vec![Range{mark: Mark::Strike, start: 0, end: 1}, Range{mark: Mark::Under, start: 2, end: 3}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\-!x-"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("!x")],
            style_ranges: vec![Range{mark: Mark::Strike, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\-\\!a!-"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Mono, start: 0, end: 1}, Range{mark: Mark::Strike, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        // ===== Under =====

        let out = PreparsedLine::new(vec!["\\=a="]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Under, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\=a=b\\!c!"]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a"), InlineText::String("b"), InlineText::String("c")],
            style_ranges: vec![Range{mark: Mark::Under, start: 0, end: 1}, Range{mark: Mark::Mono, start: 2, end: 3}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\=:x="]);
        let expected = PreparsedLine {
            text: vec![InlineText::String(":x")],
            style_ranges: vec![Range{mark: Mark::Under, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLine::new(vec!["\\=\\-a-="]);
        let expected = PreparsedLine {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Strike, start: 0, end: 1}, Range{mark: Mark::Under, start: 0, end: 1}]
        };
        assert_eq!(out, expected);
    }

    #[test]
    fn check_line_clear_braces_with_bold(){
        check("A ( \\:a ) b:", "a ( a ) b");
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
    fn check_line_clear_invalid_after_mark(){
        check("\\Abc","abc");
    }

    #[test]
    fn check_line_clear_braces_in_name(){
        check("F(x)", "f(x)");
    }

    #[test]
    fn check_line_clear_shealding() {
        check("snake_Case", "snake_case");
        check("snake_ёж", "snake_еж");
    }

    #[test]
    fn check_line_clear_brace_balance() {
        check("\\%R(a(b)c)", "a(b)c");
        check("\\%К(a(b\\%К(c)d)e)", "a(bcd)e");
        check("\\%К(x\\:a\\%С(b:c)d)", "xabc)d");
    }

    #[test]
    fn check_line_clear_color_cuts_bold() {
        check("\\%К(a \\:b) c:", "a b c:");
    }

    #[test]
    fn check_line_clear_rbrace_without_color() {
        check("A) b", "a) b");
    }

    #[test]
    fn check_line_clear_unknown_gliph_after_mark(){
        check("\\*p(A)", "*p(a)");
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
