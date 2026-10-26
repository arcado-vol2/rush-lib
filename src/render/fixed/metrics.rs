use crate::style::{Font, Fonts};
use ttf_parser::{Face, GlyphId};

pub(crate) struct Metrics {
    /// indexed by mono * 4 + bold * 2 + italic
    faces: Vec<Face<'static>>,
}

fn index(font: Font) -> usize {
    font.mono as usize * 4 + font.bold as usize * 2 + font.italic as usize
}

impl Metrics {
    pub(crate) fn new(fonts: &Fonts) -> Self {
        let faces = [fonts.body, fonts.mono].iter()
            .flat_map(|f| f.files.iter())
            .map(|(name, data)| Face::parse(data, 0).unwrap_or_else(|e| panic!("встроенный шрифт {name}: {e}")))
            .collect();
        Self { faces }
    }

    pub(crate) fn width(&self, text: &str, font: Font, size: f32) -> f32 {
        let face = &self.faces[index(font)];
        let upem = face.units_per_em() as f32;
        let units: u32 = text.chars()
            .map(|c| {
                let id = face.glyph_index(c).unwrap_or(GlyphId(0));
                face.glyph_hor_advance(id).unwrap_or(0) as u32
            })
            .sum();
        units as f32 / upem * size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DocStyle;
    use crate::style::Style;

    #[test]
    fn mono_is_fixed_and_serif_is_not() {
        let m = Metrics::new(&Style::of(DocStyle::Gost7_32).fonts);
        let mono = m.width("iiii", Font::MONO, 12.0);
        assert!((mono - m.width("WWWW", Font::MONO, 12.0)).abs() < 1e-3);
        assert!((mono - 4.0 * 0.6 * 12.0).abs() < 0.1, "{mono}");
        let serif = Font::default();
        assert!(m.width("W", serif, 14.0) > m.width("i", serif, 14.0));
        assert!(m.width("Ж", Font::BOLD, 14.0) >= m.width("Ж", serif, 14.0));
    }
}
