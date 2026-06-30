//! Spherical Voronoi cells dual to the Delaunay triangulation (Plan B investigation).
//!
//! Each lattice vertex is the generator of one cell. Cell boundaries pass through
//! circumcenters of adjacent Delaunay triangles, not through the wireframe edges
//! used for routing.

use std::collections::BTreeMap;

use crate::topology::SphericalMesh;

const POLE_CAP_EPS: f32 = 1e-4;
const SOUTH_POLE: [f32; 3] = [0.0, -1.0, 0.0];
const NORTH_POLE: [f32; 3] = [0.0, 1.0, 0.0];

/// One Voronoi cell centered on a lattice site.
#[derive(Debug, Clone, PartialEq)]
pub struct VoronoiCell {
    /// Generator vertex index (cell center).
    pub site_index: usize,
    /// Boundary polygon vertices on the unit sphere (circumcenters of incident triangles).
    pub boundary: Vec<[f32; 3]>,
    /// Neighbor site across boundary edge `i` (from vertex `i` to `(i + 1) % n`).
    pub boundary_neighbors: Vec<usize>,
}

/// Compute spherical Voronoi cells from positions and the Delaunay mesh.
///
/// Cell boundaries connect circumcenters of Delaunay triangles in cyclic order
/// around each site.
pub fn spherical_voronoi_cells(positions: &[[f32; 3]], mesh: &SphericalMesh) -> Vec<VoronoiCell> {
    let circumcenters = triangle_circumcenters(positions, mesh);
    let edge_to_faces = triangle_edge_to_faces(mesh);
    let vertex_count = positions.len();
    let mut incident: Vec<Vec<usize>> = vec![Vec::new(); vertex_count];

    for (face_index, triangle) in mesh.triangles.iter().enumerate() {
        for &vertex in triangle {
            incident[vertex].push(face_index);
        }
    }

    let south_index = polar_extreme_index(positions, true);
    let north_index = polar_extreme_index(positions, false);

    (0..vertex_count)
        .map(|site_index| {
            let ordered_faces = order_faces_around_vertex(
                site_index,
                positions,
                &incident[site_index],
                mesh,
                &edge_to_faces,
            );
            let mut boundary_neighbors =
                boundary_neighbors_for_faces(site_index, &ordered_faces, mesh);
            let site = normalize_f32(positions[site_index]);
            let mut boundary = ensure_outward_boundary_winding(
                site,
                ordered_faces
                    .iter()
                    .filter_map(|&face_index| circumcenters[face_index])
                    .collect(),
            );
            (boundary, boundary_neighbors) = close_polar_cap_boundary(
                positions,
                site_index,
                south_index,
                north_index,
                site,
                boundary,
                boundary_neighbors,
            );
            VoronoiCell {
                site_index,
                boundary,
                boundary_neighbors,
            }
        })
        .collect()
}

/// Unit-sphere apex for fan-triangulating a Voronoi cell mesh.
///
/// Only the southernmost or northernmost site fans from a geographic pole when no
/// sample sits at that pole. All other cells fan from their generator site.
pub fn voronoi_cell_fan_apex(site_index: usize, positions: &[[f32; 3]]) -> [f32; 3] {
    let south_index = polar_extreme_index(positions, true);
    let north_index = polar_extreme_index(positions, false);

    if site_index == south_index && positions[south_index][1] > -1.0 + POLE_CAP_EPS {
        return SOUTH_POLE;
    }
    if site_index == north_index && positions[north_index][1] < 1.0 - POLE_CAP_EPS {
        return NORTH_POLE;
    }
    normalize_f32(positions[site_index])
}

