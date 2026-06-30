use std::f64::consts::PI;

use crate::point::SpherePoint;

use super::Distribution;

/// Regular latitude–longitude grid (baseline for area-integration comparisons).
///
/// Equal-area colatitude rings with per-ring longitude samples. Strong polar clustering
/// versus Fibonacci methods (Gonzalez 2009). Details: [`super::info::LATITUDE_LONGITUDE`].
#[derive(Debug, Clone, Copy, Default)]
pub struct LatitudeLongitude;

impl Distribution for LatitudeLongitude {
    fn name(&self) -> &'static str {
        "latitude_longitude"
    }

    fn optimizes(&self) -> super::OptimizationGoal {
        super::OptimizationGoal::EqualArea
    }

    fn generate(&self, n: usize, radius: f64) -> Vec<SpherePoint> {
        if n == 0 {
            return Vec::new();
        }

        let n_f = n as f64;
        let rings = (n_f.sqrt().round() as usize).max(1);
        let mut points = Vec::with_capacity(n);
        let mut index = 0;

        for ring in 0..rings {
            if index >= n {
                break;
            }

            let v = if rings == 1 {
                0.5
            } else {
                (ring as f64 + 0.5) / rings as f64
            };
            let phi = (1.0 - 2.0 * v).clamp(-1.0, 1.0).acos();

            let remaining = n - index;
            let rings_left = rings - ring;
            let points_in_ring = remaining.div_ceil(rings_left);

            for j in 0..points_in_ring {
                if index >= n {
                    break;
                }
                let u = (j as f64 + 0.5) / points_in_ring as f64;
                let theta = 2.0 * PI * u;
                points.push(SpherePoint::from_spherical(index, theta, phi, radius));
                index += 1;
            }
        }

        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{assert_on_sphere, assert_sequential_indices};

    #[test]
    fn latlong_count() {
        let points = LatitudeLongitude.generate(100, 1.0);
        assert_eq!(points.len(), 100);
    }

    #[test]
    fn latlong_zero_returns_empty() {
        assert!(LatitudeLongitude.generate(0, 1.0).is_empty());
    }

    #[test]
    fn latlong_single_point() {
        let points = LatitudeLongitude.generate(1, 3.0);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].index, 0);
        assert_on_sphere(&points, 3.0);
    }

    #[test]
    fn latlong_on_sphere_for_various_counts() {
        for n in [1, 4, 10, 25, 100, 101] {
            let points = LatitudeLongitude.generate(n, 2.0);
            assert_eq!(points.len(), n);
            assert_on_sphere(&points, 2.0);
            assert_sequential_indices(&points);
        }
    }

    #[test]
    fn latlong_metadata() {
        assert_eq!(LatitudeLongitude.name(), "latitude_longitude");
        assert_eq!(
            LatitudeLongitude.optimizes(),
            super::super::OptimizationGoal::EqualArea
        );
    }
}
