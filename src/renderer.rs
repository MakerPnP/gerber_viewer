use std::sync::Arc;

use egui::epaint::emath::Align2;
use egui::epaint::{
    Color32, ColorMode, FontId, Mesh, PathShape, PathStroke, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2, Vertex,
};
use egui::Painter;
use nalgebra::Vector2;

use crate::layer::GerberPrimitive;
use crate::{color, GerberLayer, Mirroring, ViewState};
use crate::{
    ArcGerberPrimitive, CircleGerberPrimitive, LineGerberPrimitive, PolygonGerberPrimitive, RectangleGerberPrimitive,
    Transform2D,
};

#[derive(Default)]
pub struct GerberRenderer {}

impl GerberRenderer {
    #[profiling::function]
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
        design_origin: Vector2<f64>,
        // in gerber coordinates
        design_offset: Vector2<f64>,
    ) {
        let relative_origin = Vector2::new(design_origin.x, -design_origin.y);
        let offset = Vector2::new(design_offset.x, -design_offset.y);

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
                GerberPrimitive::Circle(circle) => {
                    circle.render(painter, &view, &transform, color, rotation, use_polygon_numbering)
                }
                GerberPrimitive::Rectangle(rect) => {
                    rect.render(painter, &view, &transform, color, rotation, use_polygon_numbering)
                }
                GerberPrimitive::Line(line) => {
                    line.render(painter, &view, &transform, color, rotation, use_polygon_numbering)
                }
                GerberPrimitive::Arc(arc) => {
                    arc.render(painter, &view, &transform, color, rotation, use_polygon_numbering)
                }
                GerberPrimitive::Polygon(polygon) => {
                    polygon.render(painter, &view, &transform, color, rotation, use_polygon_numbering)
                }
            }
        }
    }
}

trait Renderable {
    fn render(
        &self,
        painter: &Painter,
        view: &ViewState,
        transform: &Transform2D,
        color: Color32,
        rotation: f32,
        use_polygon_numbering: bool,
    );
}

impl Renderable for CircleGerberPrimitive {
    #[cfg_attr(feature = "profile-renderables", profiling::function)]
    fn render(
        &self,
        painter: &Painter,
        view: &ViewState,
        transform: &Transform2D,
        color: Color32,
        _rotation: f32,
        _use_polygon_numbering: bool,
    ) {
        let Self {
            center,
            diameter,
            exposure,
        } = self;

        let color = exposure.to_color(&color);

        let center = Pos2::new(center.x as f32, -(center.y as f32));

        let center = view.translation.to_pos2() + transform.apply_to_pos2(center) * view.scale;

        let radius = (*diameter as f32 / 2.0) * view.scale;
        #[cfg(feature = "egui")]
        painter.circle(center, radius, color, Stroke::NONE);
    }
}

impl Renderable for RectangleGerberPrimitive {
    #[cfg_attr(feature = "profile-renderables", profiling::function)]
    fn render(
        &self,
        painter: &Painter,
        view: &ViewState,
        transform: &Transform2D,
        color: Color32,
        rotation: f32,
        _use_polygon_numbering: bool,
    ) {
        let Self {
            origin,
            width,
            height,
            exposure,
        } = self;

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
            // Determine if width/height should be swapped
            let mut width = *width as f32;
            let mut height = *height as f32;

            if (angle_normalized - 90.0).abs() < f32::EPSILON || (angle_normalized - 270.0).abs() < f32::EPSILON {
                std::mem::swap(&mut width, &mut height);
            }

            let center = (view.translation + transform.apply_to_pos2(screen_center) * view.scale).to_pos2();
            let size = Vec2::new(width, height) * view.scale;
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
                    (view.translation + transform.apply_to_pos2(screen_center + (*corner).to_vec2()) * view.scale)
                        .to_pos2()
                })
                .collect();

            painter.add(Shape::convex_polygon(screen_corners, color, Stroke::NONE));
        }
    }
}

impl Renderable for LineGerberPrimitive {
    #[cfg_attr(feature = "profile-renderables", profiling::function)]
    fn render(
        &self,
        painter: &Painter,
        view: &ViewState,
        transform: &Transform2D,
        color: Color32,
        _rotation: f32,
        _use_polygon_numbering: bool,
    ) {
        let Self {
            start,
            end,
            width,
            exposure,
        } = self;
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
}

impl Renderable for ArcGerberPrimitive {
    #[cfg_attr(feature = "profile-renderables", profiling::function)]
    fn render(
        &self,
        painter: &Painter,
        view: &ViewState,
        transform: &Transform2D,
        color: Color32,
        _rotation: f32,
        _use_polygon_numbering: bool,
    ) {
        let Self {
            center,
            radius,
            width,
            start_angle,
            sweep_angle,
            exposure,
        } = self;
        let color = exposure.to_color(&color);
        let screen_center = Pos2::new(center.x as f32, -(center.y as f32));

        // Check if this is a full circle
        let is_full_circle = self.is_full_circle();

        let steps = if is_full_circle { 33 } else { 32 };
        let mut points = Vec::with_capacity(steps);

        let effective_sweep = if is_full_circle {
            2.0 * std::f64::consts::PI
        } else {
            *sweep_angle
        };

        // Calculate the absolute sweep for determining the step size
        let abs_sweep = effective_sweep.abs();
        let angle_step = abs_sweep / (steps - 1) as f64;

        // Generate points along the outer radius
        for i in 0..steps {
            // Adjust the angle based on sweep direction
            let angle = if effective_sweep >= 0.0 {
                start_angle + angle_step * i as f64
            } else {
                start_angle - angle_step * i as f64
            };

            let x = *radius * angle.cos();
            let y = *radius * angle.sin();

            let local = Vec2::new(x as f32, -y as f32);
            let position = (view.translation + transform.apply_to_pos2(screen_center + local) * view.scale).to_pos2();

            points.push(position);
        }

        // Ensure exact closure for full circles
        if is_full_circle {
            points[steps - 1] = points[0];
        }

        painter.add(Shape::Path(PathShape {
            points,
            closed: is_full_circle,
            fill: Color32::TRANSPARENT,
            stroke: PathStroke {
                width: *width as f32 * view.scale,
                color: ColorMode::Solid(color),
                kind: StrokeKind::Middle,
            },
        }));
    }
}

impl Renderable for PolygonGerberPrimitive {
    #[cfg_attr(feature = "profile-renderables", profiling::function)]
    fn render(
        &self,
        painter: &Painter,
        view: &ViewState,
        transform: &Transform2D,
        color: Color32,
        _rotation: f32,
        use_polygon_numbering: bool,
    ) {
        let Self {
            center,
            exposure,
            geometry,
        } = self;
        let color = exposure.to_color(&color);

        let screen_center = Pos2::new(center.x as f32, -(center.y as f32));

        if geometry.is_convex {
            // Direct convex rendering
            let screen_vertices: Vec<Pos2> = geometry
                .relative_vertices
                .iter()
                .map(|v| {
                    let local = Vec2::new(v.x as f32, -v.y as f32);
                    let position =
                        (view.translation + transform.apply_to_pos2(screen_center + local) * view.scale).to_pos2();
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
                    let position =
                        (view.translation + transform.apply_to_pos2(screen_center + local) * view.scale).to_pos2();
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
                    let position =
                        (view.translation + transform.apply_to_pos2(screen_center + local) * view.scale).to_pos2();
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
