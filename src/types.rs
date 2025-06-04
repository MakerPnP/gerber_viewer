use crate::Position;

pub(crate) enum Winding {
    /// Aka 'Positive' in Geometry
    Clockwise,
    /// Aka 'Negative' in Geometry
    CounterClockwise,
}

impl Winding {
    pub(crate) fn from_vertices(vertices: &[Position]) -> Self {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exposure {
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
