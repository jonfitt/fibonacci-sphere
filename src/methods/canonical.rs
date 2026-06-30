use std::f64::consts::PI;

use crate::point::{GOLDEN_RATIO, SpherePoint};

use super::Distribution;

/// Original Fibonacci lattice with `y = i / n` (no midpoint offset).
///
/// Baseline golden-angle spiral; first sample at the north pole. Prefer
/// [`super::CanonicalMidpoint`] or [`super::OffsetPacking`] for better packing.
/// Details: [`super::info::CANONICAL`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Canonical;

impl Distribution for Canonical {
    fn name(&self) -> &'static str {
        "canonical"
    }

    fn optimizes(&self) -> super::OptimizationGoal {
        super::OptimizationGoal::Baseline
    }

    fn generate(&self, n: usize, radius: f64) -> Vec<SpherePoint> {
        fibonacci_lattice(n, radius, 0.0, n as f64, false)
    }
}

/// Canonical lattice with midpoint rule `(i + 0.5) / n` for better packing distance.
///
/// Default general-purpose Fibonacci sphere; avoids a pole sample at index 0.
/// Details: [`super::info::CANONICAL_MIDPOINT`].
#[derive(Debug, Clone, Copy, Default)]
pub struct CanonicalMidpoint;

impl Distribution for CanonicalMidpoint {
    fn name(&self) -> &'static str {
        "canonical_midpoint"
    }

    fn optimizes(&self) -> super::OptimizationGoal {
        super::OptimizationGoal::PackingDistance
    }

    fn generate(&self, n: usize, radius: f64) -> Vec<SpherePoint> {
        fibonacci_lattice(n, radius, 0.5, n as f64, false)
    }
}

/// Shared Fibonacci lattice generator.
///
/// `y_offset` and `y_scale` control the vertical lattice coordinate before the
/// equal-area spherical mapping: `y = (i + y_offset) / y_scale`.
pub(crate) fn fibonacci_lattice(
    n: usize,
    radius: f64,
    y_offset: f64,
    y_scale: f64,
    use_fractional_theta: bool,
) -> Vec<SpherePoint> {
    if n == 0 {
        return Vec::new();
    }

    (0..n)
        .map(|i| {
            let i_f = i as f64;
            let theta = if use_fractional_theta {
                let x = i_f / GOLDEN_RATIO;
                2.0 * PI * (x - x.floor())
            } else {
                2.0 * PI * i_f / GOLDEN_RATIO
            };
            let y = (i_f + y_offset) / y_scale;
            let phi = (1.0 - 2.0 * y).clamp(-1.0, 1.0).acos();
            SpherePoint::from_spherical(i, theta, phi, radius)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        assert_on_sphere, assert_sequential_indices, assert_unique_positions,
    };

    #[test]
    fn canonical_midpoint_count() {
        let points = CanonicalMidpoint.generate(50, 1.0);
        assert_eq!(points.len(), 50);
    }

    #[test]
    fn zero_points_returns_empty() {
        assert!(Canonical.generate(0, 1.0).is_empty());
        assert!(CanonicalMidpoint.generate(0, 1.0).is_empty());
        assert!(fibonacci_lattice(0, 1.0, 0.5, 1.0, false).is_empty());
    }

    #[test]
    fn single_point_is_north_pole_when_y_is_zero() {
        let p = Canonical.generate(1, 2.0)[0];
        assert!((p.position[1] - 2.0).abs() < 1e-4);
    }

    #[test]
    fn canonical_differs_from_midpoint() {
        let canonical = Canonical.generate(10, 1.0);
        let midpoint = CanonicalMidpoint.generate(10, 1.0);
        assert_ne!(canonical[0].phi, midpoint[0].phi);
        assert_ne!(canonical[5].position, midpoint[5].position);
    }

    #[test]
    fn canonical_first_point_phi_is_acos_one() {
        let p = Canonical.generate(10, 1.0)[0];
        assert!((p.phi - 0.0).abs() < 1e-10);
    }

    #[test]
    fn midpoint_first_point_uses_half_offset() {
        let n = 50;
        let p = CanonicalMidpoint.generate(n, 1.0)[0];
        let expected_phi = (1.0 - 2.0 * 0.5 / n as f64).acos();
        assert!((p.phi - expected_phi).abs() < 1e-10);
    }

    #[test]
    fn fractional_theta_wraps_into_range() {
        let points = fibonacci_lattice(100, 1.0, 0.5, 100.0, true);
        for p in &points {
            assert!(p.theta >= 0.0);
            assert!(p.theta < 2.0 * PI);
        }
    }

    #[test]
    fn distribution_metadata() {
        assert_eq!(Canonical.name(), "canonical");
        assert_eq!(CanonicalMidpoint.name(), "canonical_midpoint");
        assert_eq!(
            Canonical.optimizes(),
            super::super::OptimizationGoal::Baseline
        );
        assert_eq!(
            CanonicalMidpoint.optimizes(),
            super::super::OptimizationGoal::PackingDistance
        );
    }

    #[test]
    fn on_sphere_and_unique_for_both_variants() {
        for n in [2, 10, 50] {
            for points in [
                Canonical.generate(n, 1.5),
                CanonicalMidpoint.generate(n, 1.5),
            ] {
                assert_eq!(points.len(), n);
                assert_on_sphere(&points, 1.5);
                assert_sequential_indices(&points);
                assert_unique_positions(&points);
            }
        }
    }
}
