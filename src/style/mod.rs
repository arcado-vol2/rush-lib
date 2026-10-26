use crate::config::DocStyle;
use crate::render::Feature;

mod gost;

pub(crate) const MM: f32 = 72.0 / 25.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    Justify,
    Left,
    Center,
    Right,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Font {
    pub(crate) mono: bool,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
}

impl Font {
    pub(crate) const MONO: Font = Font { mono: true, bold: false, italic: false };
    pub(crate) const BOLD: Font = Font { mono: false, bold: true, italic: false };
    pub(crate) const ITALIC: Font = Font { mono: false, bold: false, italic: true };

    pub(crate) fn with(self, other: Font) -> Font {
        Font { mono: self.mono || other.mono, bold: self.bold || other.bold, italic: self.italic || other.italic }
    }
}

/// Font face file: (file name, data)
pub(crate) type FontFile = (&'static str, &'static [u8]);

#[derive(Debug, Clone, Copy)]
pub(crate) struct FontFamily {
    pub(crate) name: &'static str,
    /// fallback families if the files are unavailable
    pub(crate) fallback: &'static str,
    /// regular, italic, bold, bold italic
    pub(crate) files: [FontFile; 4],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Fonts {
    pub(crate) body: FontFamily,
    pub(crate) mono: FontFamily,
}

impl Fonts {
    pub(crate) fn family(&self, font: Font) -> &FontFamily {
        if font.mono { &self.mono } else { &self.body }
    }

    pub(crate) fn file(&self, font: Font) -> FontFile {
        self.family(font).files[font.bold as usize * 2 + font.italic as usize]
    }

