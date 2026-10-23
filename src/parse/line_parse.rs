use std::borrow::Cow;
use std::iter::Peekable;
use std::ops::AddAssign;
use std::str::CharIndices;

use crate::parse::{lexemas::*, common::*};

impl Color {
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

    fn from_hex(col: &str) -> Option<Self>{
        let hex = col.strip_prefix('#').unwrap_or(col);
        Self::from_iterator( hex.chars().map(|c| c.to_digit(16).map(|d| d as u8)))
    }

    fn from_ru_hex(col: &str) -> Option<Self >{
        let hex = col.strip_prefix(['ц', 'Ц']).unwrap_or(col);
        Self::from_iterator(hex.chars().map(|c| Self::ru_hex_handler(c)))
    }

    fn from_digits(col: &str) -> Option<Self> {
        let mut digits = col.chars().map(|c| c.to_digit(10));
        let mut next_byte = || {
            let val = digits.next()?? * 100 + digits.next()?? * 10 + digits.next()??;
            u8::try_from(val).ok()
        };
        match (next_byte(), next_byte(), next_byte()) {
            (Some(r), Some(g), Some(b)) => {
                if digits.next().is_none() {
                    return  Some(Self::new(r, g, b));
                }
                None
            },
            _ => None
        }
    }

    fn from_iterator(mut digits: impl Iterator<Item = Option<u8>>) -> Option<Self> {
        let mut next_byte = || Some(digits.next()?? * 16 + digits.next()??);
        let (r, g, b) = (next_byte()?, next_byte()?, next_byte()?);
        if digits.next().is_none(){
            return  Some(Self::new(r, g, b));
        } else {
            return None;
        }
        
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


#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum InlineContext {
    Text,
    TableCell,
    Name,
    ImageName,
}

impl InlineContext {
    pub(crate) fn is_name(self) -> bool {
        matches!(self, Self::Name | Self::ImageName)
    }
}

fn is_inline_marking(ch: char, ctx: InlineContext) -> bool {
    matches!(
        ch,
        MARK | SHIELD | BOLD | ITALIC | STRIKE | UNDER | MONO | COLOR | LBRACE | RBRACE
    ) || match ctx {
        InlineContext::TableCell => ch == TABLE_SEP,
        InlineContext::ImageName => ch == IMG_SEP,
        _ => false
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum InlineClearKind {
    Bold,
    Italic,
    Strike,
    Under,
    Mono,
    Braced(u32),
}
pub(crate) fn clear_name_attribute<'a>(line: &'a str) -> Cow<'a, str> {
    clear_name_attribute_in(line, InlineContext::Text)
}

fn collapse_blanks<'a>(line: Cow<'a, str>) -> Cow<'a, str> {
    let is_collapsed = !line.contains(TAB)
        && !line.contains("  ")
        && !line.starts_with(is_blank)
        && !line.ends_with(is_blank);
    if is_collapsed {
        return line;
    }
    Cow::Owned(line.split(is_blank).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" "))
}


pub(crate) fn clear_name_attribute_in<'a>(line: &'a str, ctx: InlineContext) -> Cow<'a, str> {
    let Some(first) = line.find(is_should_be_changed_on_name) else {
        return collapse_blanks(Cow::Borrowed(line));
    };

    let mut out = String::with_capacity(line.len());
    out.push_str(&line[..first]);

    let mut chars = line[first..].chars().peekable();
    let mut braces_stack: Vec<InlineClearKind> = Vec::new();
    let mut depth: u32 = 0;
    let mut in_param = false;

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

    'outer: while let Some(ch) = chars.next() {
        match ch {
            SHIELD => {
                if let Some(next) = chars.next_if(|&c| is_inline_marking(c, ctx)) {
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
                            push_to_stack(&mut braces_stack, &mut chars, InlineClearKind::Braced(depth));
                            in_param = true;
                        },
                        LINK => {
                            push_to_stack(&mut braces_stack, &mut chars, InlineClearKind::Braced(depth));
                            if chars.next_if_eq(&QUOTES).is_some() {
                                if !chars.by_ref().any(|c| c == QUOTES) || chars.next_if_eq(&LBRACE).is_none() {
                                    break 'outer;
                                }
                                depth += 1;
                            } else {
                                in_param = true;
                            }
                        },
                        NOTE => {
                            chars.next();
                            let mut ahead = chars.clone();
                            if let Some(kind) = ahead.next() && ref_kind(kind).is_some() && ahead.next() == Some(LBRACE) {
                                chars = ahead;
                                let mut ref_depth: u32 = 1;
                                while let Some(c) = chars.next() {
                                    match c {
                                        SHIELD => { chars.next_if(|&c| is_inline_marking(c, ctx)); },
                                        LBRACE => ref_depth += 1,
                                        RBRACE => {
                                            ref_depth -= 1;
                                            if ref_depth == 0 {
                                                break;
                                            }
                                        },
                                        _ => ()
                                    }
                                }
                            } else {
                                let mut digits = String::new();
                                while let Some(d) = chars.next_if(|c| c.is_ascii_digit()) {
                                    digits.push(d);
                                }
                                if !digits.is_empty() && digits.parse::<u16>().is_err() {
                                    out.push(NOTE);
                                    out.push_str(&digits);
                                }
                            }
                        },
                        _ => ()
                    }
                }
            },
            _ => {
                if in_param {
                    if ch == LBRACE{
                        in_param = false;
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
                        RBRACE => {
                            depth = depth.saturating_sub(1);
                            let braced = braces_stack.iter().rposition(|e| matches!(e, InlineClearKind::Braced(_)));
                            match braced {
                                Some(pos) if braces_stack[pos] == InlineClearKind::Braced(depth) => braces_stack.truncate(pos),
                                _ => out.push(ch),
                            }
                        },
                        SMALL_YO => out.push(SMALL_E),
                        BOLD => push_closer(&mut braces_stack, &mut out, InlineClearKind::Bold, &ch),
                        ITALIC => push_closer(&mut braces_stack, &mut out, InlineClearKind::Italic, &ch),
                        MONO => push_closer(&mut braces_stack, &mut out, InlineClearKind::Mono, &ch),
                        STRIKE => push_closer(&mut braces_stack, &mut out, InlineClearKind::Strike, &ch),
                        UNDER => push_closer(&mut braces_stack, &mut out, InlineClearKind::Under, &ch),
                        _ => out.push(ch)
                    }
                }
            }
        }
    }
    return collapse_blanks(Cow::Owned(out));
}

