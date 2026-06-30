//! Shared assertions for integration tests.

use fibonacci_sphere::SpherePoint;

pub fn assert_on_sphere(points: &[SpherePoint], radius: f64) {
    for p in points {
        let [x, y, z] = p.position;
        let dist = ((x * x + y * y + z * z) as f64).sqrt();
        assert!(
            (dist - radius).abs() < 1e-4,
            "point {} off sphere: dist={dist}, expected={radius}",
            p.index
        );
    }
}

pub fn assert_unique_positions(points: &[SpherePoint]) {
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            assert_ne!(
                points[i].position, points[j].position,
                "duplicate positions at indices {i} and {j}"
            );
        }
    }
}

pub fn position_distance(a: [f32; 3], b: [f32; 3]) -> f64 {
    let dx = (a[0] - b[0]) as f64;
    let dy = (a[1] - b[1]) as f64;
    let dz = (a[2] - b[2]) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
