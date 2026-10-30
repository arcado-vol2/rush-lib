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

#[derive(PartialEq, Debug)]
pub(crate) enum Named<T> {
    WithName { name: T, index: u16 },
    Anonymous {index: u16}
}
impl<T> Named<T> {
    pub(crate) fn map<U>(self, f: impl FnOnce(T) -> U) -> Named<U> {
        match self {
            Named::WithName { name, index } => Named::WithName { name: f(name), index },
            Named::Anonymous {index} => Named::Anonymous {index}
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum RefKind {
    Formula,
    Table,
    Code,
    Image
}
