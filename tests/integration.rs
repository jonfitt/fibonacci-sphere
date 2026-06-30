//! Smoke tests for the public `SphereLattice` routing API (Godot/GDExtension path).

use fibonacci_sphere::{DistributionMethod, SphereLattice};

#[test]
fn lattice_routing_facade_matches_vertex_positions() {
    let lattice = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 48, 1.0).unwrap();

    let start_index = 3;
    let goal_index = 27;
    let start_pos = lattice.points()[start_index].position;
    let goal_pos = lattice.points()[goal_index].position;

    assert_eq!(
        lattice.nearest_vertex_index(start_pos).unwrap(),
        start_index
    );
    assert_eq!(lattice.nearest_vertex_index(goal_pos).unwrap(), goal_index);

    let path = lattice
        .shortest_surface_path(start_index, goal_index)
        .unwrap();
    let positions = lattice
        .shortest_surface_path_positions(start_index, goal_index)
        .unwrap();

    assert_eq!(positions.len(), path.vertices.len());
    assert_eq!(positions.first().copied(), Some(start_pos));
    assert_eq!(positions.last().copied(), Some(goal_pos));
}
