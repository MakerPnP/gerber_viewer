use std::sync::Arc;

use egui::epaint::emath::Align2;
use egui::epaint::{Color32, FontId, Mesh, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, Vertex};

use crate::layer::{GerberPrimitive, ViewState};
use crate::{GerberLayer, color};

#[derive(Default)]
pub struct GerberRenderer {}

impl GerberRenderer {
    pub fn paint_layer(
        &self,
        painter: &egui::Painter,
        view: ViewState,
        layer: &GerberLayer,
        base_color: Color32,
        use_unique_shape_colors: bool,
        use_polygon_numbering: bool,
    ) {
        for (index, primitive) in layer.primitives().iter().enumerate() {
            let color = match use_unique_shape_colors {
                true => color::generate_pastel_color(index as u64),
                false => base_color,
            };

            match primitive {
                GerberPrimitive::Circle {
                    center,
                    diameter,
                    exposure,
                } => {
                    let color = exposure.to_color(&color);

                    let center = view.translation + center.invert_y().to_vec2() * view.scale;
                    let radius = (*diameter as f32 / 2.0) * view.scale;
                    #[cfg(feature = "egui")]
                    painter.circle(center.to_pos2(), radius, color, Stroke::NONE);
                }
                GerberPrimitive::Rectangle {
                    origin,
                    width,
                    height,
                    exposure,
                } => {
                    let color = exposure.to_color(&color);

                    // Calculate center-based position
                    let center = view.translation
                        + Vec2::new(
                            origin.x as f32 + *width as f32 / 2.0,     // Add half width to get center
                            -(origin.y as f32 + *height as f32 / 2.0), // Flip Y and add half height
                        ) * view.scale;

                    let size = Vec2::new(*width as f32, *height as f32) * view.scale;
                    let top_left = center - size / 2.0; // Calculate top-left from center

                    painter.rect(
                        Rect::from_min_size(top_left.to_pos2(), size),
                        0.0,
                        color,
                        Stroke::NONE,
                        StrokeKind::Middle,
                    );
                }
                GerberPrimitive::Line {
                    start,
                    end,
                    width,
                    exposure,
                } => {
                    let color = exposure.to_color(&color);

                    let start_position = view.translation + Vec2::new(start.x as f32, -(start.y as f32)) * view.scale;
                    let end_position = view.translation + Vec2::new(end.x as f32, -(end.y as f32)) * view.scale;
                    painter.line_segment(
                        [start_position.to_pos2(), end_position.to_pos2()],
                        Stroke::new((*width as f32) * view.scale, color),
                    );
                    // Draw circles at either end of the line.
                    let radius = (*width as f32 / 2.0) * view.scale;
                    painter.circle(start_position.to_pos2(), radius, color, Stroke::NONE);
                    painter.circle(end_position.to_pos2(), radius, color, Stroke::NONE);
                }
                GerberPrimitive::Polygon {
                    center,
                    exposure,
                    geometry,
                } => {
                    let color = exposure.to_color(&color);
                    let screen_center = Vec2::new(
                        view.translation.x + (center.x as f32) * view.scale,
                        view.translation.y - (center.y as f32) * view.scale,
                    );

                    if geometry.is_convex {
                        // Direct convex rendering
                        let screen_vertices: Vec<Pos2> = geometry
                            .relative_vertices
                            .iter()
                            .map(|v| {
                                (screen_center + Vec2::new(v.x as f32 * view.scale, -v.y as f32 * view.scale)).to_pos2()
                            })
                            .collect();

                        painter.add(Shape::convex_polygon(screen_vertices, color, Stroke::NONE));
                    } else if let Some(tess) = &geometry.tessellation {
                        // Transform tessellated geometry
                        let vertices: Vec<Vertex> = tess
                            .vertices
                            .iter()
                            .map(|[x, y]| Vertex {
                                pos: (screen_center + Vec2::new(*x * view.scale, -*y * view.scale)).to_pos2(),
                                uv: egui::epaint::WHITE_UV,
                                color,
                            })
                            .collect();

                        painter.add(Shape::Mesh(Arc::new(Mesh {
                            vertices,
                            indices: tess.indices.clone(),
                            texture_id: egui::TextureId::default(),
                        })));
                    }

                    if use_polygon_numbering {
                        // Debug visualization
                        let debug_vertices: Vec<Pos2> = geometry
                            .relative_vertices
                            .iter()
                            .map(|v| {
                                let point =
                                    screen_center + Vec2::new(v.x as f32 * view.scale, -v.y as f32 * view.scale);
                                point.to_pos2()
                            })
                            .collect();

                        for (i, pos) in debug_vertices.iter().enumerate() {
                            painter.text(
                                *pos,
                                Align2::CENTER_CENTER,
                                format!("{}", i),
                                FontId::monospace(8.0),
                                Color32::RED,
                            );
                        }
                    }
                }
            }
        }
    }
}
