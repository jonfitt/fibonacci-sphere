//! Shared test assertions for unit tests.

use crate::point::SpherePoint;

/// Assert every point lies on a sphere of the given radius.
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

/// Assert indices run from 0 to len - 1 without gaps.
pub fn assert_sequential_indices(points: &[SpherePoint]) {
    for (expected, p) in points.iter().enumerate() {
        assert_eq!(
            p.index, expected,
            "non-sequential index at position {expected}"
        );
    }
}

/// Assert no two points share the same position.
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

/// Euclidean distance between two f32 positions.
pub fn position_distance(a: [f32; 3], b: [f32; 3]) -> f64 {
    let dx = (a[0] - b[0]) as f64;
    let dy = (a[1] - b[1]) as f64;
    let dz = (a[2] - b[2]) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_distance_is_zero_for_identical_points() {
        assert!(position_distance([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]).abs() < 1e-12);
    }

    #[test]
    fn assert_on_sphere_passes_for_unit_y() {
        let p = SpherePoint {
            index: 0,
            position: [0.0, 1.0, 0.0],
            theta: 0.0,
            phi: 0.0,
        };
        assert_on_sphere(&[p], 1.0);
    }
}
