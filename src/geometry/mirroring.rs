use egui::Vec2b;

#[derive(Debug, Copy, Clone)]
pub struct Mirroring {
    pub x: bool,
    pub y: bool,
}

impl core::ops::BitXor for Mirroring {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x ^ rhs.x,
            y: self.y ^ rhs.y,
        }
    }
}

impl Default for Mirroring {
    fn default() -> Self {
        Self {
            x: false,
            y: false,
        }
    }
}

impl From<[bool; 2]> for Mirroring {
    fn from(value: [bool; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
        }
    }
}

#[cfg(feature = "egui")]
impl From<Vec2b> for Mirroring {
    fn from(value: Vec2b) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}
