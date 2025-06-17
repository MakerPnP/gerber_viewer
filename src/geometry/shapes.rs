use nalgebra::Point2;

pub fn is_convex(vertices: &[Point2<f64>]) -> bool {
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
