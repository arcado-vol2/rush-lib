#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub(crate) struct Color {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
}
impl Color {
    pub(crate) const BLACK: Self = Self {r: 0, g: 0, b: 0};
    pub(crate) const WHITE: Self = Self {r: 255, g: 255, b: 255};
    pub(crate) const RED: Self        = Self { r: 220, g: 50,  b: 47  };
    pub(crate) const ORANGE: Self     = Self { r: 190, g: 90,  b: 0   };
    pub(crate) const YELLOW: Self     = Self { r: 158, g: 112, b: 0   }; 
    pub(crate) const LEMON: Self      = Self { r: 132, g: 124, b: 0   }; 
    pub(crate) const CHARTREUSE: Self = Self { r: 100, g: 128, b: 0   };
    pub(crate) const GREEN: Self      = Self { r: 35,  g: 135, b: 65  };
    pub(crate) const CYAN: Self       = Self { r: 0,   g: 128, b: 168 };
    pub(crate) const LIGHT_BLUE: Self = Self { r: 40,  g: 115, b: 230 };
    pub(crate) const LAZULI: Self     = Self { r: 56,  g: 116, b: 180 };
    pub(crate) const DARK_BLUE: Self  = Self { r: 88,  g: 96,  b: 232 };
    pub(crate) const MAGENTA: Self    = Self { r: 165, g: 64,  b: 225 }; 
    pub(crate) const PURPLE: Self     = Self { r: 200, g: 40,  b: 170 };
    pub(crate) const PINK: Self       = Self { r: 204, g: 72,  b: 110 };
    pub(crate) const BROWN: Self      = Self { r: 160, g: 105, b: 70  };

    pub(crate) fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}
impl Color {
    pub(crate) fn to_hex_tag(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}
