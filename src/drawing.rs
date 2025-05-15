use egui::{Color32, Painter, Pos2, Shape, Stroke};

pub fn draw_crosshair(painter: &Painter, position: Pos2, color: Color32) {
    // Calculate viewport bounds to extend lines across entire view
    let viewport = painter.clip_rect();

    // Draw a horizontal line (extending across viewport)
    painter.line_segment(
        [
            Pos2::new(viewport.min.x, position.y),
            Pos2::new(viewport.max.x, position.y),
        ],
        Stroke::new(1.0, color),
    );

    // Draw a vertical line (extending across viewport)
    painter.line_segment(
        [
            Pos2::new(position.x, viewport.min.y),
            Pos2::new(position.x, viewport.max.y),
        ],
        Stroke::new(1.0, color),
    );
}

pub fn draw_arrow(painter: &Painter, start: Pos2, end: Pos2, color: Color32) {
    painter.line_segment([start, end], Stroke::new(1.0, color));
}

pub fn draw_outline(painter: &Painter, vertices: Vec<Pos2>, color: Color32) {
    painter.add(Shape::closed_line(vertices, Stroke::new(1.0, color)));
}

pub fn draw_marker(painter: &Painter, position: Pos2, color1: Color32, color2: Color32, radius: f32) {
    let start1 = Pos2::new(position.x - radius, position.y - 0.0);
    let end1 = Pos2::new(position.x + radius, position.y - 0.0);
    let start2 = Pos2::new(position.x + 0.0, position.y - radius);
    let end2 = Pos2::new(position.x + 0.0, position.y + radius);

    painter.line_segment([start1, end1], Stroke::new(1.0, color1));

    painter.line_segment([start2, end2], Stroke::new(1.0, color1));

    painter.circle(position, radius * 0.25, Color32::TRANSPARENT, Stroke::new(1.0, color2));
}
