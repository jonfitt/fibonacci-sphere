//! Geographic queries on sphere lattice vertices (poles and equator).

/// Angular distance from a point to the north pole `(0, 1, 0)` in radians.
pub fn angular_distance_to_north_pole(position: [f32; 3]) -> f64 {
    let unit = normalize(position);
    unit[1].clamp(-1.0, 1.0).acos()
}

/// Angular distance from a point to the south pole `(0, -1, 0)` in radians.
pub fn angular_distance_to_south_pole(position: [f32; 3]) -> f64 {
    let unit = normalize(position);
    (-unit[1]).clamp(-1.0, 1.0).acos()
}

/// Angular distance from a point to the equator in radians (`0` on the equator).
pub fn angular_distance_to_equator(position: [f32; 3]) -> f64 {
    let unit = normalize(position);
    unit[1].clamp(-1.0, 1.0).asin().abs()
}

/// Vertex indices whose angular distance to the north pole is at most `max_angle`.
pub fn vertices_within_north_polar_distance(positions: &[[f32; 3]], max_angle: f64) -> Vec<usize> {
    filter_by_angular_distance(positions, max_angle, angular_distance_to_north_pole)
}

/// Vertex indices whose angular distance to the south pole is at most `max_angle`.
pub fn vertices_within_south_polar_distance(positions: &[[f32; 3]], max_angle: f64) -> Vec<usize> {
    filter_by_angular_distance(positions, max_angle, angular_distance_to_south_pole)
}

/// Vertex indices whose angular distance to the equator is at most `max_angle`.
pub fn vertices_within_equatorial_distance(positions: &[[f32; 3]], max_angle: f64) -> Vec<usize> {
    filter_by_angular_distance(positions, max_angle, angular_distance_to_equator)
}

/// Line segments approximating a small circle at a fixed angular distance from a pole.
///
/// Returns `(start, end)` pairs forming a closed loop on the sphere surface.
/// `angular_distance` is the geodesic angle from the pole in radians.
/// `segment_count` must be at least 3.
pub fn polar_cap_circle_segments(
    south: bool,
    angular_distance: f64,
    sphere_radius: f32,
    segment_count: usize,
) -> Vec<([f32; 3], [f32; 3])> {
    if angular_distance <= 0.0 || segment_count < 3 {
        return Vec::new();
    }

    let angular_distance = angular_distance as f32;
    let sin_d = angular_distance.sin();
    let cos_d = angular_distance.cos();
    let pole_y = if south { -cos_d } else { cos_d };

    let mut vertices = Vec::with_capacity(segment_count);
    for index in 0..segment_count {
        let theta = (index as f32 / segment_count as f32) * std::f32::consts::TAU;
        let (sin_theta, cos_theta) = theta.sin_cos();
        vertices.push([
            sphere_radius * sin_d * cos_theta,
            sphere_radius * pole_y,
            sphere_radius * sin_d * sin_theta,
        ]);
    }

    let mut segments = Vec::with_capacity(segment_count);
    for index in 0..segment_count {
        let next = (index + 1) % segment_count;
        segments.push((vertices[index], vertices[next]));
    }
    segments
}

fn filter_by_angular_distance(
    positions: &[[f32; 3]],
    max_angle: f64,
    distance: fn([f32; 3]) -> f64,
) -> Vec<usize> {
    positions
        .iter()
        .enumerate()
        .filter_map(|(index, position)| {
            if distance(*position) <= max_angle + 1e-9 {
                Some(index)
            } else {
                None
            }
        })
        .collect()
}

fn normalize(v: [f32; 3]) -> [f64; 3] {
    let x = f64::from(v[0]);
    let y = f64::from(v[1]);
    let z = f64::from(v[2]);
    let len = (x * x + y * y + z * z).sqrt();
    if len <= f64::EPSILON {
        return [0.0, 1.0, 0.0];
    }
    [x / len, y / len, z / len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn north_pole_has_zero_distance_to_north() {
        assert!(angular_distance_to_north_pole([0.0, 1.0, 0.0]).abs() < 1e-9);
        assert!(
            (angular_distance_to_north_pole([0.0, 0.0, 1.0]) - std::f64::consts::FRAC_PI_2).abs()
                < 1e-6
        );
    }

    #[test]
    fn equator_distance_is_zero_on_x_axis() {
        assert!(angular_distance_to_equator([1.0, 0.0, 0.0]).abs() < 1e-9);
    }

    #[test]
    fn polar_cap_filter_includes_pole_site() {
        let positions = vec![[0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let north = vertices_within_north_polar_distance(&positions, 0.1);
        assert_eq!(north, vec![0]);
    }

    #[test]
    fn polar_cap_circle_segments_lie_on_sphere_at_expected_distance() {
        let radius = 2.0;
        let distance = 0.35;
        let segments = polar_cap_circle_segments(false, distance, radius, 72);
        assert_eq!(segments.len(), 72);

        for (start, end) in segments {
            for point in [start, end] {
                let len = f64::from(
                    (point[0] * point[0] + point[1] * point[1] + point[2] * point[2]).sqrt(),
                );
                assert!((len - f64::from(radius)).abs() < 1e-4);
                assert!((angular_distance_to_north_pole(point) - distance).abs() < 1e-4);
            }
        }

        let south_segments = polar_cap_circle_segments(true, distance, radius, 36);
        assert_eq!(south_segments.len(), 36);
        for (start, _) in south_segments {
            assert!((angular_distance_to_south_pole(start) - distance).abs() < 1e-4);
        }
    }
}
