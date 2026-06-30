//! Shortest paths on the spherical Delaunay mesh using geodesic edge weights.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
#[cfg(feature = "terrain")]
use std::collections::HashSet;

use crate::error::SphereError;
use crate::point::SpherePoint;

use super::spherical_delaunay_edges;

/// Shortest path along mesh edges between two lattice vertices.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfacePath {
    /// Vertex indices from start to end (inclusive).
    pub vertices: Vec<usize>,
    /// Total geodesic length along the path (same units as lattice radius).
    pub length: f64,
}

impl SurfacePath {
    /// World-space positions for each vertex on the path.
    pub fn positions(&self, points: &[SpherePoint]) -> Vec<[f32; 3]> {
        self.vertices
            .iter()
            .map(|&index| points[index].position)
            .collect()
    }

    /// Line segment endpoints for rendering the path (`[a, b, c, d, ...]`).
    pub fn segment_positions(&self, points: &[SpherePoint]) -> Vec<[f32; 3]> {
        let positions = self.positions(points);
        let mut segments = Vec::with_capacity(positions.len().saturating_sub(1) * 2);
        for window in positions.windows(2) {
            segments.push(window[0]);
            segments.push(window[1]);
        }
        segments
    }
}

/// Adjacency graph built from the spherical Delaunay wireframe.
#[derive(Debug, Clone)]
pub struct SurfaceGraph {
    /// Adjacency lists with `(neighbor_index, geodesic_edge_weight)`.
    pub(crate) adjacency: Vec<Vec<(usize, f64)>>,
}

impl SurfaceGraph {
    /// Build a surface graph from lattice sample positions and Delaunay edges.
    pub fn from_positions(positions: &[[f32; 3]]) -> Self {
        Self::from_edges(positions, &spherical_delaunay_edges(positions))
    }

    /// Build a surface graph from precomputed Delaunay edges.
    pub fn from_edges(positions: &[[f32; 3]], edges: &[[usize; 2]]) -> Self {
        let mut adjacency = vec![Vec::new(); positions.len()];

        for [a, b] in edges {
            let weight = geodesic_distance(positions[*a], positions[*b]);
            adjacency[*a].push((*b, weight));
            adjacency[*b].push((*a, weight));
        }

        Self { adjacency }
    }

    /// Build a surface graph from a precomputed adjacency list.
    pub fn from_adjacency(adjacency: Vec<Vec<(usize, f64)>>) -> Self {
        Self { adjacency }
    }

    /// Neighbors of `index` as `(neighbor_index, geodesic_edge_weight)` pairs.
    pub fn neighbors(&self, index: usize) -> &[(usize, f64)] {
        &self.adjacency[index]
    }

    /// Number of vertices in the graph.
    pub fn len(&self) -> usize {
        self.adjacency.len()
    }

    /// Returns true when the graph has no vertices.
    pub fn is_empty(&self) -> bool {
        self.adjacency.is_empty()
    }

    /// Shortest geodesic path between two vertex indices.
    pub fn shortest_path(&self, from: usize, to: usize) -> Result<SurfacePath, SphereError> {
        validate_vertex_index(from, self.len())?;
        validate_vertex_index(to, self.len())?;

        if from == to {
            return Ok(SurfacePath {
                vertices: vec![from],
                length: 0.0,
            });
        }

        let (vertices, length) = dijkstra_unfiltered(&self.adjacency, from, to)
            .ok_or(SphereError::NoSurfacePath { from, to })?;

        Ok(SurfacePath { vertices, length })
    }

