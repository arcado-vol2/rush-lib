use std::borrow::Cow;
use std::ops::{Add, AddAssign};

use crate::parse::lexemas::{BOLD, BYTE_SHIELD, COLOR, ITALIC, LBRACE, LINK, MARK, MONO, NOTE, RBRACE, RefKind, SHIELD, SMALL_E, SMALL_YO, SPACE, STRIKE, UNDER, ref_kind};


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


    fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    fn from_hex(col: &str) -> Option<Self>{
        let hex = col.strip_prefix('#').unwrap_or(col);
        let mut digits = hex.chars().map(|c| c.to_digit(16).map(|d| d as u8));
        Self::from_iterator(&mut digits)
    }

    fn from_ru_hex(col: &str) -> Option<Self >{
        let hex = col.strip_prefix(['ц', 'Ц']).unwrap_or(col);
        let mut digits = hex.chars().map(|c| Self::ru_hex_handler(c));
        Self::from_iterator(&mut digits)
    }

    fn from_digits(col: &str) -> Option<Self> {
        let mut digits = col.chars().map(|c| c.to_digit(10));
        let mut next_byte = || {
            let val = digits.next()?? * 100 + digits.next()?? * 10 + digits.next()??;
            u8::try_from(val).ok()
        };
        match (next_byte(), next_byte(), next_byte()) {
            (Some(r), Some(g), Some(b)) => Some(Self {r, g, b}),
            _ => None
        }
    }

    fn from_iterator(digits: &mut impl Iterator<Item = Option<u8>>) -> Option<Self> {
        let mut next_byte = || Some(digits.next()?? * 16 + digits.next()??);
        let (r, g, b) = (next_byte()?, next_byte()?, next_byte()?);
        Some(Self::new(r, g, b))
    }

    fn from_unknown_string(line: &str) -> Option<Self> {
        let cosnt_color = Self::ru_color_const(line);
        if cosnt_color.is_some(){
            return cosnt_color;
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

fn is_should_be_changed_on_name(ch: char) -> bool {
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
    let Some(first) = line.find(is_should_be_changed_on_name) else {
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
    
    fn push_to_stack(braces_stack: &mut Vec<InlineClearKind>, chars: &mut impl Iterator<Item = char>, what: InlineClearKind){
        braces_stack.push(what);
        chars.next();
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
                        BOLD => push_to_stack(&mut braces_stack, &mut chars, InlineClearKind::Bold),
                        ITALIC => push_to_stack(&mut braces_stack, &mut chars, InlineClearKind::Italic),
                        STRIKE => push_to_stack(&mut braces_stack, &mut chars, InlineClearKind::Strike),
                        UNDER => push_to_stack(&mut braces_stack, &mut chars, InlineClearKind::Under),
                        MONO => push_to_stack(&mut braces_stack, &mut chars, InlineClearKind::Mono),
                        COLOR => {
                            push_to_stack(&mut braces_stack, &mut chars, InlineClearKind::Color(depth));
                            in_color = true;
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

pub(crate) fn clear_name_link<'a>(line: &'a str) -> Cow<'a, str> {
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




#[derive(Debug, PartialEq)]
enum Mark<'a> {
    Bold,
    Italic,
    Strike,
    Under,
    Mono,
    Color(Option<Color>, u32),
    Link(&'a str, u32)
}


#[derive(Debug)]
struct Opened<'a> {
    mark: Mark<'a>,
    start: usize,
}

#[derive(Debug, PartialEq)]
struct Range<'a> {
    mark: Mark<'a>,
    start: usize,
    end: usize
}

#[derive(Debug, PartialEq)]
enum InlineText<'a> {
    String(&'a str),
    InnerLink(Cow<'a, str>, RefKind),
    Note(u16),
    Space
}

#[derive(Debug, PartialEq)]
struct PreparsedLines<'a> {
    text: Vec<InlineText<'a>>,
    style_ranges: Vec<Range<'a>>
}

impl<'a> PreparsedLines<'a> {
    fn try_pop_stack(
        open_stack: &mut Vec<Opened<'a>>,
        is_target: impl Fn(&Opened) -> bool
    ) -> Option<Opened<'a>>{
        let pos = open_stack.iter().rposition(is_target)?;
        let target = open_stack.remove(pos);
        open_stack.truncate(pos);
        return Some(target);
    }

    fn push_range_to_out (
        out: &mut Self, 
        line: &'a str, 
        start:usize, 
        end: usize
    ) {
        if start < end {
             out.text.push(InlineText::String(&line[start..end])); 
        }
    }

    fn char_to_mark(ch: char) -> Option<Mark<'a>> {
        match ch {
            BOLD => Some(Mark::Bold),
            ITALIC => Some(Mark::Italic),
            STRIKE => Some(Mark::Strike), 
            UNDER => Some(Mark::Under),
            MONO => Some(Mark::Mono),
            _ => None
        }
    }

    fn brace_char_to_mark(line: &'a str, ch: char, brace_depth: u32) -> Mark<'a> {
        let line = &line[ch.len_utf8()..];
        match ch {
            COLOR => {
                let color = Color::from_unknown_string(line);
                Mark::Color(color, brace_depth)
            },
            LINK => Mark::Link(line, brace_depth),
            _ => unreachable!("only color an links can be here!")
        }
    }
    
    /// Finds the bracket that closes the name of an inner link `\*р(...)`, considering nesting and escaping.
    /// `start` - first byte after the opening bracket. If the bracket is not closed, the name runs to the end of the line.
    fn find_link_end(line: &str, start: usize) -> usize {
        let mut depth: u32 = 1;
        let mut chars = line[start..].char_indices().peekable();
        while let Some((j, ch)) = chars.next() {
            match ch {
                SHIELD => {
                    chars.next_if(|&(_, c)| is_inline_marking(c));
                },
                LBRACE => depth += 1,
                RBRACE => {
                    depth -= 1;
                    if depth == 0 {
                        return start + j;
                    }
                },
                _ => ()
            }
        }
        line.len()
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
            let mut brace_depth: u32 = 0;
            while let Some((i, ch)) = chars.next(){
                println!("{}, {}", i, ch);
                match ch {    
                    MARK => {
                        if let Some(&(next_i, next_ch)) = chars.peek() {
                            match next_ch {
                                COLOR | LINK => {
                                    match chars.find(|&(_, c)| c == LBRACE) {
                                        Some((brace_i, i_ch)) => {
                                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                            let mark = Self::brace_char_to_mark(&line[next_i..brace_i], next_ch, brace_depth);
                                            byte_after_last_tag = brace_i + i_ch.len_utf8();
                                            open_stack.push(
                                                Opened { mark, start: out.text.len()}
                                            );
                                            brace_depth += 1;
                                        }
                                        None => {
                                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                            byte_after_last_tag = line.len();
                                        }
                                    }
                                },
                                NOTE => {
                                    chars.next();
                                    Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                    if let Some(&(l_i, l_c)) = chars.peek() && 
                                       let Some(kind) = ref_kind(l_c) && line[l_i + l_c.len_utf8()..].starts_with(LBRACE)
                                    {
                                        let name_start = l_i + l_c.len_utf8() + LBRACE.len_utf8();
                                        let name_end = Self::find_link_end(line, name_start);
                                        while chars.next_if(|&(j, _)| j <= name_end).is_some() {}
                                        let name = clear_name_attribute(&line[name_start..name_end]);
                                        out.text.push(InlineText::InnerLink(name, kind));
                                        byte_after_last_tag = (name_end + RBRACE.len_utf8()).min(line.len());
                                    } else if let Some((start, _)) = chars.next_if(|&(_, c)| c.is_ascii_digit()) {
                                        let mut end = start + 1;
                                        while let Some(_) = chars.next_if(|&(_, c)| c.is_ascii_digit()){
                                            end += 1;
                                        }
                                        let n = line[start..end].parse::<u16>().unwrap_or(u16::MAX);
                                        out.text.push(InlineText::Note(n));
                                        if let Some(_) = chars.next_if(|&(_, c)| c == SPACE) {
                                            byte_after_last_tag = end + SPACE.len_utf8();    
                                        } else {
                                            byte_after_last_tag = end;
                                        }
                                    } else {
                                        byte_after_last_tag = i + MARK.len_utf8() + NOTE.len_utf8();
                                    }
                                }
                                _ => {
                                    if let Some(mark) = Self::char_to_mark(next_ch) {
                                        Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                        byte_after_last_tag = next_i + next_ch.len_utf8();
                                        open_stack.push(Opened { mark, start: out.text.len()});
                                        chars.next();
                                    } else {
                                        Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                        byte_after_last_tag = i + ch.len_utf8();
                                    }
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
                    LBRACE => {
                        brace_depth = brace_depth.saturating_add(1);
                    },
                    RBRACE => {
                        brace_depth = brace_depth.saturating_sub(1);
                        if let Some(opened) = Self::try_pop_stack(
                            &mut open_stack, |t| matches!(t.mark, Mark::Color(_, d) | Mark::Link(_, d) if d == brace_depth)
                        ) {
                            println!("Depth is zero also color in stack: {opened:?}");
                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                            byte_after_last_tag = i + ch.len_utf8();
                            out.style_ranges.push(
                                Range { mark: opened.mark, start: opened.start, end: out.text.len() }
                            );
                        }
                    },
                    _ => {
                        if let Some(mark) = Self::char_to_mark(ch) {
                            if let Some(opened) = Self::try_pop_stack(&mut open_stack, |t| t.mark == mark){ 
                                Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                byte_after_last_tag = i + ch.len_utf8();
                                out.style_ranges.push(
                                    Range { mark, start: opened.start, end: out.text.len() }
                                );
                            }
                        }
                    }
                }
                
            }
            Self::push_range_to_out(&mut out, line, byte_after_last_tag, line.len());
            for open in open_stack.into_iter().rev() {
                out.style_ranges.push(Range { mark: open.mark, start: open.start, end: out.text.len()});
            }
            out.text.push(InlineText::Space);
        }

        out
    }
}



#[derive(Debug, Default, Clone)]
struct StyleDiff {
    bold: i32,
    italic: i32,
    mono: i32,
    strike: i32,
    under: i32,
}
impl StyleDiff {
    fn is_zero(&self) -> bool {
        self.bold == 0 &&
        self.italic == 0 &&
        self.mono == 0 &&
        self.strike == 0 &&
        self.under == 0
    }

    fn to_style(&self) -> Style {
        Style {
            is_bold: self.bold > 0,
            is_italic: self.italic > 0,
            is_mono: self.mono > 0,
            is_strike: self.strike > 0,
            is_under: self.under > 0
        }
    }
}

impl Add for StyleDiff {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            bold: self.bold + rhs.bold,
            italic: self.italic + rhs.italic,
            mono: self.mono + rhs.mono,
            under: self.bold + rhs.bold,
            strike: self.strike + rhs.strike
        }
    }
}

impl AddAssign for StyleDiff {
    fn add_assign(&mut self, rhs: Self) {
        self.bold += rhs.bold;
        self.italic += rhs.italic;
        self.mono += rhs.mono;
        self.bold += rhs.bold;
        self.strike += rhs.strike;
    }
}

#[derive(Debug)]
struct Style {
    is_bold: bool,
    is_italic: bool,
    is_mono: bool,
    is_strike: bool,
    is_under: bool,
}
impl PartialEq for Style {
    fn eq(&self, other: &Self) -> bool {
        self.is_bold == other.is_bold &&
        self.is_italic == other.is_italic &&
        self.is_mono == other.is_mono &&
        self.is_strike == other.is_strike &&
        self.is_under == other.is_under
    }   
}


#[derive(Debug)]
struct Span<'a> {
    text: Vec<InlineText<'a>>,
    style: Style
}
impl<'a> Span<'a>{
    fn new(text: Vec<InlineText<'a>>, style: Style) -> Self {
        Self {text, style}
    }

    fn push(&mut self, line: InlineText<'a>) {
        self.text.push(line);
    }
}




fn generate_spans<'a>(preparsed: PreparsedLines<'a>) -> Vec<Span<'a>> {

    

    let mut difs: Vec<StyleDiff> = vec![StyleDiff::default(); preparsed.text.len()];
    
    for range in preparsed.style_ranges {
        match range.mark {
            Mark::Bold => {
                difs[range.start].bold += 1;
                difs[range.end].bold -= 1;
            },
            Mark::Italic => {
                difs[range.start].italic += 1;
                difs[range.end].italic -= 1;
            },
            Mark::Mono => {
                difs[range.start].mono += 1;
                difs[range.end].mono -= 1;
            },
            Mark::Strike => {
                difs[range.start].strike += 1;
                difs[range.end].strike -= 1;
            },
            Mark::Under => {
                difs[range.start].under += 1;
                difs[range.end].under -= 1;
            },
            _ => {

            }
        }
    }

    let mut out:Vec<Span> = Vec::new();
    let mut counter = StyleDiff::default();
    for (text, delta) in preparsed.text.into_iter().zip(difs){
        counter += delta;
        if let Some(last) = out.last_mut() && counter.to_style() == last.style{
            last.push(text);
        } else {
            out.push(
                Span::new(vec![text], counter.to_style())
            );
        }
    }

    println!("out is {:?}", out);

    return out;
}



