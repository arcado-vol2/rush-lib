use crate::content::Content;
use crate::resources::Resources;
use crate::style::Style;

pub(crate) mod html;
pub(crate) mod markdown;

pub(crate) struct PageCtx<'a> {
    pub(crate) res: &'a Resources,
    pub(crate) content: &'a Content,
    pub(crate) style: &'a Style,
}