pub(crate) fn parse_note_line(line: &str) -> Option<(u16, &str)> {
    let rest = NOTE_TAG.strip(line.strip_prefix(MARK)?)?;
    let (num, text) = rest.split_at(rest.find(is_blank)?);
    let text = text.trim_matches(is_blank);
    if num.is_empty() || !num.bytes().all(|b| b.is_ascii_digit()) || text.is_empty() {
        return None;
    }
    num.parse::<u16>().ok().map(|n| (n, text))
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
    Color(Option<Color>),
    Link(&'a str)
}


#[derive(Debug)]
struct Opened<'a> {
    mark: Mark<'a>,
    start: usize,
    brace_depth: Option<u32>,
}

#[derive(Debug, PartialEq)]
struct Range<'a> {
    mark: Mark<'a>,
    start: usize,
    end: usize
}

#[derive(Debug, PartialEq)]
pub(crate) enum InlinePreEl<'a> {
    String(&'a str),
    InnerLink(Cow<'a, str>, RefKind),
    Note(u16),
    Space,
    LongDash
}

#[derive(Debug, PartialEq)]
pub(crate)  struct PreparsedLines<'a> {
    text: Vec<InlinePreEl<'a>>,
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
             out.text.push(InlinePreEl::String(&line[start..end])); 
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

    fn color_head(line: &'a str, chars: &mut Peekable<CharIndices<'a>>, value_start: usize) -> Option<(Mark<'a>, usize)> {
        let (brace_i, _) = chars.find(|&(_, c)| c == LBRACE)?;
        let color = Color::from_unknown_string(&line[value_start..brace_i]);
        Some((Mark::Color(color), brace_i + LBRACE.len_utf8()))
    }

    fn link_head(line: &'a str, chars: &mut Peekable<CharIndices<'a>>, value_start: usize) -> Option<(Mark<'a>, usize)> {
        if line[value_start..].starts_with(QUOTES) {
            chars.nth(1);
            let link_start = value_start + QUOTES.len_utf8();
            let (quote_i, _) = chars.find(|&(_, c)| c == QUOTES)?;
            let (brace_i, _) = chars.next_if(|&(_, c)| c == LBRACE)?;
            Some((Mark::Link(&line[link_start..quote_i]), brace_i + LBRACE.len_utf8()))
        } else {
            let (brace_i, _) = chars.find(|&(_, c)| c == LBRACE)?;
            Some((Mark::Link(line[value_start..brace_i].trim_matches(is_blank)), brace_i + LBRACE.len_utf8()))
        }
    }

    fn find_link_end(line: &str, start: usize, ctx: InlineContext) -> usize {
        let mut depth: u32 = 1;
        let mut chars = line[start..].char_indices().peekable();
        while let Some((j, ch)) = chars.next() {
            match ch {
                SHIELD => {
                    chars.next_if(|&(_, c)| is_inline_marking(c, ctx));
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

    pub(crate) fn new(lines: Vec<&'a str>, ctx: InlineContext) -> Self {
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
                match ch {    
                    MARK => {
                        if let Some(&(next_i, next_ch)) = chars.peek() {
                            match next_ch {
                                COLOR | LINK => {
                                    let value_start = next_i + next_ch.len_utf8();
                                    let head = match next_ch {
                                        COLOR => Self::color_head(line, &mut chars, value_start),
                                        _ => Self::link_head(line, &mut chars, value_start),
                                    };
                                    match head {
                                        Some((mark, after_brace)) => {
                                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                            byte_after_last_tag = after_brace;
                                            open_stack.push(
                                                Opened { mark, start: out.text.len(), brace_depth: Some(brace_depth)}
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
                                        let name_end = Self::find_link_end(line, name_start, ctx);
                                        while chars.next_if(|&(j, _)| j <= name_end).is_some() {}
                                        let name = clear_name_attribute_in(&line[name_start..name_end], ctx);
                                        out.text.push(InlinePreEl::InnerLink(name, kind));
                                        byte_after_last_tag = (name_end + RBRACE.len_utf8()).min(line.len());
                                    } else if let Some((start, _)) = chars.next_if(|&(_, c)| c.is_ascii_digit()) {
                                        let mut end = start + 1;
                                        while let Some(_) = chars.next_if(|&(_, c)| c.is_ascii_digit()){
                                            end += 1;
                                        }
                                        if let Ok(n) = line[start..end].parse::<u16>() {
                                            out.text.push(InlinePreEl::Note(n));
                                            if let Some(_) = chars.next_if(|&(_, c)| c == SPACE) {
                                                byte_after_last_tag = end + SPACE.len_utf8();
                                            } else {
                                                byte_after_last_tag = end;
                                            }
                                        } else {
                                            byte_after_last_tag = i + MARK.len_utf8();
                                        }
                                    } else {
                                        byte_after_last_tag = i + MARK.len_utf8() + NOTE.len_utf8();
                                    }
                                }
                                _ => {
                                    if let Some(mark) = Self::char_to_mark(next_ch) {
                                        Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                        byte_after_last_tag = next_i + next_ch.len_utf8();
                                        open_stack.push(Opened { mark, start: out.text.len(), brace_depth: None});
                                        chars.next();
                                    } else {
                                        Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                                        byte_after_last_tag = i + ch.len_utf8();
                                    }
                                }
                            }
                        } else {
                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                            byte_after_last_tag = line.len();
                        }
                    },
                    SHIELD => {
                        if chars.next_if(|&(_, c)| is_inline_marking(c, ctx)).is_some() {
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
                            &mut open_stack, |t| t.brace_depth == Some(brace_depth)
                        ) {
                            Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                            byte_after_last_tag = i + ch.len_utf8();
                            out.style_ranges.push(
                                Range { mark: opened.mark, start: opened.start, end: out.text.len() }
                            );
                        }
                    },
                    DASH if chars.next_if(|&(_, c)| c == DASH).is_some() => {
                        Self::push_range_to_out(&mut out, line, byte_after_last_tag, i);
                        out.text.push(InlinePreEl::LongDash);
                        byte_after_last_tag = i + DASH.len_utf8() + DASH.len_utf8();
                    }
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
            out.text.push(InlinePreEl::Space);
            
        }

        out
    }
}



#[derive(Debug, Default, Clone)]
struct PreStyleDiff<'a> {
    bold: i32,
    italic: i32,
    mono: i32,
    strike: i32,
    under: i32,
    color_open: i32,
    color_close: i32,
    color: Option<Color>,
    link_open: i32,
    link_close: i32,
    link: Option<&'a str>
    
}
impl<'a> PreStyleDiff<'a> {
    fn to_style(&self) -> PreStyle<'a> {
        PreStyle {
            is_bold: self.bold > 0,
            is_italic: self.italic > 0,
            is_mono: self.mono > 0,
            is_strike: self.strike > 0,
            is_under: self.under > 0,
            color: self.color.clone(),
            link: self.link.clone(),
        }
    }
}

impl<'a> AddAssign for PreStyleDiff<'a> {
    fn add_assign(&mut self, rhs: Self) {
        self.bold += rhs.bold;
        self.italic += rhs.italic;
        self.mono += rhs.mono;
        self.under += rhs.under;
        self.strike += rhs.strike;

        self.color_open -= rhs.color_close;
        if self.color_open <= 0 {
            self.color = rhs.color;
        }
        self.color_open += rhs.color_open;

        self.link_open -= rhs.link_close;
        if self.link_open <= 0 {
            self.link = rhs.link;
        }
        self.link_open += rhs.link_open
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct PreStyle<'a> {
    pub(crate) is_bold: bool,
    pub(crate) is_italic: bool,
    pub(crate) is_mono: bool,
    pub(crate) is_strike: bool,
    pub(crate) is_under: bool,
    pub(crate) color: Option<Color>,
    pub(crate) link: Option<&'a str>
}


#[derive(Debug)]
pub(crate) struct PreSpan<'a> {
    pub(crate) text: Vec<InlinePreEl<'a>>,
    pub(crate) style: PreStyle<'a>
}
impl<'a> PreSpan<'a>{
    fn new(text: Vec<InlinePreEl<'a>>, style: PreStyle<'a>) -> Self {
        Self {text, style}
    }

    fn push(&mut self, line: InlinePreEl<'a>) {
        self.text.push(line);
    }
}




pub(crate) fn generate_pre_spans<'a>(preparsed: PreparsedLines<'a>) -> Vec<PreSpan<'a>> {
    let mut difs: Vec<PreStyleDiff> = vec![PreStyleDiff::default(); preparsed.text.len()];
    
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
            Mark::Color(color) if range.start < range.end => {
                difs[range.start].color_open += 1;
                difs[range.start].color = color;
                difs[range.end].color_close += 1;
            },
            Mark::Link(link) if range.start < range.end => {
                difs[range.start].link_open += 1;
                difs[range.start].link = Some(link);
                difs[range.end].link_close += 1;
            },
            _ => ()
        }
    }

    let mut out:Vec<PreSpan> = Vec::new();
    let mut counter = PreStyleDiff::default();
    let mut items = preparsed.text.into_iter().zip(difs).peekable();
    while let Some((text, delta)) = items.next() {
        if let Some(last) = out.last_mut() {
            match text {
                InlinePreEl::Space => {
                    if let Some(next_style) = items.peek().map(|(_, d)| d) {
                        let next_style = next_style.to_style();
                        if counter.to_style() == next_style {
                            last.push(text);
                        } else {
                            counter += delta;
                            out.push(PreSpan::new(vec![text], counter.to_style()));
                        }
                    }
                },
                _ => {
                    counter += delta;
                    if counter.to_style() == last.style{
                        last.push(text);
                    } else {
                        out.push(PreSpan::new(vec![text], counter.to_style()));
                    }
                }
            }
        } else {
            counter += delta;
            out.push(PreSpan::new(vec![text], counter.to_style()));
        }
        
        
    }


    return out;
}