#[cfg(test)] mod test {
    use std::{borrow::Cow, vec};

    use crate::parse::line_parse::*;

    fn check(input: &str, expected: &str) {
        assert_eq!(clear_name_attribute(input), expected);
    }

    #[test]
    fn main() {
        let out = PreparsedLines::new(vec!["aa\\: bb\\cc:ee"]);

        println!();
        println!("---RESULT---");
        println!("{out:?}");
        println!();


        println!("---RESULT 2---");
        generate_spans(out);
        println!();
    }


    #[test]
    fn check_line_preparsed_inner_link() {
        let out = PreparsedLines::new(vec!["раз\\*р(ЖОПА) раз"]);
        let expected = PreparsedLines {
            text: vec![
                InlineText::String("раз"),
                InlineText::InnerLink(Cow::Borrowed("жопа"), RefKind::Image),
                InlineText::String(" раз")
            ],
            style_ranges: vec![]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["см. \\*л(\\:Пример: Кода)"]);
        let expected = PreparsedLines {
            text: vec![
                InlineText::String("см. "),
                InlineText::InnerLink(Cow::Borrowed("пример кода"), RefKind::Code),
            ],
            style_ranges: vec![]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\*ф(F(x) и y_))!"]);
        let expected = PreparsedLines {
            text: vec![
                InlineText::InnerLink(Cow::Borrowed("f(x) и y)"), RefKind::Formula),
                InlineText::String("!"),
            ],
            style_ranges: vec![]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\*т(\\%КРАС(Ёлка))"]);
        let expected = PreparsedLines {
            text: vec![InlineText::InnerLink(Cow::Borrowed("елка"), RefKind::Table)],
            style_ranges: vec![]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\*т(Таблица"]);
        let expected = PreparsedLines {
            text: vec![InlineText::InnerLink(Cow::Borrowed("таблица"), RefKind::Table)],
            style_ranges: vec![]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\:а \\*р(Б) в:"]);
        let expected = PreparsedLines {
            text: vec![
                InlineText::String("а "),
                InlineText::InnerLink(Cow::Borrowed("б"), RefKind::Image),
                InlineText::String(" в"),
            ],
            style_ranges: vec![Range{mark: Mark::Bold, start: 0, end: 3}]
        };
        assert_eq!(out, expected);
    }

    #[test]
    fn check_line_preparsed_line_common() {
        let out = PreparsedLines::new(vec!["\\:a:"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Bold, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\:a:b\\;c;"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("a"), InlineText::String("b"), InlineText::String("c")],
            style_ranges: vec![Range{mark: Mark::Bold, start: 0, end: 1}, Range{mark: Mark::Italic, start: 2, end: 3}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\;:x;"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String(":x")],
            style_ranges: vec![Range{mark: Mark::Italic, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\;\\:a:;"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Bold, start: 0, end: 1}, Range{mark: Mark::Italic, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        // ===== Mono =====

        let out = PreparsedLines::new(vec!["\\!a!"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Mono, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\!a!b\\-c-"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("a"), InlineText::String("b"), InlineText::String("c")],
            style_ranges: vec![Range{mark: Mark::Mono, start: 0, end: 1}, Range{mark: Mark::Strike, start: 2, end: 3}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\!-x!"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("-x")],
            style_ranges: vec![Range{mark: Mark::Mono, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\!\\=a=!"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Under, start: 0, end: 1}, Range{mark: Mark::Mono, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        // ===== Strike =====

        let out = PreparsedLines::new(vec!["\\-a-"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Strike, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\-a-b\\=c="]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("a"), InlineText::String("b"), InlineText::String("c")],
            style_ranges: vec![Range{mark: Mark::Strike, start: 0, end: 1}, Range{mark: Mark::Under, start: 2, end: 3}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\-!x-"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("!x")],
            style_ranges: vec![Range{mark: Mark::Strike, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\-\\!a!-"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Mono, start: 0, end: 1}, Range{mark: Mark::Strike, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        // ===== Under =====

        let out = PreparsedLines::new(vec!["\\=a="]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("a")],
            style_ranges: vec![Range{mark: Mark::Under, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\=a=b\\!c!"]);
        let expected = PreparsedLines {
            text: vec![InlineText::String("a"), InlineText::String("b"), InlineText::String("c")],
            style_ranges: vec![Range{mark: Mark::Under, start: 0, end: 1}, Range{mark: Mark::Mono, start: 2, end: 3}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\=:x="]);
        let expected = PreparsedLines {
            text: vec![InlineText::String(":x")],
            style_ranges: vec![Range{mark: Mark::Under, start: 0, end: 1}]
        };
        assert_eq!(out, expected);

        let out = PreparsedLines::new(vec!["\\=\\-a-="]);
        let expected = PreparsedLines {
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
