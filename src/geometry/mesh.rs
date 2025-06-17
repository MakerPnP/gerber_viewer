use nalgebra::Point2;

#[derive(Debug, Clone)]
pub struct PolygonMesh {
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

pub fn tessellate_polygon(vertices: &[Point2<f64>]) -> PolygonMesh {
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
