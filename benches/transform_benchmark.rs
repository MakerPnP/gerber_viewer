use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gerber_viewer::Matrix3Point2Ext;
use nalgebra::{Matrix3, Point2, Vector3};
use rand::Rng;

// Implement a basic Transform struct for testing
pub struct TestTransform {
    // Add relevant fields to match your actual Transform struct
    scale_x: f64,
    scale_y: f64,
    rotation: f64,
    translation_x: f64,
    translation_y: f64,
}

impl TestTransform {
    fn new_random() -> Self {
        let mut rng = rand::thread_rng();
        Self {
            scale_x: rng.gen_range(0.5..2.0),
            scale_y: rng.gen_range(0.5..2.0),
            rotation: rng.gen_range(0.0..std::f64::consts::PI * 2.0),
            translation_x: rng.gen_range(-100.0..100.0),
            translation_y: rng.gen_range(-100.0..100.0),
        }
    }

    // Simplified version of apply_to_position for benchmarking
    #[inline]
    fn apply_to_position(&self, pos: Point2<f64>) -> Point2<f64> {
        // Apply scale
        let scaled_x = pos.x * self.scale_x;
        let scaled_y = pos.y * self.scale_y;

        // Apply rotation
        let cos_angle = self.rotation.cos();
        let sin_angle = self.rotation.sin();
        let rotated_x = scaled_x * cos_angle - scaled_y * sin_angle;
        let rotated_y = scaled_x * sin_angle + scaled_y * cos_angle;

        // Apply translation
        Point2::new(rotated_x + self.translation_x, rotated_y + self.translation_y)
    }

    // Version using matrix multiplication
    #[inline]
    fn apply_to_position_matrix(&self, position: Point2<f64>) -> Point2<f64> {
        // Convert to homogeneous coordinates
        let point_vec = Vector3::new(position.x, position.y, 1.0);

        // Apply the transformation matrix
        let matrix = self.to_matrix();
        let transformed = matrix * point_vec;

        // Convert back from homogeneous coordinates
        Point2::new(transformed[0], transformed[1])
    }

    // Create transformation matrix
    fn to_matrix(&self) -> Matrix3<f64> {
        let cos_angle = self.rotation.cos();
        let sin_angle = self.rotation.sin();

        Matrix3::new(
            self.scale_x * cos_angle,
            -self.scale_y * sin_angle,
            self.translation_x,
            self.scale_x * sin_angle,
            self.scale_y * cos_angle,
            self.translation_y,
            0.0,
            0.0,
            1.0,
        )
    }
}

fn generate_random_points(count: usize) -> Vec<Point2<f64>> {
    let mut rng = rand::thread_rng();
    (0..count)
        .map(|_| Point2::new(rng.gen_range(-1000.0..1000.0), rng.gen_range(-1000.0..1000.0)))
        .collect()
}

fn benchmark_transforms(c: &mut Criterion) {
    let num_points = 10000;
    let points = generate_random_points(num_points);
    let transform = TestTransform::new_random();
    let matrix = transform.to_matrix();

    let mut group = c.benchmark_group("Point Transformations");

    // Benchmark apply_to_position
    group.bench_function("apply_to_position", |b| {
        b.iter(|| {
            for point in &points {
                black_box(transform.apply_to_position(black_box(*point)));
            }
        })
    });

    // Benchmark apply_to_position_matrix
    group.bench_function("apply_to_position_matrix", |b| {
        b.iter(|| {
            for point in &points {
                black_box(transform.apply_to_position_matrix(black_box(*point)));
            }
        })
    });

    // Benchmark transform_point2
    group.bench_function("transform_point2", |b| {
        b.iter(|| {
            for point in &points {
                black_box(matrix.transform_point2(black_box(*point)));
            }
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_transforms);
criterion_main!(benches);
