use std::sync::OnceLock;
use ttf_parser::{Face, GlyphId};

pub(crate) const FONT_FILES: [(&str, &[u8]); 8] = [
    ("LiberationSerif-Regular.ttf", include_bytes!("../../../templates/gost_7_32/fonts/LiberationSerif-Regular.ttf")),
    ("LiberationSerif-Italic.ttf", include_bytes!("../../../templates/gost_7_32/fonts/LiberationSerif-Italic.ttf")),
    ("LiberationSerif-Bold.ttf", include_bytes!("../../../templates/gost_7_32/fonts/LiberationSerif-Bold.ttf")),
    ("LiberationSerif-BoldItalic.ttf", include_bytes!("../../../templates/gost_7_32/fonts/LiberationSerif-BoldItalic.ttf")),
    ("LiberationMono-Regular.ttf", include_bytes!("../../../templates/gost_7_32/fonts/LiberationMono-Regular.ttf")),
    ("LiberationMono-Italic.ttf", include_bytes!("../../../templates/gost_7_32/fonts/LiberationMono-Italic.ttf")),
    ("LiberationMono-Bold.ttf", include_bytes!("../../../templates/gost_7_32/fonts/LiberationMono-Bold.ttf")),
    ("LiberationMono-BoldItalic.ttf", include_bytes!("../../../templates/gost_7_32/fonts/LiberationMono-BoldItalic.ttf")),
];

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Font {
    pub(crate) mono: bool,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
}

impl Font {
    pub(crate) const MONO: Font = Font { mono: true, bold: false, italic: false };

    fn index(self) -> usize {
        self.mono as usize * 4 + self.bold as usize * 2 + self.italic as usize
    }
}

fn faces() -> &'static [Face<'static>] {
    static FACES: OnceLock<Vec<Face<'static>>> = OnceLock::new();
    FACES.get_or_init(|| FONT_FILES.iter()
        .map(|(name, data)| Face::parse(data, 0).unwrap_or_else(|e| panic!("встроенный шрифт {name}: {e}")))
        .collect())
}

pub(crate) fn width(text: &str, font: Font, size: f32) -> f32 {
    let face = &faces()[font.index()];
    let upem = face.units_per_em() as f32;
    let units: u32 = text.chars()
        .map(|c| {
            let id = face.glyph_index(c).unwrap_or(GlyphId(0));
            face.glyph_hor_advance(id).unwrap_or(0) as u32
        })
        .sum();
    units as f32 / upem * size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_is_fixed_and_serif_is_not() {
        let m = width("iiii", Font::MONO, 12.0);
        assert!((m - width("WWWW", Font::MONO, 12.0)).abs() < 1e-3);
        assert!((m - 4.0 * 0.6 * 12.0).abs() < 0.1, "{m}");
        let serif = Font::default();
        assert!(width("W", serif, 14.0) > width("i", serif, 14.0));
        assert!(width("Ж", Font { bold: true, ..serif }, 14.0) >= width("Ж", serif, 14.0));
    }
}
