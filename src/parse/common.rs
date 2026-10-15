use crate::parse::lexemas::{FORMULA_REF, TABLE_REF, CODE_REF, IMAGE_REF};



pub(crate) fn ref_kind(c: char) -> Option<RefKind> {
    match c {
        FORMULA_REF => Some(RefKind::Formula),
        TABLE_REF => Some(RefKind::Table),
        CODE_REF => Some(RefKind::Code),
        IMAGE_REF => Some(RefKind::Image),
        _ => None
    }
}

#[derive(PartialEq, Debug)]
pub(crate) struct ImageKind<T> {
    pub(crate) named: Named<T>,
    pub(crate) scale: u16
}
impl<T> ImageKind<T> {
    pub(crate) fn map<U>(self, f: impl FnOnce(T) -> U) -> ImageKind<U> {
        ImageKind { named: self.named.map(f), scale: self.scale }
    }
}

/// T -name representation: a raw string with block-level markup, with spans parsed into models
#[derive(PartialEq, Debug)]
pub(crate) enum Named<T> {
    WithName { name: T, index: u16 },
    Anonymous
}
impl<T> Named<T> {
    pub(crate) fn map<U>(self, f: impl FnOnce(T) -> U) -> Named<U> {
        match self {
            Named::WithName { name, index } => Named::WithName { name: f(name), index },
            Named::Anonymous => Named::Anonymous
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub(crate) struct Color {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
}
impl Color {
    pub(crate) const BLACK: Self = Self {r: 0, g: 0, b: 0};
    pub(crate) const WHITE: Self = Self {r: 255, g: 255, b: 255};
    pub(crate) const RED: Self = Self {r: 255, g: 0, b: 0};
    pub(crate) const ORANGE: Self = Self {r: 255, g: 128, b: 0};
    pub(crate) const YELLOW: Self = Self {r: 255, g: 200, b: 0};
    pub(crate) const GREEN: Self = Self {r: 0, g: 255, b: 0};
    pub(crate) const LIGHT_BLUE: Self = Self {r: 0, g: 128, b: 255};
    pub(crate) const DARK_BLUE: Self = Self {r: 0, g: 0, b: 255};
    pub(crate) const MAGENTA: Self = Self {r: 200, g: 0, b: 255};
    pub(crate) const PINK: Self = Self {r: 255, g: 128, b: 128};
    pub(crate) const BROWN: Self = Self {r: 64, g: 32, b: 0};
    pub(crate) const LEMON: Self = Self {r: 255, g: 247, b: 0};
    pub(crate) const CHARTREUSE: Self = Self {r: 127, g: 255, b: 0};
    pub(crate) const CYAN: Self = Self {r: 0, g: 200, b: 255};
    pub(crate) const LAZULI: Self = Self {r: 38, g: 97, b: 156};
    pub(crate) const FUCHSIA: Self = Self {r: 255, g: 0, b: 255};

    pub(crate) fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum RefKind {
    Formula,
    Table,
    Code,
    Image
}