/// Voronoi diagram edges as segments between circumcenters of adjacent Delaunay triangles.
///
/// Each internal Delaunay edge is shared by two triangles; the segment between their
/// circumcenters is one Voronoi edge. This is the preferred representation for drawing.
pub fn spherical_voronoi_border_segments(
    positions: &[[f32; 3]],
    mesh: &SphericalMesh,
) -> Vec<([f32; 3], [f32; 3])> {
    let circumcenters = triangle_circumcenters(positions, mesh);
    let edge_to_faces = triangle_edge_to_faces(mesh);

    edge_to_faces
        .values()
        .filter(|faces| faces.len() == 2)
        .filter_map(|faces| {
            let start = circumcenters[faces[0]]?;
            let end = circumcenters[faces[1]]?;
            Some((start, end))
        })
        .collect()
}

fn triangle_circumcenters(positions: &[[f32; 3]], mesh: &SphericalMesh) -> Vec<Option<[f32; 3]>> {
    mesh.triangles
        .iter()
        .map(|&triangle| spherical_triangle_circumcenter(positions, triangle))
        .collect()
}

fn triangle_edge_to_faces(mesh: &SphericalMesh) -> BTreeMap<(usize, usize), Vec<usize>> {
    let mut edge_to_faces: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();

    for (face_index, [a, b, c]) in mesh.triangles.iter().enumerate() {
        for (left, right) in [(*a, *b), (*b, *c), (*c, *a)] {
            edge_to_faces
                .entry(normalized_edge(left, right))
                .or_default()
                .push(face_index);
        }
    }

    edge_to_faces
}

fn order_faces_around_vertex(
    site: usize,
    positions: &[[f32; 3]],
    incident_faces: &[usize],
    mesh: &SphericalMesh,
    edge_to_faces: &BTreeMap<(usize, usize), Vec<usize>>,
) -> Vec<usize> {
    if incident_faces.len() <= 1 {
        return incident_faces.to_vec();
    }

    let site_y = normalize_f32(positions[site])[1];
    if site_y.abs() > 0.92 {
        return sort_faces_by_angle(site, positions, incident_faces, mesh);
    }

    let start_face = incident_faces[0];
    let Some((first_neighbor, _second_neighbor)) =
        neighbors_of_vertex_in_face(start_face, site, mesh)
    else {
        return sort_faces_by_angle(site, positions, incident_faces, mesh);
    };

    let mut ordered = Vec::with_capacity(incident_faces.len());
    let mut current_face = start_face;
    let mut previous_neighbor = first_neighbor;

    for _ in 0..incident_faces.len() {
        ordered.push(current_face);

        let Some((neighbor_a, neighbor_b)) = neighbors_of_vertex_in_face(current_face, site, mesh)
        else {
            break;
        };
        let next_neighbor = if previous_neighbor == neighbor_a {
            neighbor_b
        } else {
            neighbor_a
        };

        let Some(next_face) = opposite_face_across_edge(
            edge_to_faces,
            normalized_edge(site, next_neighbor),
            current_face,
        ) else {
            break;
        };

        current_face = next_face;
        previous_neighbor = next_neighbor;
    }

    if ordered.len() == incident_faces.len() {
        return ordered;
    }

    sort_faces_by_angle(site, positions, incident_faces, mesh)
}

fn opposite_face_across_edge(
    edge_to_faces: &BTreeMap<(usize, usize), Vec<usize>>,
    edge: (usize, usize),
    current_face: usize,
) -> Option<usize> {
    edge_to_faces
        .get(&edge)?
        .iter()
        .copied()
        .find(|&face_index| face_index != current_face)
}

fn neighbors_of_vertex_in_face(
    face_index: usize,
    site: usize,
    mesh: &SphericalMesh,
) -> Option<(usize, usize)> {
    let [a, b, c] = mesh.triangles[face_index];
    if a == site {
        Some((b, c))
    } else if b == site {
        Some((c, a))
    } else if c == site {
        Some((a, b))
    } else {
        None
    }
}

