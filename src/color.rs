#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub(crate) struct Color {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
}
impl Color {
    pub(crate) const BLACK: Self = Self {r: 0, g: 0, b: 0};
    pub(crate) const WHITE: Self = Self {r: 255, g: 255, b: 255};
    pub(crate) const RED: Self = Self {r: 255, g: 0, b: 0};
    pub(crate) const ORANGE: Self = Self {r: 255, g: 128, b: 0};
    pub(crate) const YELLOW: Self = Self {r: 255, g: 200, b: 0};
    pub(crate) const GREEN: Self = Self {r: 0, g: 255, b: 0};
    pub(crate) const LIGHT_BLUE: Self = Self {r: 0, g: 128, b: 255};
    pub(crate) const DARK_BLUE: Self = Self {r: 0, g: 0, b: 255};
    pub(crate) const MAGENTA: Self = Self {r: 200, g: 0, b: 255};
    pub(crate) const PINK: Self = Self {r: 255, g: 128, b: 128};
    pub(crate) const BROWN: Self = Self {r: 64, g: 32, b: 0};
    pub(crate) const LEMON: Self = Self {r: 255, g: 247, b: 0};
    pub(crate) const CHARTREUSE: Self = Self {r: 127, g: 255, b: 0};
    pub(crate) const CYAN: Self = Self {r: 0, g: 200, b: 255};
    pub(crate) const LAZULI: Self = Self {r: 38, g: 97, b: 156};
    pub(crate) const FUCHSIA: Self = Self {r: 255, g: 0, b: 255};

    pub(crate) fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}
impl Color {
    pub(crate) fn to_hex_tag(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}
