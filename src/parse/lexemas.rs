#![allow(dead_code)]
pub(crate) const MARK: char = '\\';
pub(crate) const TAB: char = '\t';
pub(crate) const CR: char = '\r';
pub(crate) const SPACE: char = ' ';
pub(crate) const HEADER: char = '+';
pub(crate) const BULLET_LIST: char = '.';
pub(crate) const NUMERIC_LIST: char = '1';
pub(crate) const IMG_LBRACE: char = '(';
pub(crate) const IMG_RBRACE: char = ')';
pub(crate) const IMG_SEP: char = ',';
pub(crate) const FORMULA: char = '$';
pub(crate) const LINK: char = ',';
pub(crate) const LINK_LBRACE: char = '(';
pub(crate) const LINK_RBRACE: char = ')';
pub(crate) const NOTE: char = '*';
pub(crate) const ANON: char = '0';
pub(crate) const COMMENT: char = '\'';
pub(crate) const COLOR: char = '%';
pub(crate) const BOLD: char = ':';
pub(crate) const ITALIC: char = ';';
pub(crate) const STRIKE: char = '-';
pub(crate) const UNDER: char = '=';
pub(crate) const CODE: char = '/';
pub(crate) const MONO: char = '!';
pub(crate) const TABLE: char = '№';
pub(crate) const TABLE_CENTER: char = 'Ц';
pub(crate) const TABLE_LEFT: char = 'Л'; 
pub(crate) const TABLE_RIGHT: char = 'П';
pub(crate) const TABLE_SEP: char = '/';
pub(crate) const TABLE_MERGE_LEFT: char = '+';
pub(crate) const TABLE_MERGE_UP: char = '=';
pub(crate) const NEW_LINE: char = '\n';
pub(crate) const SHIELD: char = '_'; 

pub(crate) fn is_blank(c: char) -> bool{
    matches!(c, SPACE | TAB | CR)
}

pub(crate) fn is_special(c: char) -> bool {
    matches!(
        c,
        SPACE | TAB |
        HEADER | 
        BULLET_LIST | NUMERIC_LIST |
        IMG_LBRACE | IMG_RBRACE | IMG_SEP |
        FORMULA | 
        LINK | LINK_LBRACE | LINK_RBRACE |
        NOTE |
        ANON |
        COMMENT |
        COLOR | BOLD | ITALIC | STRIKE | UNDER |
        CODE |
        TABLE | TABLE_CENTER | TABLE_LEFT | TABLE_RIGHT | TABLE_MERGE_LEFT | TABLE_MERGE_UP |
        SHIELD
    )
}

pub(crate) const NEW_LINE_BREAK: usize = 2;

#[derive(PartialEq, Debug)]
pub(crate) enum BlockKind {
    Paragraph,
    List,
    Table,
    Header(u8),
    Mark,
    Comment,
    Formula,
    Code
}


#[derive(PartialEq, Debug)]
pub(crate) enum LexemaKind {
    Empty,
    NewLine,
    Name,
    NumericList(u32),
    BulletList(u32),
    ImgStart,
    ImgPath,
    ImgName,
    ImgSize,
    ImgEnd,
    NoteRef,
    NoteDef,
    CommentStart,
    CommentEnd,
    FormulaStart,
    FormulaEnd,
    CodeStart,
    CodeEnd,
    TableStart,
    TableHeaderEnd,
    TableSep,
    TableMergeLeft,
    TableMergeUp,
    TableMergeLeftUp,
    TableEnd,
    BoldStart,
    BoldEnd,
    ItalicStart,
    ItalicEnd,
    StrikeStart,
    StrikeEnd,
    UnderlineStart,
    UnderlineEnd,
    MonoStart,
    MonoEnd,
    ColorStart,
    ColorValue,
    ColorEnd,
    LinkStart,
    LinkTarget,
    LinkText,
    LinkEnd,
}

#[derive(Debug)]
pub(crate) struct Lexema<'a> {
    pub(crate) kind: LexemaKind,
    pub(crate) data: &'a str
}

#[derive(Debug)]
pub(crate) struct Block<'a> {
    pub(crate) kind: BlockKind,
    pub(crate) lines: Vec<&'a str>
}

