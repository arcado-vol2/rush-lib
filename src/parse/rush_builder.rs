use std::collections::HashSet;
use std::borrow::Cow;

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
        Some(Self { r, g, b })
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

enum SpanAtribute {
    Bold,
    Italic,
    Strike,
    Underline,
    Monon,
    Colored(Color)
}

struct Span<'a>{
    attributes: HashSet<SpanAtribute>,
    text: Cow<'a, str>
}

enum Object<'a> {
    Named(Named<'a>),
    Unnamed(UnnamedKind<'a>)
}

struct Named<'a> {
    name: Option<&'a str>,
    index: Option<usize>,
    kind: NamedKind<'a>
}

enum NamedKind<'a> {
    Table(Table),
    Code(Code<'a>),
    Formula(Formula<'a> ),
    Image(Image<'a>)
}

enum UnnamedKind<'a> {
    Paragraph(Vec<Span<'a>>),
    List(List),
    Header(Header<'a>),
    Note(Note<'a>),
    Comment(Vec<Span<'a>>),
}

struct Table {
    //TODO
}
struct Code<'a> {
    lines: Vec<&'a str>
}

struct Formula<'a> {
    lines: Vec<&'a str>
}

struct Image<'a> {
    link: Cow<'a, str>,
    name: &'a str,
    size: u16
}

struct List {
    //TODO idk, something hard here
}

struct Header<'a> {
    level: u8,
    text: Vec<Span<'a>>
}

struct Note<'a> {
    index: u16,
    text: Vec<Span<'a>>
}