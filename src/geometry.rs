use egui::{Pos2, Vec2, Vec2b};
use log::debug;

use crate::spacial::Vector;
use crate::Position;

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

#[derive(Debug, Copy, Clone)]
pub struct Transform2D {
    pub rotation_radians: f32,
    pub mirroring: Mirroring,
    // origin for rotation and mirroring, in gerber coordinates
    pub origin: Vector,
    // offset, in gerber coordinates
    pub offset: Vector,
}

impl Transform2D {
    /// Apply the transform to a logical `Position` (Gerber-space)
    pub fn apply_to_position(&self, pos: Position) -> Position {
        let mut x = pos.x - self.origin.x;
        let mut y = pos.y - self.origin.y;

        if self.mirroring.x {
            x = -x;
        }
        if self.mirroring.y {
            y = -y;
        }

        let (sin_theta, cos_theta) = (-self.rotation_radians as f64).sin_cos();
        let rotated_x = x * cos_theta - y * sin_theta;
        let rotated_y = x * sin_theta + y * cos_theta;

        Position::new(
            rotated_x + self.origin.x + self.offset.x,
            rotated_y + self.origin.y + self.offset.y,
        )
    }

    /// Apply transform to a Vec2 instead of Position (used for bbox drawing)
    pub fn apply_to_pos2(&self, pos: Pos2) -> Vec2 {
        let mut x = pos.x as f64 - self.origin.x;
        let mut y = pos.y as f64 - self.origin.y;

        if self.mirroring.x {
            x = -x;
        }
        if self.mirroring.y {
            y = -y;
        }

        let (sin_theta, cos_theta) = (self.rotation_radians as f64).sin_cos();
        let rotated_x = x * cos_theta - y * sin_theta;
        let rotated_y = x * sin_theta + y * cos_theta;

        Vec2::new(
            (rotated_x + self.origin.x + self.offset.x) as f32,
            (rotated_y + self.origin.y + self.offset.y) as f32,
        )
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct BoundingBox {
    pub min: Position,
    pub max: Position,
}

impl BoundingBox {
    /// Use to generate an outline of the bbox
    pub fn transform_vertices(&self, transform: Transform2D) -> Vec<Position> {
        self.vertices()
            .into_iter()
            .map(|v| transform.apply_to_position(v))
            .collect::<Vec<_>>()
    }

    pub fn expand(&mut self, other: &BoundingBox) {
        self.min.x = self.min.x.min(other.min.x);
        self.min.y = self.min.y.min(other.min.y);
        self.max.x = self.max.x.max(other.max.x);
        self.max.y = self.max.y.max(other.max.y);
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self {
            min: Position::MAX,
            max: Position::MIN,
        }
    }
}

impl BoundingBox {
    /// Note that a bounding box of 0,0 -> 0,0 is NOT empty
    /// e.g., you could have a shape that defines a rectangle with an origin of 0,0 and a width + height of 0,0.
    ///
    /// Only a bounding box which is the same as the one returned by `default` counts as empty.
    pub fn is_empty(&self) -> bool {
        self.eq(&BoundingBox::default())
    }

    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }

    pub fn apply_transform(&self, transform: Transform2D) -> Self {
        // Step 1: Transform each corner of the original bbox
        let transformed_bbox_vertices: Vec<_> = self
            .vertices()
            .into_iter()
            .map(|v| transform.apply_to_position(v))
            .collect();

        // Step 2: Create a new axis-aligned bbox from transformed points (for viewport fitting)
        let result = BoundingBox::from_points(&transformed_bbox_vertices);
        debug!(
            "Applying transform.  transform {:?}: before: {:?}, after: {:?}",
            transform, self, result
        );
        result
    }

    /// Returns a new bounding box with X and/or Y mirroring applied.
    pub fn apply_mirroring(&self, mirror_x: bool, mirror_y: bool, offset: Vector) -> Self {
        let mut vertices = self.vertices();

        for Position {
            x,
            y,
        } in &mut vertices
        {
            if mirror_x {
                *x = offset.x - (*x - offset.x);
            }
            if mirror_y {
                *y = offset.y - (*y - offset.y);
            }
        }

        Self::from_points(&vertices)
    }

    /// Returns a new bounding box rotated around origin (0, 0) by given angle in radians.
    pub fn apply_rotation(&self, radians: f64, offset: Vector) -> Self {
        let (sin_theta, cos_theta) = radians.sin_cos();
        let mut corners = self.vertices();

        for pt in &mut corners {
            let x = pt.x - offset.x;
            let y = pt.y - offset.y;

            let rotated_x = x * cos_theta - y * sin_theta;
            let rotated_y = x * sin_theta + y * cos_theta;

            pt.x = rotated_x + offset.x;
            pt.y = rotated_y + offset.y;
        }

        Self::from_points(&corners)
    }

    /// Returns the geometric center of the bounding box as a Position
    pub fn center(&self) -> Position {
        (self.min + self.max) / 2.0
    }

    /// Returns 4 corner points of the bounding box such that the result is useable as a closed path.
    /// ```plaintext
    /// (min_x, min_y) 1 ┌────────────┐ 2 (max_x, min_y)
    ///                  │            │
    /// (min_x, max_y) 4 └────────────┘ 3 (max_x, max_y)
    /// ```
    pub fn vertices(&self) -> Vec<Position> {
        vec![
            Position::new(self.min.x, self.min.y),
            Position::new(self.max.x, self.min.y),
            Position::new(self.max.x, self.max.y),
            Position::new(self.min.x, self.max.y),
        ]
    }

    /// Constructs a bounding box from a list of points
    pub fn from_points(points: &[Position]) -> Self {
        let mut min = Position::MAX;
        let mut max = Position::MIN;

        for &Position {
            x,
            y,
        } in points
        {
            min.x = min.x.min(x);
            min.y = min.y.min(y);
            max.x = max.x.max(x);
            max.y = max.y.max(y);
        }

        Self {
            min,
            max,
        }
    }
}

#[cfg(test)]
mod bbox_tests {
    use rstest::rstest;