fn sort_faces_by_angle(
    site: usize,
    positions: &[[f32; 3]],
    incident_faces: &[usize],
    mesh: &SphericalMesh,
) -> Vec<usize> {
    let site_position = normalize_f32(positions[site]);
    let (axis_u, axis_v) = tangent_basis(site_position);
    let mut ranked: Vec<(f32, usize)> = incident_faces
        .iter()
        .copied()
        .filter_map(|face_index| {
            let circumcenter =
                spherical_triangle_circumcenter(positions, mesh.triangles[face_index])?;
            let angle = tangent_angle(circumcenter, axis_u, axis_v);
            Some((angle, face_index))
        })
        .collect();
    ranked.sort_by(|(left, _), (right, _)| {
        left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked
        .into_iter()
        .map(|(_, face_index)| face_index)
        .collect()
}

fn polar_extreme_index(positions: &[[f32; 3]], south: bool) -> usize {
    positions
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            if south {
                left[1]
                    .partial_cmp(&right[1])
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                right[1]
                    .partial_cmp(&left[1])
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

/// Close a polar Voronoi cell through the geographic pole when no sample sits there.
fn close_polar_cap_boundary(
    positions: &[[f32; 3]],
    site_index: usize,
    south_index: usize,
    north_index: usize,
    site: [f32; 3],
    boundary: Vec<[f32; 3]>,
    boundary_neighbors: Vec<usize>,
) -> (Vec<[f32; 3]>, Vec<usize>) {
    if boundary.len() < 3 || boundary.len() != boundary_neighbors.len() {
        return (boundary, boundary_neighbors);
    }

    if site_index == south_index && positions[south_index][1] > -1.0 + POLE_CAP_EPS {
        return close_polar_cap_with_neighbors(site, boundary, boundary_neighbors, SOUTH_POLE);
    }

    if site_index == north_index && positions[north_index][1] < 1.0 - POLE_CAP_EPS {
        return close_polar_cap_with_neighbors(site, boundary, boundary_neighbors, NORTH_POLE);
    }

    (boundary, boundary_neighbors)
}

fn close_polar_cap_with_neighbors(
    site: [f32; 3],
    boundary: Vec<[f32; 3]>,
    mut boundary_neighbors: Vec<usize>,
    pole: [f32; 3],
) -> (Vec<[f32; 3]>, Vec<usize>) {
    let (boundary, insert_index) = insert_pole_in_boundary(site, boundary, pole);
    if let Some(index) = insert_index {
        let split_neighbor = boundary_neighbors[index];
        boundary_neighbors.insert(index + 1, split_neighbor);
    }
    (
        ensure_outward_boundary_winding(site, boundary),
        boundary_neighbors,
    )
}

fn insert_pole_in_boundary(
    site: [f32; 3],
    boundary: Vec<[f32; 3]>,
    pole: [f32; 3],
) -> (Vec<[f32; 3]>, Option<usize>) {
    if boundary.len() < 3 {
        return (boundary, None);
    }

    let pole = normalize_f32(pole);
    if boundary
        .iter()
        .any(|point| (normalize_f32(*point)[1] - pole[1]).abs() <= POLE_CAP_EPS)
    {
        return (boundary, None);
    }

    let (axis_u, axis_v) = tangent_basis(site);
    let pole_angle = tangent_angle(pole, axis_u, axis_v);
    let angles: Vec<f32> = boundary
        .iter()
        .map(|point| tangent_angle(normalize_f32(*point), axis_u, axis_v))
        .collect();

    for index in 0..boundary.len() {
        let start = angles[index];
        let end = angles[(index + 1) % boundary.len()];
        if angle_in_ccw_arc(pole_angle, start, end) {
            let mut closed = boundary;
            closed.insert(index + 1, pole);
            return (closed, Some(index));
        }
    }

    let mut closed = boundary;
    closed.push(pole);
    (closed, None)
}

fn boundary_neighbors_for_faces(
    site: usize,
    ordered_faces: &[usize],
    mesh: &SphericalMesh,
) -> Vec<usize> {
    if ordered_faces.len() < 2 {
        return Vec::new();
    }

    ordered_faces
        .iter()
        .enumerate()
        .filter_map(|(index, &face)| {
            let next_face = ordered_faces[(index + 1) % ordered_faces.len()];
            neighbor_across_faces(site, face, next_face, mesh)
        })
        .collect()
}

fn neighbor_across_faces(
    site: usize,
    face_a: usize,
    face_b: usize,
    mesh: &SphericalMesh,
) -> Option<usize> {
    let (a0, a1) = neighbors_of_vertex_in_face(face_a, site, mesh)?;
    let (b0, b1) = neighbors_of_vertex_in_face(face_b, site, mesh)?;
    if a0 == b0 || a0 == b1 {
        Some(a0)
    } else if a1 == b0 || a1 == b1 {
        Some(a1)
    } else {
        None
    }
}

fn angle_in_ccw_arc(target: f32, start: f32, end: f32) -> bool {
    let mut span = end - start;
    if span <= 0.0 {
        span += std::f32::consts::TAU;
    }
    let mut offset = target - start;
    if offset < 0.0 {
        offset += std::f32::consts::TAU;
    }
    offset <= span + 1e-5
}

fn ensure_outward_boundary_winding(site: [f32; 3], mut boundary: Vec<[f32; 3]>) -> Vec<[f32; 3]> {
    if boundary.len() < 3 {
        return boundary;
    }

    if boundary_winding_sign(site, &boundary) < 0.0 {
        boundary.reverse();
    }

    boundary
}

fn boundary_winding_sign(site: [f32; 3], boundary: &[[f32; 3]]) -> f32 {
    let mut sum = 0.0;
    for edge in 0..boundary.len() {
        let next = (edge + 1) % boundary.len();
        let a = normalize_f32(boundary[edge]);
        let b = normalize_f32(boundary[next]);
        let normal = cross(
            [a[0] - site[0], a[1] - site[1], a[2] - site[2]],
            [b[0] - site[0], b[1] - site[1], b[2] - site[2]],
        );
        sum += dot(normal, site);
    }
    sum
}

fn tangent_basis(site: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let reference = if site[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let axis_u = normalize_f32(cross(site, reference));
    let axis_v = cross(site, axis_u);
    (axis_u, axis_v)
}

fn tangent_angle(point: [f32; 3], axis_u: [f32; 3], axis_v: [f32; 3]) -> f32 {
    dot(point, axis_v).atan2(dot(point, axis_u))
}

/// Circumcenter of a spherical triangle whose vertices are unit-sphere samples.
fn spherical_triangle_circumcenter(
    positions: &[[f32; 3]],
    [a, b, c]: [usize; 3],
) -> Option<[f32; 3]> {
    let pa = normalize_f64(to_f64(positions[a]));
    let pb = normalize_f64(to_f64(positions[b]));
    let pc = normalize_f64(to_f64(positions[c]));
    spherical_triangle_circumcenter_unit_f64(pa, pb, pc).map(to_f32)
}

fn spherical_triangle_circumcenter_unit_f64(
    pa: [f64; 3],
    pb: [f64; 3],
    pc: [f64; 3],
) -> Option<[f64; 3]> {
    let ab = dot_f64(pa, pb);
    let ac = dot_f64(pa, pc);
    let bc = dot_f64(pb, pc);

    let matrix = [[1.0, ab, ac], [ab, 1.0, bc], [ac, bc, 1.0]];
    let rhs = [1.0, 1.0, 1.0];
    let [alpha, beta, gamma] = solve_symmetric_3x3(matrix, rhs)?;

    let center = add_f64(
        add_f64(scale_f64(pa, alpha), scale_f64(pb, beta)),
        scale_f64(pc, gamma),
    );
    let center_length = length_f64(center);
    if center_length <= 1e-12 {
        return None;
    }

    Some(scale_f64(center, 1.0 / center_length))
}

fn solve_symmetric_3x3(matrix: [[f64; 3]; 3], rhs: [f64; 3]) -> Option<[f64; 3]> {
    let mut a = matrix;
    let mut b = rhs;

    for pivot in 0..3 {
        let mut max_row = pivot;
        for row in (pivot + 1)..3 {
            if a[row][pivot].abs() > a[max_row][pivot].abs() {
                max_row = row;
            }
        }
        if a[max_row][pivot].abs() <= 1e-12 {
            return None;
        }
        if max_row != pivot {
            a.swap(pivot, max_row);
            b.swap(pivot, max_row);
        }

        for row in (pivot + 1)..3 {
            let factor = a[row][pivot] / a[pivot][pivot];
            #[allow(clippy::needless_range_loop)]
            for col in pivot..3 {
                a[row][col] -= factor * a[pivot][col];
            }
            b[row] -= factor * b[pivot];
        }
    }

    let mut solution = [0.0; 3];
    for row in (0..3).rev() {
        let mut sum = b[row];
        for col in (row + 1)..3 {
            sum -= a[row][col] * solution[col];
        }
        if a[row][row].abs() <= 1e-12 {
            return None;
        }
        solution[row] = sum / a[row][row];
    }

    Some(solution)
}

fn to_f64(v: [f32; 3]) -> [f64; 3] {
    [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]
}

fn to_f32(v: [f64; 3]) -> [f32; 3] {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}

fn normalize_f64(v: [f64; 3]) -> [f64; 3] {
    let len = length_f64(v);
    if len <= f64::EPSILON {
        return [0.0, 1.0, 0.0];
    }
    scale_f64(v, 1.0 / len)
}

fn length_f64(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn dot_f64(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add_f64(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale_f64(v: [f64; 3], factor: f64) -> [f64; 3] {
    [v[0] * factor, v[1] * factor, v[2] * factor]
}

fn normalized_edge(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

fn normalize_f32(v: [f32; 3]) -> [f32; 3] {
    let len = length(v);
    if len <= f32::EPSILON {
        return [0.0, 1.0, 0.0];
    }
    scale(v, 1.0 / len)
}

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn scale(v: [f32; 3], factor: f32) -> [f32; 3] {
    [v[0] * factor, v[1] * factor, v[2] * factor]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SphereLattice;
    use crate::methods::DistributionMethod;

    #[test]
    fn circumcenter_is_equidistant_from_triangle_vertices() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 120, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let mesh = lattice.spherical_mesh();

        for triangle in &mesh.triangles {
            let [a, b, c] = *triangle;
            let Some(center) = spherical_triangle_circumcenter(&positions, [a, b, c]) else {
                continue;
            };
            let pa = normalize_f32(positions[a]);
            let pb = normalize_f32(positions[b]);
            let pc = normalize_f32(positions[c]);
            let da = dot(normalize_f32(center), pa);
            let db = dot(normalize_f32(center), pb);
            let dc = dot(normalize_f32(center), pc);
            assert!(
                (da - db).abs() < 1e-4 && (db - dc).abs() < 1e-4,
                "circumcenter not equidistant by dot product: {da:?} {db:?} {dc:?}"
            );
        }
    }

    #[test]
    fn voronoi_cells_cover_every_site() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 60, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let mesh = lattice.spherical_mesh();
        let cells = spherical_voronoi_cells(&positions, &mesh);
        assert_eq!(cells.len(), positions.len());
        assert!(cells.iter().all(|cell| cell.site_index < positions.len()));
    }

    #[test]
    fn interior_sites_have_polygon_boundaries() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 120, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let mesh = lattice.spherical_mesh();
        let cells = spherical_voronoi_cells(&positions, &mesh);
        let with_boundary = cells.iter().filter(|cell| cell.boundary.len() >= 3).count();
        assert!(
            with_boundary > positions.len() / 2,
            "expected most interior sites to have at least a triangle boundary"
        );
    }

    #[test]
    fn voronoi_borders_match_dual_edges() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 120, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let mesh = lattice.spherical_mesh();
        let segments = spherical_voronoi_border_segments(&positions, &mesh);
        assert!(!segments.is_empty());
        assert!(
            segments.len() >= mesh.edges.len() / 2,
            "expected one dual segment per internal Delaunay edge"
        );
    }

    #[test]
    fn ordered_cell_boundaries_are_cyclic() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 120, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let mesh = lattice.spherical_mesh();
        let cells = spherical_voronoi_cells(&positions, &mesh);

        for cell in cells {
            if cell.boundary.len() < 3 {
                continue;
            }
            let incident = mesh
                .triangles
                .iter()
                .filter(|triangle| triangle.contains(&cell.site_index))
                .count();
            let expected = expected_boundary_len(&positions, cell.site_index, incident);
            assert_eq!(
                cell.boundary.len(),
                expected,
                "site {} should have one circumcenter per incident triangle plus optional polar closure",
                cell.site_index
            );
        }
    }

    #[test]
    fn offset_average_neighbor_south_pole_cell_winding_is_outward() {
        let count = 6000;
        let lattice =
            SphereLattice::generate(DistributionMethod::OffsetAverageNeighbor, count, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let mesh = lattice.spherical_mesh();
        let cells = spherical_voronoi_cells(&positions, &mesh);

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

        let site = normalize_f32(positions[south_index]);
        let boundary = &cells[south_index].boundary;
        assert!(boundary.len() >= 3);
        assert!(
            boundary_winding_sign(site, boundary) > 0.0,
            "south pole cell should use outward fan winding"
        );
    }

    #[test]
    fn all_cells_use_outward_fan_winding() {
        let lattice =
            SphereLattice::generate(DistributionMethod::OffsetAverageNeighbor, 320, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let mesh = lattice.spherical_mesh();
        let cells = spherical_voronoi_cells(&positions, &mesh);

        for cell in cells {
            if cell.boundary.len() < 3 {
                continue;
            }
            let site = normalize_f32(positions[cell.site_index]);
            assert!(
                boundary_winding_sign(site, &cell.boundary) > 0.0,
                "site {} has inward Voronoi fan winding",
                cell.site_index
            );
        }
    }

    #[test]
    fn explicit_south_pole_hub_cell_is_valid() {
        let count = 6000;
        let lattice =
            SphereLattice::generate(DistributionMethod::OffsetPackingWithPoles, count, 1.0)
                .unwrap();
        let positions = lattice.position_arrays();
        let mesh = lattice.spherical_mesh();
        let cells = spherical_voronoi_cells(&positions, &mesh);

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

        let site = normalize_f32(positions[south_index]);
        let boundary = &cells[south_index].boundary;

        eprintln!(
            "OffsetPackingWithPoles south site {south_index} y={:.8} incident={incident} boundary={}",
            positions[south_index][1],
            boundary.len()
        );

        assert!(
            incident > 6,
            "expected hub triangulation at explicit south pole"
        );
        assert_eq!(boundary.len(), incident);

        let south_cap: Vec<usize> = positions
            .iter()
            .enumerate()
            .filter(|(_, position)| position[1] < -0.95)
            .map(|(index, _)| index)
            .collect();

        let mut violations = Vec::new();
        for &other in &south_cap {
            if other == south_index {
                continue;
            }
            if euclidean_fan_contains_point(site, boundary, normalize_f32(positions[other])) {
                violations.push(other);
            }
        }

        assert!(
            violations.is_empty(),
            "south pole hub fan covers neighbors: {violations:?}"
        );
        assert!(
            boundary_winding_sign(site, boundary) > 0.0,
            "south pole hub fan should wind outward"
        );
    }

    #[test]
    fn southernmost_cell_covers_south_pole_for_all_methods() {
        use crate::methods::DistributionMethod;

        let methods = [
            DistributionMethod::CanonicalMidpoint,
            DistributionMethod::OffsetAverageNeighbor,
            DistributionMethod::OffsetPacking,
            DistributionMethod::OffsetPackingWithPoles,
        ];

        for method in methods {
            let lattice = SphereLattice::generate(method, 600, 1.0).unwrap();
            let positions = lattice.position_arrays();
            let mesh = lattice.spherical_mesh();
            let cells = spherical_voronoi_cells(&positions, &mesh);

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
                incident >= 3,
                "{method:?}: expected pole hub triangulation (incident={incident})"
            );
            if positions[south_index][1] > -1.0 + POLE_CAP_EPS {
                assert!(
                    cells[south_index]
                        .boundary
                        .iter()
                        .any(|point| point[1] <= -1.0 + POLE_CAP_EPS),
                    "{method:?}: expected south pole vertex in southernmost cell boundary"
                );
            }
            assert!(
                spherical_voronoi_cell_contains_point(&positions, south_index, [0.0, -1.0, 0.0],),
                "{method:?}: south pole must belong to the southernmost Voronoi cell"
            );
        }
    }

    #[test]
    fn southernmost_cell_mesh_covers_south_pole_after_polar_close() {
        let lattice =
            SphereLattice::generate(DistributionMethod::OffsetAverageNeighbor, 6000, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let mesh = lattice.spherical_mesh();
        let cells = spherical_voronoi_cells(&positions, &mesh);

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

        let boundary = &cells[south_index].boundary;
        assert!(
            boundary.iter().any(|point| point[1] <= -1.0 + POLE_CAP_EPS),
            "expected south pole in boundary"
        );
        assert_eq!(voronoi_cell_fan_apex(south_index, &positions), SOUTH_POLE);
    }

    #[test]
    fn only_polar_extreme_sites_use_geographic_pole_apex() {
        let lattice =
            SphereLattice::generate(DistributionMethod::OffsetAverageNeighbor, 600, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let south_index = polar_extreme_index(&positions, true);
        let north_index = polar_extreme_index(&positions, false);

        for site_index in 0..positions.len() {
            let apex = voronoi_cell_fan_apex(site_index, &positions);
            if site_index == south_index && positions[south_index][1] > -1.0 + POLE_CAP_EPS {
                assert_eq!(apex, SOUTH_POLE);
            } else if site_index == north_index && positions[north_index][1] < 1.0 - POLE_CAP_EPS {
                assert_eq!(apex, NORTH_POLE);
            } else {
                assert_ne!(apex, SOUTH_POLE);
                assert_ne!(apex, NORTH_POLE);
            }
        }
    }

    #[test]
    fn southernmost_cell_needs_pole_fan_apex_without_explicit_pole() {
        let count = 6000;
        let lattice =
            SphereLattice::generate(DistributionMethod::OffsetAverageNeighbor, count, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let mesh = lattice.spherical_mesh();
        let cells = spherical_voronoi_cells(&positions, &mesh);

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

        assert!(
            positions[south_index][1] > -1.0 + 1e-4,
            "expected no explicit south pole sample"
        );

        let boundary = &cells[south_index].boundary;
        assert!(boundary.len() >= 3);
        assert!(
            boundary.iter().any(|point| point[1] <= -1.0 + POLE_CAP_EPS),
            "polar cap should close through the south pole"
        );
    }

    #[test]
    fn no_euclidean_fan_mesh_covers_foreign_site() {
        let count = 6000;
        let lattice =
            SphereLattice::generate(DistributionMethod::OffsetAverageNeighbor, count, 1.0).unwrap();
        let positions = lattice.position_arrays();
        let mesh = lattice.spherical_mesh();
        let cells = spherical_voronoi_cells(&positions, &mesh);

        let south_cap: Vec<usize> = positions
            .iter()
            .enumerate()
            .filter(|(_, position)| position[1] < -0.95)
            .map(|(index, _)| index)
            .collect();

        let mut violations = Vec::new();
        for &owner in &south_cap {
            let site = normalize_f32(positions[owner]);
            let boundary = &cells[owner].boundary;
            if boundary.len() < 3 {
                continue;
            }
            for &other in &south_cap {
                if other == owner {
                    continue;
                }
                let point = normalize_f32(positions[other]);
                if euclidean_fan_contains_point(site, boundary, point) {
                    violations.push((owner, other));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "Euclidean fan meshes cover foreign south-cap sites: {:?}",
            &violations[..violations.len().min(10)]
        );
    }

    fn expected_boundary_len(positions: &[[f32; 3]], site_index: usize, incident: usize) -> usize {
        let south_index = polar_extreme_index(positions, true);
        let north_index = polar_extreme_index(positions, false);
        let mut expected = incident;
        if site_index == south_index && positions[south_index][1] > -1.0 + POLE_CAP_EPS {
            expected += 1;
        }
        if site_index == north_index && positions[north_index][1] < 1.0 - POLE_CAP_EPS {
            expected += 1;
        }
        expected
    }

    fn spherical_voronoi_cell_contains_point(
        positions: &[[f32; 3]],
        site_index: usize,
        point: [f32; 3],
    ) -> bool {
        let point = normalize_f32(point);
        let site = normalize_f32(positions[site_index]);
        positions.iter().enumerate().all(|(index, position)| {
            if index == site_index {
                return true;
            }
            dot(point, site) + 1e-6 >= dot(point, normalize_f32(*position))
        })
    }

    fn euclidean_fan_contains_point(
        site: [f32; 3],
        boundary: &[[f32; 3]],
        point: [f32; 3],
    ) -> bool {
        if boundary.len() < 3 {
            return false;
        }

        for edge in 0..boundary.len() {
            let next = (edge + 1) % boundary.len();
            if point_in_euclidean_triangle(
                point,
                site,
                normalize_f32(boundary[edge]),
                normalize_f32(boundary[next]),
            ) {
                return true;
            }
        }

        false
    }

    fn point_in_euclidean_triangle(point: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> bool {
        let v0 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v1 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let v2 = [point[0] - a[0], point[1] - a[1], point[2] - a[2]];
        let dot00 = dot(v0, v0);
        let dot01 = dot(v0, v1);
        let dot02 = dot(v0, v2);
        let dot11 = dot(v1, v1);
        let dot12 = dot(v1, v2);
        let denom = dot00 * dot11 - dot01 * dot01;
        if denom.abs() <= 1e-12 {
            return false;
        }
        let inv = 1.0 / denom;
        let u = (dot11 * dot02 - dot01 * dot12) * inv;
        let v = (dot00 * dot12 - dot01 * dot02) * inv;
        u >= -1e-6 && v >= -1e-6 && (u + v) <= 1.0 + 1e-6
    }

    #[test]
    fn offset_average_neighbor_south_pole_has_complete_circumcenters() {
        for &count in &[100, 320, 6000] {
            let lattice =
                SphereLattice::generate(DistributionMethod::OffsetAverageNeighbor, count, 1.0)
                    .unwrap();
            let positions = lattice.position_arrays();
            let mesh = lattice.spherical_mesh();
            let cells = spherical_voronoi_cells(&positions, &mesh);

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

            let incident: Vec<[usize; 3]> = mesh
                .triangles
                .iter()
                .copied()
                .filter(|triangle| triangle.contains(&south_index))
                .collect();

            let missing_circumcenters = incident
                .iter()
                .filter(|triangle| {
                    spherical_triangle_circumcenter(&positions, **triangle).is_none()
                })
                .count();

            let small_cells = cells.iter().filter(|cell| cell.boundary.len() < 3).count();

            assert_eq!(
                missing_circumcenters, 0,
                "count {count}: south pole triangles missing circumcenters"
            );
            assert_eq!(
                cells[south_index].boundary.len(),
                incident.len() + usize::from(positions[south_index][1] > -1.0 + POLE_CAP_EPS),
                "count {count}: south pole site {south_index} y={:.6} boundary/incident mismatch",
                positions[south_index][1]
            );
            assert_eq!(
                small_cells, 0,
                "count {count}: {small_cells} sites skipped by visualizer shading"
            );
        }
    }
}
