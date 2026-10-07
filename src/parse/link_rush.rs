use std::collections::{HashSet, HashMap};
use std::borrow::Cow;
use crate::parse::lexemas::{Block, RefKind};

struct InnerLinks<'a> {
    notes: HashSet<u16>,
    
    tables: HashMap<Cow<'a, str>, u16>,
    images: HashMap<Cow<'a, str>, u16>,
    code: HashMap<Cow<'a, str>, u16>,
    formulas: HashMap<Cow<'a, str>, u16>,
}



impl InnerLinks<'_> {
    fn resolve(&self, kind: RefKind, name: &str) -> Option<usize> {
        None
    }

    fn has_note(&self, n: u16) -> bool {
        false
    }
}

fn create_inner_links<'a>(inp: Vec<Block<'a>>){

}