    use super::BoundingBox;
    use crate::spacial::Vector;
    use crate::Position;

    #[rstest]
    #[case(BoundingBox::default(), true)]
    #[case(BoundingBox { min: Position::new(0.0, 0.0), max: Position::new(0.0, 0.0) }, false)]
    #[case(BoundingBox { min: Position::new(-10.0, -10.0), max: Position::new(10.0, 10.0) }, false)]
    pub fn test_is_empty(#[case] input: BoundingBox, #[case] expected: bool) {
        assert_eq!(input.is_empty(), expected);
    }

    #[test]
    pub fn test_apply_rotation_90_degrees_zero_offset() {
        let bbox = BoundingBox {
            min: Position::new(1.0, 2.0),
            max: Position::new(3.0, 4.0),
        };

        let rotated = bbox.apply_rotation(std::f64::consts::FRAC_PI_2, Vector::ZERO); // 90 degrees

        // Expected:
        // Points rotate CCW around origin:
        // (1,2) => (-2,1)
        // (1,4) => (-4,1)
        // (3,2) => (-2,3)
        // (3,4) => (-4,3)
        //
        // So bounds are:
        // min_x = -4, max_x = -2
        // min_y = 1,  max_y = 3

        assert!((rotated.min.x - -4.0).abs() < 1e-6);
        assert!((rotated.max.x - -2.0).abs() < 1e-6);
        assert!((rotated.min.y - 1.0).abs() < 1e-6);
        assert!((rotated.max.y - 3.0).abs() < 1e-6);
    }

    #[rstest]
    #[case((0.0, 0.0), (10.0, 10.0), (5.0, 5.0))] // Case 1: Origin 0, 10x10
    #[case((10.0, 10.0), (10.0, 10.0), (15.0, 15.0))] // Case 2: Origin 10, 10x10
    #[case((0.0, 0.0), (5.0, 10.0), (2.5, 5.0))] // Case 3: Origin 0, 5x10
    #[case((0.0, 0.0), (10.0, 5.0), (5.0, 2.5))] // Case 4: Origin 0, 10x5
    #[case((10.0, 10.0), (5.0, 10.0), (12.5, 15.0))] // Case 5: Origin 10, 5x10
    #[case((10.0, 10.0), (10.0, 5.0), (15.0, 12.5))] // Case 6: Origin 10, 10x5
    fn test_geometric_center(#[case] origin: (f64, f64), #[case] size: (f64, f64), #[case] expected: (f64, f64)) {
        // Create bounding box from origin and size
        let bbox = BoundingBox {
            min: Position {
                x: origin.0,
                y: origin.1,
            },
            max: Position {
                x: origin.0 + size.0,
                y: origin.1 + size.1,
            },
        };

        let center = bbox.center();

        // Compare with precision to handle floating-point numbers
        let epsilon = 1e-9;
        assert!(
            (center.x - expected.0).abs() < epsilon,
            "X mismatch: expected {}, got {}",
            expected.0,
            center.x
        );
        assert!(
            (center.y - expected.1).abs() < epsilon,
            "Y mismatch: expected {}, got {}",
            expected.1,
            center.y
        );
    }
}

pub fn is_convex(vertices: &[Position]) -> bool {
    if vertices.len() < 3 {
        return true;
    }

    let n = vertices.len();
    let mut sign = 0;

    for i in 0..n {
        let p1 = vertices[i];
        let p2 = vertices[(i + 1) % n];
        let p3 = vertices[(i + 2) % n];

        let v1 = p2 - p1;
        let v2 = p3 - p2;

        // Cross product in 2D
        let cross = v1.x * v2.y - v1.y * v2.x;

        if sign == 0 {
            sign = if cross > 0.0 { 1 } else { -1 };
        } else if (cross > 0.0 && sign < 0) || (cross < 0.0 && sign > 0) {
            return false;
        }
    }

    true
}

#[derive(Debug, Clone)]
pub struct PolygonMesh {
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

pub fn tessellate_polygon(vertices: &[Position]) -> PolygonMesh {
    use lyon::path::Path;
    use lyon::tessellation::{BuffersBuilder, FillOptions, FillRule, FillTessellator, VertexBuffers};

    let mut path_builder = Path::builder();
    if let Some(first) = vertices.first() {
        path_builder.begin(lyon::math::Point::new(first.x as f32, first.y as f32));
        for pos in &vertices[1..] {
            path_builder.line_to(lyon::math::Point::new(pos.x as f32, pos.y as f32));
        }
        path_builder.close();
    }
    let path = path_builder.build();

    let mut geometry = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();

    tessellator
        .tessellate_path(
            &path,
            &FillOptions::default().with_fill_rule(FillRule::EvenOdd),
            &mut BuffersBuilder::new(&mut geometry, |vertex: lyon::tessellation::FillVertex| {
                [vertex.position().x, vertex.position().y]
            }),
        )
        .unwrap();

    PolygonMesh {
        vertices: geometry.vertices,
        indices: geometry.indices,
    }
}
