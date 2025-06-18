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

impl From<[i8; 2]> for Mirroring {
    fn from(value: [i8; 2]) -> Self {
        Self {
            x: value[0] != 0,
            y: value[1] != 0,
        }
    }
} 

impl From<(bool, bool)> for Mirroring {
    fn from(value: (bool, bool)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

impl From<(i8, i8)> for Mirroring {
    fn from(value: (i8, i8)) -> Self {
        Self {
            x: value.0 != 0,
            y: value.1 != 0,
        }
    }
}

impl Mirroring {
    pub fn as_f64(&self) -> [f64; 2] {
        [
            if self.x { -1.0 } else { 1.0 },
            if self.y { -1.0 } else { 1.0 },
        ]
    }
    
    pub fn as_f32(&self) -> [f32; 2] {
        [
            if self.x { -1.0 } else { 1.0 },
            if self.y { -1.0 } else { 1.0 },
        ]
    }
    
    pub fn as_i8(&self) -> [i8; 2] {
        [
            if self.x { -1 } else { 1 },
            if self.y { -1 } else { 1 },
        ]
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