    /// Shortest path visiting only vertices whose terrain type is allowed.
    ///
    /// When `allowed` is empty, all terrain types are permitted.
    #[cfg(feature = "terrain")]
    pub fn shortest_path_with_allowed_terrain(
        &self,
        from: usize,
        to: usize,
        terrain: &[crate::terrain::TerrainType],
        allowed: &[crate::terrain::TerrainType],
    ) -> Result<SurfacePath, SphereError> {
        validate_vertex_index(from, self.len())?;
        validate_vertex_index(to, self.len())?;

        if from == to {
            return Ok(SurfacePath {
                vertices: vec![from],
                length: 0.0,
            });
        }

        let filter_active = !allowed.is_empty() && !terrain.is_empty();
        if filter_active {
            let allowed_set: HashSet<_> = allowed.iter().copied().collect();
            if !vertex_terrain_allowed(from, terrain, &allowed_set)
                || !vertex_terrain_allowed(to, terrain, &allowed_set)
            {
                return Err(SphereError::NoSurfacePath { from, to });
            }

            let (vertices, length) =
                dijkstra_with_allowed(&self.adjacency, from, to, terrain, &allowed_set)
                    .ok_or(SphereError::NoSurfacePath { from, to })?;

            return Ok(SurfacePath { vertices, length });
        }

        let (vertices, length) = dijkstra_unfiltered(&self.adjacency, from, to)
            .ok_or(SphereError::NoSurfacePath { from, to })?;

        Ok(SurfacePath { vertices, length })
    }
}

/// Geodesic arc length between two points on a sphere (from their Cartesian positions).
pub fn geodesic_distance(a: [f32; 3], b: [f32; 3]) -> f64 {
    let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) as f64;
    let na = ((a[0] * a[0] + a[1] * a[1] + a[2] * a[2]) as f64).sqrt();
    let nb = ((b[0] * b[0] + b[1] * b[1] + b[2] * b[2]) as f64).sqrt();
    if na <= f64::EPSILON || nb <= f64::EPSILON {
        return 0.0;
    }
    let cos_angle = (dot / (na * nb)).clamp(-1.0, 1.0);
    na * cos_angle.acos()
}

/// Index of the lattice vertex closest to `query` (by angular distance on the sphere).
pub fn nearest_vertex_index(positions: &[[f32; 3]], query: [f32; 3]) -> Result<usize, SphereError> {
    if positions.is_empty() {
        return Err(SphereError::InvalidPointCount { n: 0 });
    }

    let nq = normalize(query);
    let mut best_index = 0;
    let mut best_angle = f64::INFINITY;

    for (index, position) in positions.iter().enumerate() {
        let np = normalize(*position);
        let cos_angle = (nq[0] * np[0] + nq[1] * np[1] + nq[2] * np[2]).clamp(-1.0, 1.0);
        let angle = cos_angle.acos();
        if angle < best_angle {
            best_angle = angle;
            best_index = index;
        }
    }

    Ok(best_index)
}

fn normalize(v: [f32; 3]) -> [f64; 3] {
    let len = ((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) as f64).sqrt();
    if len <= f64::EPSILON {
        return [0.0, 1.0, 0.0];
    }
    [v[0] as f64 / len, v[1] as f64 / len, v[2] as f64 / len]
}

fn validate_vertex_index(index: usize, count: usize) -> Result<(), SphereError> {
    if index >= count {
        return Err(SphereError::InvalidVertexIndex { index, count });
    }
    Ok(())
}

fn dijkstra_unfiltered(
    adjacency: &[Vec<(usize, f64)>],
    start: usize,
    goal: usize,
) -> Option<(Vec<usize>, f64)> {
    let n = adjacency.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut came_from: Vec<Option<usize>> = vec![None; n];
    let mut heap: BinaryHeap<(Reverse<u64>, usize)> = BinaryHeap::new();

    dist[start] = 0.0;
    heap.push((Reverse(0.0_f64.to_bits()), start));

    while let Some((Reverse(cost_bits), node)) = heap.pop() {
        let cost = f64::from_bits(cost_bits);
        if cost > dist[node] + 1e-9 {
            continue;
        }
        if node == goal {
            break;
        }

        for &(next, weight) in &adjacency[node] {
            let next_cost = cost + weight;
            if next_cost + 1e-9 < dist[next] {
                dist[next] = next_cost;
                came_from[next] = Some(node);
                heap.push((Reverse(next_cost.to_bits()), next));
            }
        }
    }

    if !dist[goal].is_finite() {
        return None;
    }

    Some(reconstruct_path(start, goal, &came_from, dist[goal]))
}

