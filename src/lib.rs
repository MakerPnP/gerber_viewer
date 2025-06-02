mod color;
mod expressions;
mod geometry;
mod layer;
pub mod position;

#[cfg(feature = "egui")]
mod renderer;

#[cfg(feature = "egui")]
mod drawing;

#[cfg(feature = "egui")]
mod ui;

pub use color::*;
#[cfg(feature = "egui")]
pub use drawing::*;
pub use geometry::*;
/// re-export 'gerber_parser' crate
#[cfg(feature = "parser")]
pub use gerber_parser;
/// re-export 'gerber_types' crate
#[cfg(feature = "types")]
pub use gerber_types;
pub use layer::*;
#[cfg(feature = "egui")]
pub use renderer::*;
#[cfg(feature = "egui")]
pub use ui::*;

#[cfg(feature = "testing")]
pub mod testing;

use crate::position::Position;

pub enum Winding {
    /// Aka 'Positive' in Geometry
    Clockwise,
    /// Aka 'Negative' in Geometry
    CounterClockwise,
}

pub fn calculate_winding(vertices: &[Position]) -> Winding {
    let mut sum = 0.0;
    for i in 0..vertices.len() {
        let j = (i + 1) % vertices.len();
        sum += vertices[i].x * vertices[j].y - vertices[j].x * vertices[i].y;
    }
    if sum > 0.0 {
        Winding::Clockwise
    } else {
        Winding::CounterClockwise
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exposure {
    CutOut,
    Add,
}

impl From<bool> for Exposure {
    fn from(value: bool) -> Self {
        match value {
            true => Exposure::Add,
            false => Exposure::CutOut,
        }
    }
}
