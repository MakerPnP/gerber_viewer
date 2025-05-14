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
