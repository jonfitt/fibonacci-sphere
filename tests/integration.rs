use std::f64::consts::PI;

mod common;

use common::{assert_on_sphere, assert_unique_positions, position_distance};
use fibonacci_sphere::methods::DistributionMethod;
use fibonacci_sphere::point::GOLDEN_RATIO;
use fibonacci_sphere::SphereLattice;

#[test]
fn canonical_midpoint_golden_values() {
    let lattice = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 50, 1.0).unwrap();
    let p0 = &lattice.points()[0];

    let expected_theta = 0.0;
    let expected_y = (0.0 + 0.5) / 50.0;
    let expected_phi = (1.0_f64 - 2.0 * expected_y).acos();

    assert!((p0.theta - expected_theta).abs() < 1e-10);
    assert!((p0.phi - expected_phi).abs() < 1e-10);

    let i = 1.0;
    let p1 = &lattice.points()[1];
    let expected_theta_1 = 2.0 * PI * i / GOLDEN_RATIO;
    assert!((p1.theta - expected_theta_1).abs() < 1e-10);
}

#[test]
fn all_methods_produce_unique_points_at_n50() {
    for method in DistributionMethod::ALL {
        let lattice = SphereLattice::generate(method, 50, 1.0).unwrap();
        assert_unique_positions(lattice.points());
        assert_on_sphere(lattice.points(), 1.0);
    }
}

#[test]
fn offset_packing_improves_min_neighbor_vs_canonical_midpoint() {
    let canonical =
        SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 100, 1.0).unwrap();
    let offset = SphereLattice::generate(DistributionMethod::OffsetPacking, 100, 1.0).unwrap();

    let min_neighbor = |lattice: &SphereLattice| {
        let points = lattice.points();
        let mut min = f64::INFINITY;
        for (i, a) in points.iter().enumerate() {
            for (j, b) in points.iter().enumerate() {
                if i == j {
                    continue;
                }
                min = min.min(position_distance(a.position, b.position));
            }
        }
        min
    };

    assert!(
        min_neighbor(&offset) >= min_neighbor(&canonical),
        "offset packing should not reduce minimum neighbor distance"
    );
}

#[test]
fn lib_doc_example() {
    let lattice = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 100, 1.0).unwrap();
    assert_eq!(lattice.len(), 100);
    let flat = lattice.positions_flat();
    assert_eq!(flat.len(), 300);
}

#[test]
fn spherical_delaunay_includes_all_points() {
    let lattice = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 50, 1.0).unwrap();
    let edges = lattice.wireframe_edges();
    assert!(!edges.is_empty());
    let mut incident = vec![0usize; lattice.len()];
    for [a, b] in &edges {
        incident[*a] += 1;
        incident[*b] += 1;
    }
    assert!(incident.iter().all(|count| *count >= 3));
}

#[test]
fn spherical_delaunay_is_deterministic() {
    let lattice = SphereLattice::generate(DistributionMethod::OffsetPacking, 40, 1.0).unwrap();
    let first = lattice.wireframe_edges();
    let second = lattice.wireframe_edges();
    assert_eq!(first, second);
}
