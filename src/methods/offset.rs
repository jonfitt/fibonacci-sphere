use std::f64::consts::PI;

use crate::methods::epsilon::{self, AVERAGE_NEIGHBOR_EPSILON};
use crate::point::SpherePoint;
use crate::point::GOLDEN_RATIO;

use super::Distribution;

/// Offset Fibonacci lattice optimized for minimum nearest-neighbor distance.
///
/// Roberts (2018) tiered ε offsets tighten δ_min by pulling samples away from crowded
/// polar caps. Details: [`super::info::OFFSET_PACKING`].
#[derive(Debug, Clone, Copy, Default)]
pub struct OffsetPacking;

impl Distribution for OffsetPacking {
    fn name(&self) -> &'static str {
        "offset_packing"
    }

    fn optimizes(&self) -> super::OptimizationGoal {
        super::OptimizationGoal::PackingDistance
    }

    fn generate(&self, n: usize, radius: f64) -> Vec<SpherePoint> {
        let epsilon = epsilon::packing_epsilon(n);
        offset_lattice(n, radius, epsilon, false)
    }
}

/// Offset lattice with explicit north/south pole points.
///
/// Reserves two samples at ±Y and offsets the remaining `n − 2` points (Roberts pole
/// lattice). Details: [`super::info::OFFSET_PACKING_WITH_POLES`].
#[derive(Debug, Clone, Copy, Default)]
pub struct OffsetPackingWithPoles;

impl Distribution for OffsetPackingWithPoles {
    fn name(&self) -> &'static str {
        "offset_packing_with_poles"
    }

    fn optimizes(&self) -> super::OptimizationGoal {
        super::OptimizationGoal::PackingDistance
    }

    fn generate(&self, n: usize, radius: f64) -> Vec<SpherePoint> {
        if n == 0 {
            return Vec::new();
        }
        if n == 1 {
            return vec![SpherePoint::from_spherical(0, 0.0, 0.0, radius)];
        }
        if n == 2 {
            return vec![
                SpherePoint::from_spherical(0, 0.0, 0.0, radius),
                SpherePoint::from_spherical(1, 0.0, PI, radius),
            ];
        }

        let epsilon = epsilon::packing_with_poles_epsilon(n);
        let inner_count = n - 2;
        let mut points = Vec::with_capacity(n);

        points.push(SpherePoint::from_spherical(0, 0.0, 0.0, radius));

        let inner = offset_lattice(inner_count, radius, epsilon, false);
        for (offset, mut p) in inner.into_iter().enumerate() {
            p.index = offset + 1;
            points.push(p);
        }

        points.push(SpherePoint::from_spherical(n - 1, 0.0, PI, radius));
        points
    }
}

/// Offset lattice with fixed ε ≈ 0.36 for average nearest-neighbor distance.
///
/// Trades peak packing for more uniform local spacing (Roberts / Baskerville).
/// Details: [`super::info::OFFSET_AVERAGE_NEIGHBOR`].
#[derive(Debug, Clone, Copy, Default)]
pub struct OffsetAverageNeighbor;

impl Distribution for OffsetAverageNeighbor {
    fn name(&self) -> &'static str {
        "offset_average_neighbor"
    }

    fn optimizes(&self) -> super::OptimizationGoal {
        super::OptimizationGoal::AverageNeighborDistance
    }

    fn generate(&self, n: usize, radius: f64) -> Vec<SpherePoint> {
        offset_lattice(n, radius, AVERAGE_NEIGHBOR_EPSILON, false)
    }
}

fn offset_lattice(
    n: usize,
    radius: f64,
    epsilon: f64,
    use_fractional_theta: bool,
) -> Vec<SpherePoint> {
    if n == 0 {
        return Vec::new();
    }

    let denominator = (n as f64 - 1.0) + 2.0 * epsilon;

    (0..n)
        .map(|i| {
            let i_f = i as f64;
            let theta = if use_fractional_theta {
                let x = (i_f + epsilon) / denominator;
                2.0 * PI * (x - x.floor())
            } else {
                2.0 * PI * i_f / GOLDEN_RATIO
            };
            let y = (i_f + epsilon) / denominator;
            let phi = (1.0 - 2.0 * y).clamp(-1.0, 1.0).acos();
            SpherePoint::from_spherical(i, theta, phi, radius)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{assert_on_sphere, assert_sequential_indices, assert_unique_positions};

    #[test]
    fn offset_packing_uses_lookup_epsilon() {
        let n = 50;
        let epsilon = epsilon::packing_epsilon(n);
        let points = OffsetPacking.generate(n, 1.0);
        assert_eq!(points.len(), n);

        let i = 0.0;
        let denominator = (n as f64 - 1.0) + 2.0 * epsilon;
        let expected_phi = (1.0 - 2.0 * (i + epsilon) / denominator).acos();
        assert!((points[0].phi - expected_phi).abs() < 1e-10);
    }

    #[test]
    fn offset_average_neighbor_uses_fixed_epsilon() {
        let points = OffsetAverageNeighbor.generate(20, 1.0);
        assert_eq!(points.len(), 20);
        let denominator = 19.0 + 2.0 * AVERAGE_NEIGHBOR_EPSILON;
        let expected_phi = (1.0_f64 - 2.0 * AVERAGE_NEIGHBOR_EPSILON / denominator).acos();
        assert!((points[0].phi - expected_phi).abs() < 1e-10);
    }

    #[test]
    fn poles_places_north_and_south() {
        let points = OffsetPackingWithPoles.generate(10, 2.0);
        assert_eq!(points.len(), 10);
        assert!((points[0].position[1] - 2.0).abs() < 1e-4);
        assert!((points[9].position[1] + 2.0).abs() < 1e-4);
        assert_sequential_indices(&points);
    }

    #[test]
    fn poles_small_n_edge_cases() {
        let one = OffsetPackingWithPoles.generate(1, 1.0);
        assert_eq!(one.len(), 1);
        assert!((one[0].position[1] - 1.0).abs() < 1e-4);

        let two = OffsetPackingWithPoles.generate(2, 1.0);
        assert_eq!(two.len(), 2);
        assert!((two[0].position[1] - 1.0).abs() < 1e-4);
        assert!((two[1].position[1] + 1.0).abs() < 1e-4);
    }

    #[test]
    fn poles_zero_returns_empty() {
        assert!(OffsetPackingWithPoles.generate(0, 1.0).is_empty());
    }

    #[test]
    fn all_offset_variants_on_sphere_and_unique() {
        for n in [4, 25, 100] {
            for points in [
                OffsetPacking.generate(n, 1.0),
                OffsetAverageNeighbor.generate(n, 1.0),
                OffsetPackingWithPoles.generate(n, 1.0),
            ] {
                assert_eq!(points.len(), n);
                assert_on_sphere(&points, 1.0);
                assert_unique_positions(&points);
            }
        }
    }

    #[test]
    fn offset_lattice_empty_for_zero_n() {
        assert!(offset_lattice(0, 1.0, 0.33, false).is_empty());
    }

    #[test]
    fn fractional_theta_differs_from_linear() {
        let linear = offset_lattice(10, 1.0, 0.36, false);
        let fractional = offset_lattice(10, 1.0, 0.36, true);
        assert_ne!(linear[3].theta, fractional[3].theta);
    }
}