#[cfg(test)] mod test {
    use std::{borrow::Cow, vec};

    use crate::parse::line_parse::*;


    fn check(input: &str, expected: &str) {
        assert_eq!(clear_name_attribute(input), expected);
    }

    use super::InlinePreEl::{Note, Space};

    fn s(text: &str) -> InlinePreEl<'_> {
        InlinePreEl::String(text)
    }

    fn inner(name: &str, kind: RefKind) -> InlinePreEl<'_> {
        InlinePreEl::InnerLink(Cow::Borrowed(name), kind)
    }

    fn r(mark: Mark<'_>, start: usize, end: usize) -> Range<'_> {
        Range { mark, start, end }
    }

    fn red() -> Option<Color> { Some(Color::RED) }
    fn blue() -> Option<Color> { Some(Color::DARK_BLUE) }

    fn check_pre<'a>(lines: Vec<&'a str>, text: Vec<InlinePreEl<'a>>, style_ranges: Vec<Range<'a>>) {
        let out = PreparsedLines::new(lines.clone(), InlineContext::Text);
        assert_eq!(out, PreparsedLines { text, style_ranges }, "вход: {lines:?}");
    }

    fn check_color(param: &str, color: Option<Color>) {
        let line = format!("\\%{param}(x)");
        check_pre(vec![&line], vec![s("x"), Space], vec![r(Mark::Color(color), 0, 1)]);
    }

    fn plain() -> PreStyle<'static> {
        PreStyle {
            is_bold: false, is_italic: false, is_mono: false, is_strike: false, is_under: false,
            color: None, link: None
        }
    }

    fn bold() -> PreStyle<'static> {
        PreStyle { is_bold: true, ..plain() }
    }

    fn check_spans<'a>(lines: Vec<&'a str>, expected: Vec<(Vec<InlinePreEl<'a>>, PreStyle<'a>)>) {
        let out: Vec<_> = generate_pre_spans(PreparsedLines::new(lines.clone(), InlineContext::Text))
            .into_iter()
            .map(|span| (span.text, span.style))
            .collect();
        assert_eq!(out, expected, "input (left - out, right - expected): {lines:?}");
    }

    // ===== PreparsedLines =====
    #[test]
    fn check_line_preparsed_inner_link() {
        check_pre(vec!["раз\\*р(ЖОПА) раз"], vec![s("раз"), inner("жопа", RefKind::Image), s(" раз"), Space], vec![]);
        check_pre(vec!["см. \\*л(\\:Пример: Кода)"], vec![s("см. "), inner("пример кода", RefKind::Code), Space], vec![]);
        check_pre(vec!["\\*ф(F(x) и y_))!"], vec![inner("f(x) и y)", RefKind::Formula), s("!"), Space], vec![]);
        check_pre(vec!["\\*т(\\%КРАС(Ёлка))"], vec![inner("елка", RefKind::Table), Space], vec![]);
        check_pre(vec!["\\*т(Таблица"], vec![inner("таблица", RefKind::Table), Space], vec![]);
        check_pre(
            vec!["\\:а \\*р(Б) в:"],
            vec![s("а "), inner("б", RefKind::Image), s(" в"), Space],
            vec![r(Mark::Bold, 0, 3)]
        );
        check_pre(vec!["\\*р(Схема (вариант 2))"], vec![inner("схема (вариант 2)", RefKind::Image), Space], vec![]);
        check_pre(vec!["\\*р(Шаг 1_) настройка)"], vec![inner("шаг 1) настройка", RefKind::Image), Space], vec![]);
    }

    #[test]
    fn check_line_preparsed_note() {
        check_pre(vec!["баренцово\\*12море"], vec![s("баренцово"), Note(12), s("море"), Space], vec![]);
        check_pre(vec!["война\\*12 1912 года"], vec![s("война"), Note(12), s("1912 года"), Space], vec![]);
        check_pre(vec!["\\*7"], vec![Note(7), Space], vec![]);
        check_pre(vec!["\\:см. \\*3:"], vec![s("см. "), Note(3), Space], vec![r(Mark::Bold, 0, 2)]);
    }

    #[test]
    fn check_line_preparsed_line_common() {
        check_pre(vec!["\\:a:"], vec![s("a"), Space], vec![r(Mark::Bold, 0, 1)]);
        check_pre(vec!["\\:a:b\\;c;"], vec![s("a"), s("b"), s("c"), Space], vec![r(Mark::Bold, 0, 1), r(Mark::Italic, 2, 3)]);
        check_pre(vec!["\\;:x;"], vec![s(":x"), Space], vec![r(Mark::Italic, 0, 1)]);
        check_pre(vec!["\\;\\:a:;"], vec![s("a"), Space], vec![r(Mark::Bold, 0, 1), r(Mark::Italic, 0, 1)]);

        // ===== Mono =====
        check_pre(vec!["\\!a!"], vec![s("a"), Space], vec![r(Mark::Mono, 0, 1)]);
        check_pre(vec!["\\!a!b\\-c-"], vec![s("a"), s("b"), s("c"), Space], vec![r(Mark::Mono, 0, 1), r(Mark::Strike, 2, 3)]);
        check_pre(vec!["\\!-x!"], vec![s("-x"), Space], vec![r(Mark::Mono, 0, 1)]);
        check_pre(vec!["\\!\\=a=!"], vec![s("a"), Space], vec![r(Mark::Under, 0, 1), r(Mark::Mono, 0, 1)]);

        // ===== Strike =====
        check_pre(vec!["\\-a-"], vec![s("a"), Space], vec![r(Mark::Strike, 0, 1)]);
        check_pre(vec!["\\-a-b\\=c="], vec![s("a"), s("b"), s("c"), Space], vec![r(Mark::Strike, 0, 1), r(Mark::Under, 2, 3)]);
        check_pre(vec!["\\-!x-"], vec![s("!x"), Space], vec![r(Mark::Strike, 0, 1)]);
        check_pre(vec!["\\-\\!a!-"], vec![s("a"), Space], vec![r(Mark::Mono, 0, 1), r(Mark::Strike, 0, 1)]);

        // ===== Under =====
        check_pre(vec!["\\=a="], vec![s("a"), Space], vec![r(Mark::Under, 0, 1)]);
        check_pre(vec!["\\=a=b\\!c!"], vec![s("a"), s("b"), s("c"), Space], vec![r(Mark::Under, 0, 1), r(Mark::Mono, 2, 3)]);
        check_pre(vec!["\\=:x="], vec![s(":x"), Space], vec![r(Mark::Under, 0, 1)]);
        check_pre(vec!["\\=\\-a-="], vec![s("a"), Space], vec![r(Mark::Strike, 0, 1), r(Mark::Under, 0, 1)]);

        check_pre(vec!["10:30; итог - 5 = 5!"], vec![s("10:30; итог - 5 = 5!"), Space], vec![]);
    }

    #[test]
    fn check_line_preparsed_long_dash() {
        use super::InlinePreEl::LongDash;
        check_pre(vec!["a -- b"], vec![s("a "), LongDash, s(" b"), Space], vec![]);
        check_pre(vec!["--"], vec![LongDash, Space], vec![]);
        check_pre(vec!["a---b"], vec![s("a"), LongDash, s("-b"), Space], vec![]);
        check_pre(vec!["кто-то"], vec![s("кто-то"), Space], vec![]);
        check_pre(
            vec!["\\-a -- b-"],
            vec![s("a "), LongDash, s(" b"), Space],
            vec![r(Mark::Strike, 0, 3)]
        );
        check_pre(vec!["a_-_-b"], vec![s("a"), s("-"), s("-b"), Space], vec![]);
    }

    #[test]
    fn check_line_preparsed_shield() {
        check_pre(vec!["\\:10_:30:"], vec![s("10"), s(":30"), Space], vec![r(Mark::Bold, 0, 2)]);
        check_pre(vec!["a__:"], vec![s("a"), s("_:"), Space], vec![]);
        check_pre(vec!["_\\:"], vec![s("\\:"), Space], vec![]);
        check_pre(vec!["snake_case"], vec![s("snake_case"), Space], vec![]);
        check_pre(vec!["имя_"], vec![s("имя_"), Space], vec![]);
    }

    #[test]
    fn check_line_preparsed_nesting() {
        check_pre(vec!["\\:a \\:b: c:"], vec![s("a "), s("b"), s(" c"), Space], vec![r(Mark::Bold, 1, 2), r(Mark::Bold, 0, 3)]);
        check_pre(vec!["\\:a \\;b: c;"], vec![s("a "), s("b"), s(" c;"), Space], vec![r(Mark::Bold, 0, 2)]);
        check_pre(vec!["\\:незакрытый"], vec![s("незакрытый"), Space], vec![r(Mark::Bold, 0, 1)]);
        check_pre(vec!["\\:a \\;b c"], vec![s("a "), s("b c"), Space], vec![r(Mark::Italic, 1, 2), r(Mark::Bold, 0, 2)]);
        check_pre(
            vec!["\\:x \\%КРАС(a \\%СИН(b) c: d)"],
            vec![s("x "), s("a "), s("b"), s(" c"), s(" d)"), Space],
            vec![r(Mark::Color(blue()), 2, 3), r(Mark::Bold, 0, 4)]
        );
    }

    #[test]
    fn check_line_preparsed_multiline() {
        check_pre(vec!["\\:a", "b:"], vec![s("a"), Space, s("b:"), Space], vec![r(Mark::Bold, 0, 1)]);
        check_pre(vec!["\\%КРАС(a", "b)"], vec![s("a"), Space, s("b)"), Space], vec![r(Mark::Color(red()), 0, 1)]);
    }

    #[test]
    fn check_line_preparsed_color() {
        check_color("КРАС", red());
        check_color("ЖЁЛ", Some(Color::YELLOW));
        check_color("ЧЁРН", Some(Color::BLACK));
        check_color("БИР", Some(Color::CYAN));
        check_color("#1b2b20", Some(Color::new(0x1b, 0x2b, 0x20)));
        check_color("#ABCDEF", Some(Color::new(0xab, 0xcd, 0xef)));
        check_color("ц1б2б20", Some(Color::new(0x1b, 0x2b, 0x20)));
        check_color("Ц1Б2Б20", Some(Color::new(0x1b, 0x2b, 0x20)));
        check_color("цабвгде", Some(Color::new(0xab, 0xcd, 0xef)));
        check_color("255103020", Some(Color::new(255, 103, 20)));
        check_color("XYZ", None);
        check_color("", None);
        check_color("#1b2b2", None);
        check_color("ц1б2б2", None);
        check_color("25510302", None);
        check_color("256000000", None);
    }

    #[test]
    fn check_line_preparsed_color_exact_digits() {
        check_color("#1b2b20ff", None);
        check_color("ц1б2б200", None);
        check_color("2551030201", None);
    }

    #[test]
    fn check_line_preparsed_braces() {
        check_pre(vec!["\\%КРАС(f(x)) y"], vec![s("f(x)"), s(" y"), Space], vec![r(Mark::Color(red()), 0, 1)]);
        check_pre(vec!["\\%КРАС(a_)b) c"], vec![s("a"), s(")b"), s(" c"), Space], vec![r(Mark::Color(red()), 0, 2)]);
        check_pre(vec!["a) \\%КРАС(b)"], vec![s("a) "), s("b"), Space], vec![r(Mark::Color(red()), 1, 2)]);
        check_pre(vec!["\\%КРАС(a"], vec![s("a"), Space], vec![r(Mark::Color(red()), 0, 1)]);
        check_pre(vec!["до \\%КРАС после"], vec![s("до "), Space], vec![]);
        check_pre(vec!["до \\,a.ru после"], vec![s("до "), Space], vec![]);
    }

    #[test]
    fn check_line_preparsed_link() {
        check_pre(vec!["\\,a.ru(текст)"], vec![s("текст"), Space], vec![r(Mark::Link("a.ru"), 0, 1)]);
        check_pre(
            vec!["\\,google.com(google(гугл_))"],
            vec![s("google(гугл"), s("))"), Space],
            vec![r(Mark::Link("google.com"), 0, 2)]
        );
        check_pre(
            vec!["\\,a.ru(x \\,b.ru(y) z)"],
            vec![s("x "), s("y"), s(" z"), Space],
            vec![r(Mark::Link("b.ru"), 1, 2), r(Mark::Link("a.ru"), 0, 3)]
        );
    }

    #[test]
    fn check_line_preparsed_link_quoted() {
        check_pre(
            vec!["\\,\"www.link_with_braces.com/page(low)\"(текст)"],
            vec![s("текст"), Space],
            vec![r(Mark::Link("www.link_with_braces.com/page(low)"), 0, 1)]
        );
        check_pre(
            vec!["до \\,\"a.ru/x)\"(y) после"],
            vec![s("до "), s("y"), s(" после"), Space],
            vec![r(Mark::Link("a.ru/x)"), 1, 2)]
        );
        check_pre(
            vec!["\\,a.ru(x \\,\"b.ru/(1)\"(y) z)"],
            vec![s("x "), s("y"), s(" z"), Space],
            vec![r(Mark::Link("b.ru/(1)"), 1, 2), r(Mark::Link("a.ru"), 0, 3)]
        );
        check_pre(vec!["до \\,\"a.ru(x)"], vec![s("до "), Space], vec![]);
        check_pre(vec!["до \\,\"a.ru\" после"], vec![s("до "), Space], vec![]);
        check_pre(vec!["до \\,\"a.ru\" (x) после"], vec![s("до "), Space], vec![]);
        check_pre(vec!["до \\,\"a.ru\"b(x) после"], vec![s("до "), Space], vec![]);
    }

    #[test]
    fn check_line_spans_link_quoted() {
        check_spans(vec!["\\,\"a.ru/(1)\"(x)"], vec![(vec![s("x")], PreStyle { link: Some("a.ru/(1)"), ..plain() })]);
    }

    #[test]
    fn check_line_preparsed_unknown_tag() {
        check_pre(vec!["\\Abc"], vec![s("Abc"), Space], vec![]);
        check_pre(vec!["a\\Яb"], vec![s("a"), s("Яb"), Space], vec![]);
        check_pre(vec!["\\\\:x:"], vec![s("x"), Space], vec![r(Mark::Bold, 0, 1)]);
    }

    #[test]
    fn check_line_preparsed_invalid_outer_color() {
        check_pre(
            vec!["\\%XYZ(a \\%СИН(b) c)"],
            vec![s("a "), s("b"), s(" c"), Space],
            vec![r(Mark::Color(blue()), 1, 2), r(Mark::Color(None), 0, 3)]
        );
    }

    // ===== Span =====

    #[test]
    fn check_line_spans_invalid_outer_color() {
        check_spans(vec!["\\%XYZ(a \\%СИН(b) c)"], vec![(vec![s("a "), s("b"), s(" c")], plain())]);
        check_spans(vec!["\\%XYZ(\\%СИН(x))"], vec![(vec![s("x")], plain())]);
        check_spans(vec!["\\%XYZ(\\:a: \\%СИН(b))"], vec![
            (vec![s("a")], bold()),
            (vec![s(" "), s("b")], plain()),
        ]);
    }

    #[test]
    fn check_line_spans_format() {
        check_spans(vec![], vec![]);
        check_spans(vec![""], vec![(vec![Space], plain())]);
        check_spans(vec!["\\:a \\:b: c:"], vec![(vec![s("a "), s("b"), s(" c")], bold())]);
        check_spans(vec!["\\:a \\;b: c;"], vec![(vec![s("a "), s("b")], bold()), (vec![s(" c;")], plain())]);
        check_spans(vec!["\\:a \\;b c"], vec![
            (vec![s("a ")], bold()),
            (vec![s("b c")], PreStyle { is_italic: true, ..bold() }),
        ]);
        check_spans(vec!["\\=a="], vec![(vec![s("a")], PreStyle { is_under: true, ..plain() })]);
        check_spans(vec!["\\:\\;\\-\\=\\!x!=-;:"], vec![
            (vec![s("x")], PreStyle { is_bold: true, is_italic: true, is_mono: true, is_strike: true, is_under: true, color: None, link: None }),
        ]);
        check_spans(vec!["\\:a:\\:b:"], vec![(vec![s("a"), s("b")], bold())]);
        check_spans(vec!["\\::a"], vec![(vec![s("a")], plain())]);
        check_spans(vec!["\\:см. \\*3:"], vec![(vec![s("см. "), Note(3)], bold())]);
    }

    #[test]
    fn check_line_spans_color() {
        let red_style = PreStyle { color: red(), ..plain() };
        let blue_style = PreStyle { color: blue(), ..plain() };
        check_spans(vec!["\\%КРАС(a \\%СИН(b) c)"], vec![(vec![s("a "), s("b"), s(" c")], red_style)]);
        check_spans(vec!["\\:x \\%КРАС(a \\%СИН(b) c: d)"], vec![
            (vec![s("x "), s("a ")], bold()),
            (vec![s("b")], PreStyle { color: blue(), ..bold() }),
            (vec![s(" c")], bold()),
            (vec![s(" d)")], plain()),
        ]);
        check_spans(vec!["\\:\\%КРАС(Жирный) \\%СИН(сине) \\%КРАС(красный):"], vec![
            (vec![s("Жирный")], PreStyle { color: red(), ..bold() }),
            (vec![s(" ")], bold()),
            (vec![s("сине")], PreStyle { color: blue(), ..bold() }),
            (vec![s(" ")], bold()),
            (vec![s("красный")], PreStyle { color: red(), ..bold() }),
        ]);
        check_spans(vec!["\\%КРАС(a)\\%СИН(b)"], vec![
            (vec![s("a")], PreStyle { color: red(), ..plain() }),
            (vec![s("b")], PreStyle { color: blue(), ..plain() }),
        ]);
        check_spans(vec!["\\%КРАС(a)\\%КРАС(b)"], vec![(vec![s("a"), s("b")], PreStyle { color: red(), ..plain() })]);
        check_spans(vec!["\\%КРАС(\\%СИН(x))"], vec![(vec![s("x")], PreStyle { color: red(), ..plain() })]);
        check_spans(vec!["\\%КРАС()\\%СИН(x)"], vec![(vec![s("x")], blue_style)]);
        check_spans(vec!["\\%КРАС(a", "b"], vec![
            (vec![s("a")], PreStyle { color: red(), ..plain() }),
            (vec![Space, s("b")], plain()),
        ]);
        check_spans(vec!["\\%XYZ(a)"], vec![(vec![s("a")], plain())]);
    }

    #[test]
    fn check_line_spans_link() {
        let a = || PreStyle { link: Some("a.ru"), ..plain() };
        let b = || PreStyle { link: Some("b.ru"), ..plain() };
        check_spans(vec!["\\,a.ru(x \\,b.ru(y) z)"], vec![(vec![s("x "), s("y"), s(" z")], a())]);
        check_spans(vec!["\\,a.ru(x) \\,a.ru(y)"], vec![
            (vec![s("x")], a()),
            (vec![s(" ")], plain()),
            (vec![s("y")], a()),
        ]);
        check_spans(vec!["\\,a.ru(x)\\,a.ru(y)"], vec![(vec![s("x"), s("y")], a())]);
        check_spans(vec!["\\,a.ru(x)\\,b.ru(y)"], vec![(vec![s("x")], a()), (vec![s("y")], b())]);
        check_spans(vec!["\\,a.ru(\\%КРАС(x))"], vec![(vec![s("x")], PreStyle { color: red(), ..a() })]);
        check_spans(vec!["\\,a.ru(x", "y"], vec![(vec![s("x")], a()), (vec![Space, s("y")], plain())]);
        check_spans(vec!["\\,a.ru(x", "\\,b.ru(y"], vec![
            (vec![s("x")], a()),
            (vec![Space], plain()),
            (vec![s("y")], b()),
        ]);
    }

    #[test]
    fn check_line_spans_link_merge_multiline() {
        check_spans(vec!["\\,справка(очень длинный текст)", "\\,справка(ссылки на справку)"], vec![
            (vec![s("очень длинный текст"), Space, s("ссылки на справку")], PreStyle { link: Some("справка"), ..plain() }),
        ]);
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
        check("\\*p(A)", "p(a)");
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

    #[test]
    fn check_line_trailing_mark_dropped() {
        check_pre(vec!["аб\\"], vec![s("аб"), Space], vec![]);
        check_pre(vec!["аб_\\"], vec![s("аб"), s("\\"), Space], vec![]);
    }

    #[test]
    fn check_line_note_ref_overflow_is_text() {
        check_pre(vec!["a\\*70000 b"], vec![s("a"), s("*70000 b"), Space], vec![]);
        check_pre(vec!["a\\*65535"], vec![s("a"), Note(65535), Space], vec![]);
    }

    #[test]
    fn check_line_parse_note_line() {
        assert_eq!(parse_note_line("\\**1 текст"), Some((1, "текст")));
        assert_eq!(parse_note_line("\\**01\t \tтекст сноски"), Some((1, "текст сноски")));
        assert_eq!(parse_note_line("\\**0 a"), Some((0, "a")));
        assert_eq!(parse_note_line("\\**65535 a"), Some((65535, "a")));
        for line in ["\\**", "\\**1", "\\**1текст", "\\**70000 a", "\\**a a", "\\**-1 a", "\\**+1 a", "\\** 1 a", "\\ **1 a", "\\*1 a", "**1 a"] {
            assert_eq!(parse_note_line(line), None, "вход: {line:?}");
        }
    }

    #[test]
    fn check_line_note_ref_leading_zero() {
        check_pre(vec!["a\\*01"], vec![s("a"), Note(1), Space], vec![]);
    }

    #[test]
    fn check_line_color_rgb_channel_overflow() {
        assert_eq!(Color::from_unknown_string("999000000"), None);
        assert_eq!(Color::from_unknown_string("КРАС "), None);
        assert_eq!(Color::from_unknown_string("ЖЕЛ"), Some(Color::YELLOW));
    }

    #[test]
    fn check_line_link_target_trimmed() {
        check_pre(vec!["\\, a.ru (x)"], vec![s("x"), Space], vec![r(Mark::Link("a.ru"), 0, 1)]);
        check_pre(vec!["\\,\" a.ru \"(x)"], vec![s("x"), Space], vec![r(Mark::Link(" a.ru "), 0, 1)]);
    }

    #[test]
    fn check_line_clear_name_link_text_only() {
        check("\\,сайт.ru(Схема) узла", "схема узла");
        check("\\,\"a(b)\"(Схема) узла", "схема узла");
        check("\\,a.ru(x \\,b.ru(y) z)", "x y z");
    }

    #[test]
    fn check_line_clear_name_refs_removed() {
        check("Схема\\*1 узла", "схема узла");
        check("Схема \\*р(Другая (1)) узла", "схема узла");
        check("Схема\\*70000", "схема*70000");
    }

    #[test]
    fn check_line_clear_name_collapse_blanks() {
        check("  Схема  \t узла ", "схема узла");
        check("Схема \\:\\: узла", "схема узла");
    }

    #[test]
    fn check_line_clear_name_latin_case() {
        check("Scheme ABC", "scheme abc");
    }

    #[test]
    fn check_line_clear_name_image_comma() {
        assert_eq!(clear_name_attribute_in("Схема_, вариант", InlineContext::ImageName), "схема, вариант");
        assert_eq!(clear_name_attribute_in("Схема_, вариант", InlineContext::Name), "схема_, вариант");
    }
}
