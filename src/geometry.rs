use crate::Position;

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self {
            min_x: f64::MAX,
            min_y: f64::MAX,
            max_x: f64::MIN,
            max_y: f64::MIN,
        }
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
