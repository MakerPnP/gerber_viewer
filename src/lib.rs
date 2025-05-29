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

use crate::position::Position;

/// Prelude module that re-exports commonly used types and traits
pub mod prelude {
    pub use crate::{
        // Core types
        position::{Position, Vector},
        BoundingBox, Transform2D, Mirroring,
        
        // Layer and primitives
        GerberLayer, GerberPrimitive,
        
        // Enums
        Exposure, Winding,
        
        // Functions
        calculate_winding,
    };
    
    #[cfg(feature = "egui")]
    pub use crate::{
        // Renderer
        GerberRenderer, ViewState,
        
        // UI
        UiState,
        
        // Drawing functions
        draw_crosshair, draw_arrow, draw_outline, draw_marker,
        
        // Color utilities
        generate_pastel_color, hsv_to_rgb,
    };
    
    // Re-export parser and types if features are enabled
    #[cfg(feature = "parser")]
    pub use crate::gerber_parser;
    
    #[cfg(feature = "types")]
    pub use crate::gerber_types;
}

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

#[derive(Debug, Clone)]
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
