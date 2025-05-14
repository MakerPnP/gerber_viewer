use std::ops::Add;

#[cfg(feature = "egui")]
use egui::{Pos2, Vec2};

#[allow(dead_code)]
impl Position {
    #[cfg(feature = "egui")]
    pub const fn to_pos2(self) -> Pos2 {
        Pos2::new(self.x as f32, self.y as f32)
    }

    pub const fn to_vector(self) -> Vector {
        Vector {
            x: self.x,
            y: self.y,
        }
    }
}

#[cfg(feature = "egui")]
impl From<Vec2> for Position {
    fn from(value: Vec2) -> Self {
        Self {
            x: value.x as f64,
            y: value.y as f64,
        }
    }
}

impl From<(f64, f64)> for Position {
    fn from(value: (f64, f64)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

#[cfg(feature = "egui")]
impl Add<Vec2> for Position {
    type Output = Position;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x as f64,
            y: self.y + rhs.y as f64,
        }
    }
}

impl Vector {
    pub const fn to_position(self) -> Position {
        Position {
            x: self.x,
            y: self.y,
        }
    }
}

macro_rules! impl_constructor {
    ($name:ident, $t:ty) => {
        #[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
        pub struct $name {
            pub x: $t,
            pub y: $t,
        }

        impl $name {
            pub const fn new(x: $t, y: $t) -> Self {
                Self {
                    x,
                    y,
                }
            }
        }
    };
}

macro_rules! impl_consts {
    ($name:ident, $t:ty) => {
        impl $name {
            pub const ZERO: $name = $name::new(0.0, 0.0);
            pub const MAX: $name = $name::new(<$t>::MAX, <$t>::MAX);
            pub const MIN: $name = $name::new(<$t>::MIN, <$t>::MIN);
        }
    };
}
macro_rules! impl_ops {
    ($name:ident) => {
        impl core::ops::Add for $name {
            type Output = Self;

            fn add(self, rhs: Self) -> Self::Output {
                Self {
                    x: self.x + rhs.x,
                    y: self.y + rhs.y,
                }
            }
        }

        impl core::ops::Sub for $name {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self::Output {
                Self {
                    x: self.x - rhs.x,
                    y: self.y - rhs.y,
                }
            }
        }

        impl core::ops::Mul for $name {
            type Output = Self;

            fn mul(self, rhs: Self) -> Self::Output {
                Self {
                    x: self.x * rhs.x,
                    y: self.y * rhs.y,
                }
            }
        }

        impl core::ops::Div for $name {
            type Output = Self;

            fn div(self, rhs: Self) -> Self::Output {
                Self {
                    x: self.x / rhs.x,
                    y: self.y / rhs.y,
                }
            }
        }

        impl core::ops::Div<f64> for $name {
            type Output = $name;

            fn div(self, rhs: f64) -> Self::Output {
                Self {
                    x: self.x / rhs,
                    y: self.y / rhs,
                }
            }
        }

        impl core::ops::Mul<f64> for $name {
            type Output = $name;

            fn mul(self, rhs: f64) -> Self::Output {
                Self {
                    x: self.x * rhs,
                    y: self.y * rhs,
                }
            }
        }

        impl core::ops::Add<f64> for $name {
            type Output = $name;

            fn add(self, rhs: f64) -> Self::Output {
                Self {
                    x: self.x + rhs,
                    y: self.y + rhs,
                }
            }
        }

        impl core::ops::Sub<f64> for $name {
            type Output = $name;

            fn sub(self, rhs: f64) -> Self::Output {
                Self {
                    x: self.x - rhs,
                    y: self.y - rhs,
                }
            }
        }

        impl core::ops::AddAssign for $name {
            fn add_assign(&mut self, rhs: Self) {
                self.x += rhs.x;
                self.y += rhs.y;
            }
        }

        impl core::ops::SubAssign for $name {
            fn sub_assign(&mut self, rhs: Self) {
                self.x -= rhs.x;
                self.y -= rhs.y;
            }
        }

        impl core::ops::MulAssign for $name {
            fn mul_assign(&mut self, rhs: Self) {
                self.x *= rhs.x;
                self.y *= rhs.y;
            }
        }

        impl core::ops::DivAssign for $name {
            fn div_assign(&mut self, rhs: Self) {
                self.x /= rhs.x;
                self.y /= rhs.y;
            }
        }
    };
}