#[cfg(feature = "terrain")]
fn dijkstra_with_allowed(
    adjacency: &[Vec<(usize, f64)>],
    start: usize,
    goal: usize,
    terrain: &[crate::terrain::TerrainType],
    allowed: &HashSet<crate::terrain::TerrainType>,
) -> Option<(Vec<usize>, f64)> {
    let n = adjacency.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut came_from: Vec<Option<usize>> = vec![None; n];
    let mut heap: BinaryHeap<(Reverse<u64>, usize)> = BinaryHeap::new();

    dist[start] = 0.0;
    heap.push((Reverse(0.0_f64.to_bits()), start));

    while let Some((Reverse(cost_bits), node)) = heap.pop() {
        let cost = f64::from_bits(cost_bits);
        if cost > dist[node] + 1e-9 {
            continue;
        }
        if node == goal {
            break;
        }

        if !vertex_terrain_allowed(node, terrain, allowed) {
            continue;
        }

        for &(next, weight) in &adjacency[node] {
            if !vertex_terrain_allowed(next, terrain, allowed) {
                continue;
            }
            let next_cost = cost + weight;
            if next_cost + 1e-9 < dist[next] {
                dist[next] = next_cost;
                came_from[next] = Some(node);
                heap.push((Reverse(next_cost.to_bits()), next));
            }
        }
    }

    if !dist[goal].is_finite() {
        return None;
    }

    Some(reconstruct_path(start, goal, &came_from, dist[goal]))
}

fn reconstruct_path(
    start: usize,
    goal: usize,
    came_from: &[Option<usize>],
    length: f64,
) -> (Vec<usize>, f64) {
    let mut vertices = Vec::new();
    let mut current = goal;
    loop {
        vertices.push(current);
        if current == start {
            break;
        }
        current = came_from[current].expect("valid path must have predecessors");
    }
    vertices.reverse();
    (vertices, length)
}

#[cfg(feature = "terrain")]
fn vertex_terrain_allowed(
    index: usize,
    terrain: &[crate::terrain::TerrainType],
    allowed: &HashSet<crate::terrain::TerrainType>,
) -> bool {
    allowed.contains(&terrain[index])
}

#[cfg(test)]
mod tests {
    use crate::SphereLattice;
    use crate::methods::DistributionMethod;

    use super::*;

    #[test]
    fn geodesic_distance_on_equator_quarter_sphere() {
        let dist = geodesic_distance([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        assert!((dist - std::f64::consts::FRAC_PI_2).abs() < 1e-5);
    }

    #[test]
    fn trivial_path_has_zero_length() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 20, 1.0).unwrap();
        let graph = SurfaceGraph::from_positions(&lattice.position_arrays());
        let path = graph.shortest_path(3, 3).unwrap();
        assert_eq!(path.vertices, vec![3]);
        assert!(path.length.abs() < 1e-9);
    }

