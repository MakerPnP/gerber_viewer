use std::sync::Arc;

use egui::epaint::emath::Align2;
use egui::epaint::{Color32, FontId, Mesh, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, Vertex};

use crate::Transform2D;
use crate::layer::{GerberPrimitive, ViewState};
use crate::position::Vector;
use crate::{GerberLayer, Mirroring, color};

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
        // radians (positive=clockwise)
        rotation: f32,
        mirroring: Mirroring,
        // in gerber coordinates
        design_origin: Vector,
        // in gerber coordinates
        design_offset: Vector,
    ) {
        let relative_origin = Vector::new(design_origin.x, -design_origin.y);
        let offset = Vector::new(design_offset.x, -design_offset.y);

        let origin = relative_origin - offset;

        let transform = Transform2D {
            rotation_radians: rotation,
            mirroring,
            origin,
            offset,
        };

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

                    let center = Pos2::new(center.x as f32, -(center.y as f32));

                    let center = view.translation.to_pos2() + transform.apply_to_pos2(center) * view.scale;

                    let radius = (*diameter as f32 / 2.0) * view.scale;
                    #[cfg(feature = "egui")]
                    painter.circle(center, radius, color, Stroke::NONE);
                }
                GerberPrimitive::Rectangle {
                    origin,
                    width,
                    height,
                    exposure,
                } => {
                    let color = exposure.to_color(&color);

                    // Calculate center-based position
                    let screen_center = Pos2::new(
                        origin.x as f32 + *width as f32 / 2.0,     // Add half width to get center
                        -(origin.y as f32 + *height as f32 / 2.0), // Flip Y and add half height
                    );

                    let angle_normalized = rotation.to_degrees().rem_euclid(360.0);
                    let is_axis_aligned = (angle_normalized - 0.0).abs() < f32::EPSILON
                        || (angle_normalized - 90.0).abs() < f32::EPSILON
                        || (angle_normalized - 180.0).abs() < f32::EPSILON
                        || (angle_normalized - 270.0).abs() < f32::EPSILON;

                    if is_axis_aligned {
                        // Fast-path: axis-aligned rectangle (mirroring allowed, since mirroring across axis doesn't affect axis-alignment)
                        let center = (view.translation + transform.apply_to_pos2(screen_center) * view.scale).to_pos2();
                        let size = Vec2::new(*width as f32, *height as f32) * view.scale;
                        let top_left = center - size / 2.0; // Calculate top-left from center

                        painter.rect(
                            Rect::from_min_size(top_left, size),
                            0.0,
                            color,
                            Stroke::NONE,
                            StrokeKind::Middle,
                        );
                    } else {
                        // Arbitrary rotation: draw as polygon
                        let hw = *width as f32 / 2.0;
                        let hh = *height as f32 / 2.0;

                        // Define corners in local space (centered)
                        let corners = [
                            Pos2::new(-hw, -hh),
                            Pos2::new(hw, -hh),
                            Pos2::new(hw, hh),
                            Pos2::new(-hw, hh),
                        ];

                        let screen_corners: Vec<Pos2> = corners
                            .iter()
                            .map(|corner| {
                                (view.translation
                                    + transform.apply_to_pos2(screen_center + (*corner).to_vec2()) * view.scale)
                                    .to_pos2()
                            })
                            .collect();

                        painter.add(Shape::convex_polygon(screen_corners, color, Stroke::NONE));
                    }
                }
                GerberPrimitive::Line {
                    start,
                    end,
                    width,
                    exposure,
                } => {
                    let color = exposure.to_color(&color);

                    let start_position = Pos2::new(start.x as f32, -(start.y as f32));
                    let end_position = Pos2::new(end.x as f32, -(end.y as f32));

                    let start_position = view.translation + transform.apply_to_pos2(start_position) * view.scale;
                    let end_position = view.translation + transform.apply_to_pos2(end_position) * view.scale;

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

                    let screen_center = Pos2::new(center.x as f32, -(center.y as f32));

                    if geometry.is_convex {
                        // Direct convex rendering
                        let screen_vertices: Vec<Pos2> = geometry
                            .relative_vertices
                            .iter()
                            .map(|v| {
                                let local = Vec2::new(v.x as f32, -v.y as f32);
                                let position = (view.translation
                                    + transform.apply_to_pos2(screen_center + local) * view.scale)
                                    .to_pos2();
                                position
                            })
                            .collect();

                        painter.add(Shape::convex_polygon(screen_vertices, color, Stroke::NONE));
                    } else if let Some(tess) = &geometry.tessellation {
                        // Transform tessellated geometry
                        let vertices: Vec<Vertex> = tess
                            .vertices
                            .iter()
                            .map(|[x, y]| {
                                let local = Vec2::new(*x, -*y); // Flip Y just like convex path
                                let position = (view.translation
                                    + transform.apply_to_pos2(screen_center + local) * view.scale)
                                    .to_pos2();
                                Vertex {
                                    pos: position,
                                    uv: egui::epaint::WHITE_UV,
                                    color,
                                }
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
                                let local = Vec2::new(v.x as f32, -v.y as f32);
                                let position = (view.translation
                                    + transform.apply_to_pos2(screen_center + local) * view.scale)
                                    .to_pos2();
                                position
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
