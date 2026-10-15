use crate::parse::lexemas::{FORMULA_REF, TABLE_REF, CODE_REF, IMAGE_REF};

#[derive(Debug, PartialEq)]
pub(crate) enum RefKind {
    Formula,
    Table,
    Code,
    Image
}


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
pub(crate) struct ImageKind<'a> {
    pub(crate) named: Named<'a>,
    pub(crate) scale: u16
}
#[derive(PartialEq, Debug)]
pub(crate) struct Named<'a> {
    pub(crate) name: Option<&'a str>,
    pub(crate) index: u16
}
impl<'a> Named<'a> {
    pub(crate) fn none() -> Self {
        Self { name: None, index: 0 }
    }

    pub(crate) fn new(name: Option<&'a str>, index: u16) -> Self {
        Self {name, index}
    }
}
