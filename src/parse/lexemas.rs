pub(crate) const MARK: char = '\\';
pub(crate) const TAB: char = '\t';
pub(crate) const CR: char = '\r';
pub(crate) const SPACE: char = ' ';
pub(crate) const HEADER: char = '+';
pub(crate) const BULLET_LIST: char = '.';
pub(crate) const NUMERIC_LIST: char = '1';
pub(crate) const IMG_SEP: char = ',';
pub(crate) const FORMULA: char = '$';
pub(crate) const LINK: char = ',';
pub(crate) const LBRACE: char = '(';
pub(crate) const RBRACE: char = ')';
pub(crate) const NOTE: char = '*';
pub(crate) const ANON: char = '0';
pub(crate) const COMMENT: char = '\"';
pub(crate) const COLOR: char = '%';
pub(crate) const BOLD: char = ':';
pub(crate) const ITALIC: char = ';';
pub(crate) const STRIKE: char = '-';
pub(crate) const UNDER: char = '=';
pub(crate) const CODE: char = '/';
pub(crate) const MONO: char = '!';
pub(crate) const TABLE: char = '№';
pub(crate) const TABLE_FORMAT: char = '-';
pub(crate) const TABLE_CENTER: char = 'Ц';
pub(crate) const TABLE_LEFT: char = 'Л'; 
pub(crate) const TABLE_RIGHT: char = 'П';
pub(crate) const TABLE_SEP: char = '/';
pub(crate) const TABLE_MERGE_LEFT: char = '+';
pub(crate) const TABLE_MERGE_UP: char = '=';
pub(crate) const NEW_LINE: char = '\n';
pub(crate) const SHIELD: char = '_'; 
pub(crate) const QUOTES: char = '"'; 
pub(crate) const SMALL_YO: char = 'ё';
pub(crate) const BIG_YO: char = 'Ё';
pub(crate) const SMALL_E: char = 'е';
pub(crate) const BIG_E: char = 'Е';
pub(crate) const EXT_DIV: char = '.';


pub(crate) const BYTE_SHIELD: u8 = b'_';

pub(crate) fn is_blank(c: char) -> bool{
    matches!(c, SPACE | TAB | CR)
}


#[derive(Clone, Copy, Debug)]
pub(crate) struct Tag<const N: usize, const V: usize = 1> {
    pub(crate) variants: [[char; N]; V],
}

impl<const N: usize> Tag<N> {
    pub(crate) const fn repeat(ch: char) -> Self {
        Self { variants: [[ch; N]] }
    }

    pub(crate) const fn seq(seq: [char; N]) -> Self {
        Self { variants: [seq] }
    }
}

impl<const N: usize, const V: usize> Tag<N, V> {
    pub(crate) const fn any(variants: [[char; N]; V]) -> Self {
        Self { variants }
    }

    pub(crate) fn strip<'a>(&self, s: &'a str) -> Option<&'a str> {
        self.variants.iter().find_map(|seq| {
            let mut rest = s;
            for &ch in seq {
                rest = rest.strip_prefix(ch)?;
            }
            Some(rest)
        })
    }

    pub(crate) fn is(&self, s: &str) -> bool {
        self.strip(s) == Some("")
    }
}

pub(crate)const CODE_TAG: Tag<2> = Tag::repeat(CODE);
pub(crate)const COMMENT_TAG: Tag<1> = Tag::repeat(COMMENT);
pub(crate)const FORMUAL_TAG: Tag<1> = Tag::repeat(FORMULA);
pub(crate)const TABLE_TAG: Tag<1> = Tag::repeat(TABLE);
pub(crate)const NOTE_TAG: Tag<2> = Tag::repeat(NOTE);
pub(crate)const BULLET_LIST_TAG: Tag<1> = Tag::repeat(BULLET_LIST);
pub(crate)const NUMERIC_LIST_TAG: Tag<1> = Tag::repeat(NUMERIC_LIST);
pub(crate)const ANON_NAME_TAG: Tag<1> = Tag::repeat(ANON);
/// `\№-`
pub(crate)const TABLE_FUL_SEP: Tag<3> = Tag::seq([MARK, TABLE, TABLE_FORMAT]);
/// `+=` or `=+`
pub(crate)const TABLE_MERGE_UP_LEFT_TAG: Tag<2, 2> = Tag::any([
    [TABLE_MERGE_LEFT, TABLE_MERGE_UP],
    [TABLE_MERGE_UP, TABLE_MERGE_LEFT],
]);
pub(crate)const TABLE_MERGE_LEFT_TAG: Tag<1> = Tag::repeat(TABLE_MERGE_LEFT);
pub(crate)const TABLE_MERGE_UP_TAG: Tag<1> = Tag::repeat(TABLE_MERGE_UP);



pub(crate) const NEW_LINE_BREAK: usize = 2;
pub(crate) const TAB_WIDTH: u16 = 4;

pub(crate) const FORMULA_REF: char = 'ф';
pub(crate) const TABLE_REF: char = 'т';
pub(crate) const CODE_REF: char = 'л';
pub(crate) const IMAGE_REF: char = 'р';