    pub(crate) fn all(&self) -> impl Iterator<Item = (&FontFamily, Font, FontFile)> + '_ {
        [false, true].into_iter().flat_map(move |mono| {
            [(false, false), (false, true), (true, false), (true, true)].into_iter().map(move |(bold, italic)| {
                let font = Font { mono, bold, italic };
                (self.family(font), font, self.file(font))
            })
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Role {
    pub(crate) size: f32,
    pub(crate) lh: f32,
    pub(crate) font: Font,
    pub(crate) upper: bool,
    pub(crate) align: Align,
    /// first line indent
    pub(crate) first: f32,
    /// indent of the other lines
    pub(crate) rest: f32,
    pub(crate) space_before: f32,
    pub(crate) space_after: f32,
    pub(crate) keep_with_next: bool,
    pub(crate) break_before: bool,
    /// how many block lines to keep together at the start and end of a page
    pub(crate) min_first: usize,
    pub(crate) min_last: usize,
}

impl Role {
    pub(crate) const fn text(size: f32, lh: f32) -> Self {
        Self {
            size,
            lh,
            font: Font { mono: false, bold: false, italic: false },
            upper: false,
            align: Align::Left,
            first: 0.0,
            rest: 0.0,
            space_before: 0.0,
            space_after: 0.0,
            keep_with_next: false,
            break_before: false,
            min_first: usize::MAX,
            min_last: 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PageGeometry {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) top: f32,
    pub(crate) right: f32,
    pub(crate) bottom: f32,
    pub(crate) left: f32,
    /// bottom reserve for measurement error
    pub(crate) slack: f32,
    pub(crate) numbers: bool,
}

impl PageGeometry {
    pub(crate) const A4: PageGeometry = PageGeometry {
        width: 210.0 * MM,
        height: 297.0 * MM,
        top: 20.0 * MM,
        right: 15.0 * MM,
        bottom: 20.0 * MM,
        left: 30.0 * MM,
        slack: 4.0,
        numbers: true,
    };

    pub(crate) fn text_w(&self) -> f32 {
        self.width - self.left - self.right
    }

    pub(crate) fn text_h(&self) -> f32 {
        self.height - self.top - self.bottom
    }
}

/// Templates with `{n}` - the element number
#[derive(Debug, Clone, Copy)]
pub(crate) struct Caption {
    pub(crate) named: &'static str,
    pub(crate) anonymous: &'static str,
    pub(crate) role: Role,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Captions {
    pub(crate) table: Caption,
    pub(crate) image: Caption,
    pub(crate) code: Caption,
    pub(crate) table_cont: &'static str,
    pub(crate) formula: &'static str,
}

pub(crate) fn fill(template: &str, n: u16) -> String {
    template.replace("{n}", &n.to_string())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Numbering {
    pub(crate) from: u8,
    pub(crate) to: u8,
    /// how many leading index parts to omit
    pub(crate) skip: usize,
    pub(crate) sep: &'static str,
    pub(crate) trailing: &'static str,
}

impl Numbering {
    pub(crate) fn number(&self, level: u8, index: &[u16]) -> Option<String> {
        if !(self.from..=self.to).contains(&level) {
            return None;
        }
        let digits: Vec<String> = index.iter().skip(self.skip).map(u16::to_string).collect();
        Some(format!("{}{}", digits.join(self.sep), self.trailing))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Markers {
    pub(crate) ordered: &'static str,
    pub(crate) unordered: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Toc {
    pub(crate) title: &'static str,
    /// deeper headers are left out of the table of contents
    pub(crate) max_level: u8,
    pub(crate) title_role: Role,
    pub(crate) entry: Role,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Frame {
    pub(crate) border: f32,
    pub(crate) pad_x: f32,
    pub(crate) pad_y: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Notes {
    pub(crate) role: Role,
    /// height of the separator above notes
    pub(crate) sep: f32,
    pub(crate) rule_w: f32,
    pub(crate) rule_top: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Images {
    pub(crate) gap: f32,
    /// pt in one image pixel
    pub(crate) px: f32,
    pub(crate) broken_h: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Style {
    pub(crate) name: DocStyle,
    pub(crate) fonts: Fonts,
    pub(crate) page: Option<PageGeometry>,
    pub(crate) border: f32,
    pub(crate) sup_scale: f32,

    pub(crate) body: Role,
    pub(crate) comment: Role,
    pub(crate) comment_prefix: Option<&'static str>,
    pub(crate) doc_title: Role,
    pub(crate) headers: [Role; 5],
    pub(crate) numbering: Numbering,
    pub(crate) list: Role,
    pub(crate) markers: Markers,
    pub(crate) captions: Captions,

    pub(crate) figure: Role,
    pub(crate) images: Images,
    pub(crate) formula: Role,
    pub(crate) code: Role,
    pub(crate) code_frame: Frame,
    pub(crate) table: Role,
    pub(crate) cell: Role,
    pub(crate) head_cell: Role,
    pub(crate) cell_frame: Frame,
    pub(crate) notes: Notes,
    pub(crate) toc: Option<Toc>,
}

impl Style {
    pub(crate) fn of(name: DocStyle) -> Style {
        match name {
            DocStyle::Gost7_32 => gost::style(),
            _ => unimplemented!("not yet"),
        }
    }

    pub(crate) fn header(&self, level: u8) -> &Role {
        &self.headers[level.clamp(1, 5) as usize - 1]
    }

    pub(crate) fn supports(&self, _feature: Feature) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbering_skips_and_joins() {
        let n = Numbering { from: 2, to: 4, skip: 1, sep: ".", trailing: "" };
        assert_eq!(n.number(1, &[1]), None);
        assert_eq!(n.number(3, &[1, 2, 3]), Some("2.3".into()));
        assert_eq!(n.number(5, &[1, 2, 3, 4, 5]), None);
    }

    #[test]
    fn font_file_by_face() {
        let f = Style::of(DocStyle::Gost7_32).fonts;
        assert_eq!(f.file(Font { bold: true, ..Font::MONO }).0, "LiberationMono-Bold.ttf");
        assert_eq!(f.file(Font::ITALIC).0, "LiberationSerif-Italic.ttf");
        assert_eq!(f.all().count(), 8);
    }
}
