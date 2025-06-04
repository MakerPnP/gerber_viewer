#[cfg(feature = "egui")]
use egui::{Pos2, Vec2};

#[cfg(feature = "egui")]
pub trait ToPos2 {
    fn to_pos2(self) -> Pos2;
}

#[cfg(feature = "egui")]
impl ToPos2 for Position {
    fn to_pos2(self) -> Pos2 {
        Pos2::new(self.x as f32, self.y as f32)
    }
}

pub trait ToVector {
    fn to_vector(self) -> Vector;
}

impl ToVector for Position {
    fn to_vector(self) -> Vector {
        Vector::new(self.x, self.y)
    }
}

#[cfg(feature = "egui")]
pub trait FromVec2 {
    fn from(value: Vec2) -> Self;
}

#[cfg(feature = "egui")]
impl FromVec2 for Position {
    fn from(value: Vec2) -> Self {
        Self::new(value.x as f64, value.y as f64)
    }
}

pub trait FromTuple2 {
    fn from(value: (f64, f64)) -> Self;
}

impl FromTuple2 for Position {
    fn from(value: (f64, f64)) -> Self {
        Self::new(value.0, value.1)
    }
}

#[cfg(feature = "egui")]
pub trait AddVec2 {
    fn add(self, rhs: Vec2) -> Self;
}

#[cfg(feature = "egui")]
impl AddVec2 for Position {

    fn add(self, rhs: Vec2) -> Self {
        Self::new(self.x + rhs.x as f64, self.y + rhs.y as f64)
    }
}

pub trait ToPosition {
    fn to_position(self) -> Position;
}

impl ToPosition for Vector {
    fn to_position(self) -> Position {
        Position::new(self.x, self.y)
    }
}

pub trait Invert {
    fn invert_x(self) -> Self;
    fn invert_y(self) -> Self;
}

macro_rules! impl_invert {
    ($name:ident) => {
        impl Invert for $name {
            fn invert_x(self) -> Self {
                Self::new(-self.x, self.y)
            }

            fn invert_y(self) -> Self {
                Self::new(self.x, -self.y)
            }
        }
    };
}

pub type Vector = nalgebra::Vector2<f64>;
pub type Position = nalgebra::Point2<f64>;
pub type Size = nalgebra::Vector2<f64>;

impl_invert!(Vector);
impl_invert!(Position);

pub mod deduplicate {
    use crate::Position;

    pub trait DedupEpsilon {
        fn dedup_with_epsilon(self, epsilon: f64) -> Self;
    }

    impl DedupEpsilon for Vec<Position> {
        fn dedup_with_epsilon(mut self, epsilon: f64) -> Self {
            if self.len() < 2 {
                return self;
            }

            let mut to_remove = Vec::new();
            let mut last_index = 0;

            for i in 1..self.len() {
                let a = &self[last_index];
                let b = &self[i];
                if (a.x - b.x).abs() < epsilon && (a.y - b.y).abs() < epsilon {
                    to_remove.push(i);
                } else {
                    last_index = i;
                }
            }

            if self.len() - to_remove.len() < 3 {
                return self; // Too few remaining
            }

            for &i in to_remove.iter().rev() {
                self.remove(i);
            }

            self
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_empty_vec() {
            let vertices: Vec<Position> = vec![];
            let result = vertices.dedup_with_epsilon(0.001);
            assert_eq!(result.len(), 0);
        }

        #[test]
        fn test_single_element() {
            let vertices = vec![Position::new(1.0, 2.0)];
            let result = vertices.dedup_with_epsilon(0.001);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].x, 1.0);
            assert_eq!(result[0].y, 2.0);
        }

        #[test]
        fn test_no_duplicates() {
            let vertices = vec![
                Position::new(0.0, 0.0),
                Position::new(1.0, 1.0),
                Position::new(2.0, 2.0),
            ];

            let expected_result = vertices.clone();

            // when
            let result = vertices.dedup_with_epsilon(0.0001);

            // then
            assert_eq!(result, expected_result);
        }

        #[test]
        fn test_with_adjacent_duplicates() {
            let vertices = vec![
                Position::new(0.0, 0.0),
                Position::new(0.0, 0.0),
                Position::new(1.0, 1.0),
                Position::new(2.0, 2.0),
            ];
            let result = vertices.dedup_with_epsilon(1e-6);
            assert_eq!(result.len(), 3);
            assert_eq!(result[0], Position::new(0.0, 0.0));
            assert_eq!(result[1], Position::new(1.0, 1.0));
            assert_eq!(result[2], Position::new(2.0, 2.0));
        }

        #[test]
        fn test_dedup_would_leave_too_few() {
            let vertices = vec![
                Position::new(0.0, 0.0),
                Position::new(0.0, 0.0),
                Position::new(0.0, 0.0),
            ];
            let result = vertices
                .clone()
                .dedup_with_epsilon(1e-6);
            assert_eq!(result, vertices); // Should return original
        }

        #[test]
        fn test_dedup_edge_epsilon() {
            // given
            let vertices = vec![
                Position::new(0.0, 0.0),
                // ensure positive numbers on y axis are detected
                Position::new(0.0, 0.0000005), // Within epsilon of first point
                Position::new(0.0, 0.0000009), // Within epsilon of removed point and first point
                // ensure negative numbers on x axis are detected
                Position::new(-3.0000000, 1.0),
                Position::new(-3.0000001, 1.0), // Within epsilon
                // ensure negative numbers on y axis are detected
                Position::new(2.0, -2.0),
                Position::new(2.0, -2.0000001),
                // ensure positive numbers on x axis are detected
                Position::new(4.0, 0.0),
                Position::new(4.00000001, 0.0),
            ];

            // and
            let expected_result = vec![
                Position::new(0.0, 0.0),
                Position::new(-3.0, 1.0),
                Position::new(2.0, -2.0),
                Position::new(4.0, 0.0),
            ];

            // when
            let result = vertices.dedup_with_epsilon(0.000001);

            // then
            assert_eq!(result, expected_result);
        }
    }
}