macro_rules! impl_invert {
    ($name:ident) => {
        impl $name {
            pub const fn invert_x(self) -> Self {
                Self {
                    x: -self.x,
                    y: self.y,
                }
            }

            pub const fn invert_y(self) -> Self {
                Self {
                    x: self.x,
                    y: -self.y,
                }
            }
        }
    };
}

impl_constructor!(Vector, f64);
impl_consts!(Vector, f64);
impl_invert!(Vector);
impl_ops!(Vector);

impl_constructor!(Position, f64);
impl_consts!(Position, f64);
impl_invert!(Position);
impl_ops!(Position);

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
            let vertices = vec![Position {
                x: 1.0,
                y: 2.0,
            }];
            let result = vertices.dedup_with_epsilon(0.001);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].x, 1.0);
            assert_eq!(result[0].y, 2.0);
        }

        #[test]
        fn test_no_duplicates() {
            let vertices = vec![
                Position {
                    x: 0.0,
                    y: 0.0,
                },
                Position {
                    x: 1.0,
                    y: 1.0,
                },
                Position {
                    x: 2.0,
                    y: 2.0,
                },
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
                Position {
                    x: 0.0,
                    y: 0.0,
                },
                Position {
                    x: 0.0,
                    y: 0.0,
                }, // dup
                Position {
                    x: 1.0,
                    y: 1.0,
                },
                Position {
                    x: 2.0,
                    y: 2.0,
                },
            ];
            let result = vertices.dedup_with_epsilon(1e-6);
            assert_eq!(result.len(), 3);
            assert_eq!(result[0], Position {
                x: 0.0,
                y: 0.0
            });
            assert_eq!(result[1], Position {
                x: 1.0,
                y: 1.0
            });
            assert_eq!(result[2], Position {
                x: 2.0,
                y: 2.0
            });
        }

        #[test]
        fn test_dedup_would_leave_too_few() {
            let vertices = vec![
                Position {
                    x: 0.0,
                    y: 0.0,
                },
                Position {
                    x: 0.0,
                    y: 0.0,
                }, // dup
                Position {
                    x: 0.0,
                    y: 0.0,
                }, // dup
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
                Position {
                    x: 0.0,
                    y: 0.0,
                },
                // ensure positive numbers on y axis are detected
                Position {
                    x: 0.0,
                    y: 0.0000005,
                }, // Within epsilon of first point
                Position {
                    x: 0.0,
                    y: 0.0000009,
                }, // Within epsilon of removed point and first point
                // ensure negative numbers on x axis are detected
                Position {
                    x: -3.0000000,
                    y: 1.0,
                },
                Position {
                    x: -3.0000001,
                    y: 1.0,
                }, // Within epsilon
                // ensure negative numbers on y axis are detected
                Position {
                    x: 2.0,
                    y: -2.0,
                },
                Position {
                    x: 2.0,
                    y: -2.0000001,
                },
                // ensure positive numbers on x axis are detected
                Position {
                    x: 4.0,
                    y: 0.0,
                },
                Position {
                    x: 4.00000001,
                    y: 0.0,
                },
            ];

            // and
            let expected_result = vec![
                Position {
                    x: 0.0,
                    y: 0.0,
                },
                Position {
                    x: -3.0,
                    y: 1.0,
                },
                Position {
                    x: 2.0,
                    y: -2.0,
                },
                Position {
                    x: 4.0,
                    y: 0.0,
                },
            ];

            // when
            let result = vertices.dedup_with_epsilon(0.000001);

            // then
            assert_eq!(result, expected_result);
        }
    }
}
