//! Spherical Delaunay triangulation for points on a sphere.
//!
//! Uses stereographic projection from the south pole, a planar Delaunay triangulation,
//! and fan stitching at the projection singularity. This is the standard construction
//! described in computational geometry references and popularized for sphere meshing by
//! Red Blob Games (2018) following O'Rourke et al. / de Berg et al.
//!
//! See also Caroli et al., *Robust and Efficient Delaunay Triangulations of Points
//! on or Close to a Sphere* (INRIA RR-7004, 2009) and Renka's STRIPACK (Algorithm 772).

use super::delaunay2d;

const POLE_EPS: f64 = 1e-5;

/// Spherical Delaunay mesh: triangle faces and undirected wireframe edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SphericalMesh {
    /// Triangle faces as triples of vertex indices.
    pub triangles: Vec<[usize; 3]>,
    /// Undirected edges derived from the triangulation.
    pub edges: Vec<[usize; 2]>,
}

impl SphericalMesh {
    /// Build a mesh from precomputed triangles and edges.
    pub fn new(triangles: Vec<[usize; 3]>, edges: Vec<[usize; 2]>) -> Self {
        Self { triangles, edges }
    }
}

/// Full spherical Delaunay mesh (triangles + edges) for unit-sphere samples.
pub fn spherical_delaunay_mesh(positions: &[[f32; 3]]) -> SphericalMesh {
    let n = positions.len();
    if n < 2 {
        return SphericalMesh {
            triangles: Vec::new(),
            edges: fallback_edges(n),
        };
    }
    if n == 2 {
        return SphericalMesh {
            triangles: Vec::new(),
            edges: vec![[0, 1]],
        };
    }

    let unit: Vec<[f64; 3]> = positions.iter().map(normalize).collect();
    let south_index = unit
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a[1].partial_cmp(&b[1]).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(index, _)| index)
        .unwrap_or(0);

    // Stereographic projection is singular at the south pole. Rotate the sample set so
    // the southernmost site sits at `(0, -1, 0)` before projecting; rotation preserves
    // spherical Delaunay topology on the original coordinates.
    let mut aligned = unit.clone();
    align_site_to_south_pole(&mut aligned, south_index);
    let south_is_singular = aligned[south_index][1] <= -1.0 + POLE_EPS;

    let mut index_map = Vec::with_capacity(n);
    let mut projected = Vec::with_capacity(n);

    for (index, point) in aligned.iter().enumerate() {
        if south_is_singular && index == south_index {
            continue;
        }
        if let Some(uv) = stereographic_from_south(*point) {
            index_map.push(index);
            projected.push(uv);
        }
    }

    if projected.len() < 3 {
        return SphericalMesh {
            triangles: Vec::new(),
            edges: fallback_edges(n),
        };
    }

    let planar_triangles = delaunay2d::triangulate(&projected);
    let mut triangles = map_triangles(&planar_triangles, &index_map);
    let mut edges = undirected_edges_from_triangles(&planar_triangles, &index_map);

    if south_is_singular {
        let hull = delaunay2d::convex_hull(&projected);
        for window in hull.windows(2) {
            let a = index_map[window[0]];
            let b = index_map[window[1]];
            insert_edge(&mut edges, south_index, a);
            insert_edge(&mut edges, south_index, b);
            push_triangle(&mut triangles, [south_index, a, b]);
        }
        if hull.len() >= 2 {
            let first = index_map[hull[0]];
            let last = index_map[hull[hull.len() - 1]];
            push_triangle(&mut triangles, [south_index, last, first]);
        }
    }

    SphericalMesh { triangles, edges }
}

/// Triangle faces of the spherical Delaunay triangulation.
pub fn spherical_delaunay_triangles(positions: &[[f32; 3]]) -> Vec<[usize; 3]> {
    spherical_delaunay_mesh(positions).triangles
}

/// Undirected edges of the spherical Delaunay triangulation of unit-sphere samples.
///
/// Every input point appears in the graph. Typical vertex degree on a uniform sphere
/// sampling is about six, unlike directed k-nearest-neighbor graphs where degree varies
/// and edges may follow Euclidean chords through the sphere interior.
pub fn spherical_delaunay_edges(positions: &[[f32; 3]]) -> Vec<[usize; 2]> {
    spherical_delaunay_mesh(positions).edges
}

/// Stereographic projection from the south pole `(0, -1, 0)` onto the `y = 0` plane.
fn stereographic_from_south(point: [f64; 3]) -> Option<(f64, f64)> {
    let denom = 1.0 + point[1];
    if denom <= POLE_EPS {
        return None;
    }
    Some((point[0] / denom, point[2] / denom))
}

/// Rotate unit vectors so `positions[pole_index]` maps exactly to the south pole.
fn align_site_to_south_pole(positions: &mut [[f64; 3]], pole_index: usize) {
    let from = positions[pole_index];
    let to = [0.0, -1.0, 0.0];
    let alignment = rotation_from_to(from, to);

    for (index, point) in positions.iter_mut().enumerate() {
        *point = apply_rotation(alignment, *point);
        if index == pole_index {
            *point = to;
        }
    }
}

