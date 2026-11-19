use crate::parse::lexemas::{CODE_REF, CODE_REF_ALT_1, CODE_REF_ALT_2, CODE_REF_ALT_3, FORMULA_REF, FORMULA_REF_ALT_1, IMAGE_REF, IMAGE_REF_ALT_1, IMAGE_REF_ALT_2, IMAGE_REF_ALT_3, TABLE_REF, TABLE_REF_ALT_1};



pub(crate) fn ref_kind(c: char) -> Option<RefKind> {
    match c {
        FORMULA_REF | FORMULA_REF_ALT_1 => Some(RefKind::Formula),
        TABLE_REF | TABLE_REF_ALT_1  => Some(RefKind::Table),
        CODE_REF | CODE_REF_ALT_1 | CODE_REF_ALT_2 | CODE_REF_ALT_3 => Some(RefKind::Code),
        IMAGE_REF | IMAGE_REF_ALT_1 | IMAGE_REF_ALT_2 | IMAGE_REF_ALT_3 => Some(RefKind::Image),
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