    #[test]
    fn adjacent_vertices_form_length_one_path() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 30, 2.0).unwrap();
        let positions = lattice.position_arrays();
        let graph = SurfaceGraph::from_positions(&positions);
        let edges = spherical_delaunay_edges(&positions);
        let [a, b] = edges[0];

        let path = graph.shortest_path(a, b).unwrap();
        assert_eq!(path.vertices, vec![a, b]);
        let expected = geodesic_distance(positions[a], positions[b]);
        assert!((path.length - expected).abs() < 1e-5);
    }

    #[test]
    fn delaunay_graph_is_connected_for_midpoint_lattice() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 60, 1.0).unwrap();
        let graph = SurfaceGraph::from_positions(&lattice.position_arrays());
        assert!(is_connected(&graph));

        for from in 0..lattice.len() {
            for to in 0..lattice.len() {
                assert!(
                    graph.shortest_path(from, to).is_ok(),
                    "no path from {from} to {to}"
                );
            }
        }
    }

    #[test]
    fn path_length_is_sum_of_edge_geodesics() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 50, 1.5).unwrap();
        let positions = lattice.position_arrays();
        let graph = SurfaceGraph::from_positions(&positions);
        let path = graph.shortest_path(0, 17).unwrap();

        let mut expected = 0.0;
        for window in path.vertices.windows(2) {
            expected += geodesic_distance(positions[window[0]], positions[window[1]]);
        }
        assert!((path.length - expected).abs() < 1e-4);
    }

    #[test]
    fn nearest_vertex_finds_exact_lattice_point() {
        let lattice = SphereLattice::generate(DistributionMethod::Canonical, 10, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let index = nearest_vertex_index(&positions, positions[4]).unwrap();
        assert_eq!(index, 4);
    }

    #[test]
    fn invalid_vertex_index_is_rejected() {
        let lattice = SphereLattice::generate(DistributionMethod::Canonical, 5, 1.0).unwrap();
        let graph = SurfaceGraph::from_positions(&lattice.position_arrays());
        assert_eq!(
            graph.shortest_path(0, 10),
            Err(SphereError::InvalidVertexIndex {
                index: 10,
                count: 5
            })
        );
    }

    #[cfg(feature = "terrain")]
    #[test]
    fn terrain_filtered_path_rejects_disallowed_endpoints() {
        use crate::terrain::{PerlinNoiseConfig, TerrainType};
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 40, 1.0).unwrap();
        let terrain = lattice.generate_terrain(
            PerlinNoiseConfig {
                seed: Some(7),
                ..Default::default()
            },
            &mut StdRng::seed_from_u64(7),
        );
        let graph = lattice.surface_graph();
        let mountain_only = [TerrainType::Mountain];

        let mountain_index = (0..lattice.len())
            .find(|&index| terrain.get(index) == TerrainType::Mountain)
            .expect("expected at least one mountain vertex");
        let land_index = (0..lattice.len())
            .find(|&index| terrain.get(index) == TerrainType::Land)
            .expect("expected at least one land vertex");

        assert!(
            graph
                .shortest_path_with_allowed_terrain(
                    mountain_index,
                    land_index,
                    terrain.as_slice(),
                    &mountain_only
                )
                .is_err()
        );
    }

    #[cfg(feature = "terrain")]
    #[test]
    fn empty_allowed_terrain_list_matches_unfiltered_path() {
        use crate::terrain::PerlinNoiseConfig;
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 35, 1.0).unwrap();
        let terrain = lattice.generate_terrain(
            PerlinNoiseConfig {
                seed: Some(3),
                ..Default::default()
            },
            &mut StdRng::seed_from_u64(3),
        );
        let graph = lattice.surface_graph();
        let unfiltered = graph.shortest_path(2, 18).unwrap();
        let filtered = graph
            .shortest_path_with_allowed_terrain(2, 18, terrain.as_slice(), &[])
            .unwrap();
        assert_eq!(unfiltered.vertices, filtered.vertices);
    }

    fn is_connected(graph: &SurfaceGraph) -> bool {
        if graph.is_empty() {
            return true;
        }

        let mut visited = vec![false; graph.len()];
        let mut stack = vec![0];
        visited[0] = true;
        let mut seen = 1usize;

        while let Some(node) = stack.pop() {
            for (next, _) in graph.adjacency[node].iter().copied() {
                if !visited[next] {
                    visited[next] = true;
                    seen += 1;
                    stack.push(next);
                }
            }
        }

        seen == graph.len()
    }
}