#[derive(Clone, Copy)]
enum AlignmentRotation {
    Identity,
    HalfTurn,
    AxisAngle { axis: [f64; 3], angle: f64 },
}

fn rotation_from_to(from: [f64; 3], to: [f64; 3]) -> AlignmentRotation {
    let dot = dot3(from, to).clamp(-1.0, 1.0);
    if dot >= 1.0 - 1e-12 {
        return AlignmentRotation::Identity;
    }
    if dot <= -1.0 + 1e-12 {
        return AlignmentRotation::HalfTurn;
    }

    let mut axis = cross3(from, to);
    let axis_len = length3(axis);
    if axis_len <= 1e-12 {
        return AlignmentRotation::Identity;
    }
    axis = scale3(axis, 1.0 / axis_len);
    AlignmentRotation::AxisAngle {
        axis,
        angle: dot.acos(),
    }
}

fn apply_rotation(rotation: AlignmentRotation, vector: [f64; 3]) -> [f64; 3] {
    match rotation {
        AlignmentRotation::Identity => vector,
        AlignmentRotation::HalfTurn => [-vector[0], -vector[1], -vector[2]],
        AlignmentRotation::AxisAngle { axis, angle } => rodrigues_rotate(vector, axis, angle),
    }
}

fn rodrigues_rotate(vector: [f64; 3], axis: [f64; 3], angle: f64) -> [f64; 3] {
    let (sin, cos) = angle.sin_cos();
    let cross = cross3(axis, vector);
    let scaled = scale3(axis, dot3(axis, vector) * (1.0 - cos));
    add3(add3(scale3(vector, cos), scale3(cross, sin)), scaled)
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn scale3(v: [f64; 3], factor: f64) -> [f64; 3] {
    [v[0] * factor, v[1] * factor, v[2] * factor]
}

fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn normalize(position: &[f32; 3]) -> [f64; 3] {
    let x = f64::from(position[0]);
    let y = f64::from(position[1]);
    let z = f64::from(position[2]);
    let len = (x * x + y * y + z * z).sqrt();
    if len <= f64::EPSILON {
        return [0.0, 1.0, 0.0];
    }
    [x / len, y / len, z / len]
}

fn map_triangles(triangles: &[[usize; 3]], index_map: &[usize]) -> Vec<[usize; 3]> {
    triangles
        .iter()
        .map(|triangle| {
            [
                index_map[triangle[0]],
                index_map[triangle[1]],
                index_map[triangle[2]],
            ]
        })
        .collect()
}

fn push_triangle(triangles: &mut Vec<[usize; 3]>, face: [usize; 3]) {
    let mut sorted = face;
    sorted.sort_unstable();
    if !triangles.iter().any(|existing| {
        let mut other = *existing;
        other.sort_unstable();
        other == sorted
    }) {
        triangles.push(face);
    }
}

fn undirected_edges_from_triangles(
    triangles: &[[usize; 3]],
    index_map: &[usize],
) -> Vec<[usize; 2]> {
    let mut seen = std::collections::BTreeSet::new();
    let mut edges = Vec::new();
    for triangle in triangles {
        for (local_a, local_b) in [
            (triangle[0], triangle[1]),
            (triangle[1], triangle[2]),
            (triangle[2], triangle[0]),
        ] {
            let a = index_map[local_a];
            let b = index_map[local_b];
            let key = normalized_pair(a, b);
            if seen.insert(key) {
                edges.push([key.0, key.1]);
            }
        }
    }
    edges
}

fn insert_edge(edges: &mut Vec<[usize; 2]>, a: usize, b: usize) {
    if a == b {
        return;
    }
    let key = normalized_pair(a, b);
    if edges.iter().all(|edge| normalized_pair(edge[0], edge[1]) != key) {
        edges.push([key.0, key.1]);
    }
}

fn normalized_pair(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

fn fallback_edges(n: usize) -> Vec<[usize; 2]> {
    (0..n.saturating_sub(1))
        .map(|index| [index, index + 1])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::DistributionMethod;
    use crate::SphereLattice;

    #[test]
    fn mesh_triangles_cover_edges() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 80, 1.0).unwrap();
        let positions: Vec<[f32; 3]> = lattice.points().iter().map(|p| p.position).collect();
        let mesh = spherical_delaunay_mesh(&positions);
        assert!(!mesh.triangles.is_empty());
        assert!(!mesh.edges.is_empty());

        let mut edge_set = std::collections::BTreeSet::new();
        for [a, b, c] in &mesh.triangles {
            edge_set.insert(normalized_pair(*a, *b));
            edge_set.insert(normalized_pair(*b, *c));
            edge_set.insert(normalized_pair(*c, *a));
        }
        for edge in &mesh.edges {
            assert!(edge_set.contains(&normalized_pair(edge[0], edge[1])));
        }
    }

    #[test]
    fn all_lattice_points_participate() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 60, 1.0).unwrap();
        let positions: Vec<[f32; 3]> = lattice.points().iter().map(|p| p.position).collect();
        let edges = spherical_delaunay_edges(&positions);
        assert!(!edges.is_empty());

        let mut incident = vec![0usize; positions.len()];
        for [a, b] in &edges {
            incident[*a] += 1;
            incident[*b] += 1;
        }
        assert!(incident.iter().all(|count| *count >= 3));
    }

    #[test]
    fn average_degree_is_about_six_for_uniform_sampling() {
        let lattice =
            SphereLattice::generate(DistributionMethod::OffsetPacking, 200, 1.0).unwrap();
        let positions: Vec<[f32; 3]> = lattice.points().iter().map(|p| p.position).collect();
        let edges = spherical_delaunay_edges(&positions);
        let edge_count = edges.len();
        let avg_degree = (2 * edge_count) as f64 / positions.len() as f64;
        assert!(
            (5.0..=7.5).contains(&avg_degree),
            "average degree {avg_degree} outside expected range"
        );
    }

    #[test]
    fn southernmost_site_gets_hub_triangulation_without_explicit_pole() {
        for &method in &[
            DistributionMethod::CanonicalMidpoint,
            DistributionMethod::OffsetAverageNeighbor,
            DistributionMethod::OffsetPacking,
        ] {
            let lattice = SphereLattice::generate(method, 600, 1.0).unwrap();
            let positions: Vec<[f32; 3]> = lattice.points().iter().map(|p| p.position).collect();
            let mesh = spherical_delaunay_mesh(&positions);

            let south_index = positions
                .iter()
                .enumerate()
                .min_by(|(_, left), (_, right)| {
                    left[1]
                        .partial_cmp(&right[1])
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(index, _)| index)
                .unwrap_or(0);

            let incident = mesh
                .triangles
                .iter()
                .filter(|triangle| triangle.contains(&south_index))
                .count();

            assert!(
                positions[south_index][1] > -1.0 + 1e-4,
                "{method:?}: expected no explicit south pole sample"
            );
            assert!(
                incident >= 3,
                "{method:?}: southernmost site {south_index} should fan from pole (incident={incident})"
            );
        }
    }

    #[test]
    fn poles_method_still_triangulates() {
        let lattice =
            SphereLattice::generate(DistributionMethod::OffsetPackingWithPoles, 40, 1.0).unwrap();
        let positions: Vec<[f32; 3]> = lattice.points().iter().map(|p| p.position).collect();
        let edges = spherical_delaunay_edges(&positions);
        assert!(edges.len() >= positions.len());
    }

    #[test]
    fn delaunay_covers_all_vertices_above_brute_force_limit() {
        let count = 513;
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, count, 1.0).unwrap();
        let positions: Vec<[f32; 3]> = lattice.points().iter().map(|p| p.position).collect();
        let edges = spherical_delaunay_edges(&positions);

        let mut incident = vec![0usize; positions.len()];
        for [a, b] in &edges {
            incident[*a] += 1;
            incident[*b] += 1;
        }

        let isolated: Vec<usize> = incident
            .iter()
            .enumerate()
            .filter(|(_, degree)| **degree == 0)
            .map(|(index, _)| index)
            .collect();
        assert!(
            isolated.is_empty(),
            "isolated vertices at n={count}: {} of them",
            isolated.len()
        );
    }

    #[test]
    fn high_point_counts_maintain_full_connectivity() {
        for &count in &[600, 1_000] {
            let lattice =
                SphereLattice::generate(DistributionMethod::CanonicalMidpoint, count, 1.0).unwrap();
            let positions: Vec<[f32; 3]> = lattice.points().iter().map(|p| p.position).collect();
            let edges = spherical_delaunay_edges(&positions);
            assert!(
                !edges.is_empty(),
                "count {count}: no edges produced"
            );

            let mut incident = vec![0usize; positions.len()];
            for [a, b] in &edges {
                incident[*a] += 1;
                incident[*b] += 1;
            }

            let isolated: Vec<usize> = incident
                .iter()
                .enumerate()
                .filter(|(_, degree)| **degree == 0)
                .map(|(index, _)| index)
                .collect();
            assert!(
                isolated.is_empty(),
                "count {count}: isolated vertices {isolated:?}"
            );

            let low_degree: Vec<(usize, usize)> = incident
                .iter()
                .enumerate()
                .filter(|(_, degree)| **degree < 3)
                .map(|(index, degree)| (index, *degree))
                .collect();
            assert!(
                low_degree.is_empty(),
                "count {count}: vertices with degree < 3: {low_degree:?}"
            );

            let avg_degree = (2 * edges.len()) as f64 / positions.len() as f64;
            assert!(
                (5.0..=7.5).contains(&avg_degree),
                "count {count}: average degree {avg_degree} outside expected range"
            );
        }
    }
}
