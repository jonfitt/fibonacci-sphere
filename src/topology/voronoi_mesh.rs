//! Shared Voronoi cell fan triangulation for renderers (Bevy, Godot, etc.).

const POLE_APEX_EPS: f32 = 1e-4;
const COINCIDENT_DOT_EPS: f32 = 1e-5;

/// Default radial inset for cell meshes (slightly below the lattice radius).
pub const DEFAULT_SURFACE_INSET: f32 = 0.997;

/// Options for [`build_voronoi_cell_fan_mesh`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoronoiFanMeshOptions {
    /// Multiplier applied to `radius` for vertex placement on the sphere.
    pub surface_inset: f32,
    /// Subdivisions along each boundary edge (`1` = no subdivision).
    pub edge_subdivisions: u32,
    /// When true, emit `[apex, right, left]` instead of `[apex, left, right]`.
    ///
    /// Godot front faces are CCW with `CULL_BACK`; Bevy uses the opposite order.
    pub flip_winding: bool,
}

impl Default for VoronoiFanMeshOptions {
    fn default() -> Self {
        Self {
            surface_inset: DEFAULT_SURFACE_INSET,
            edge_subdivisions: 1,
            flip_winding: false,
        }
    }
}

/// Triangle mesh for one Voronoi cell using a fan from `fan_apex`.
#[derive(Debug, Clone, PartialEq)]
pub struct VoronoiFanMesh {
    /// Triangle vertices in world space.
    pub vertices: Vec<[f32; 3]>,
    /// Triangle corner indices into `vertices`.
    pub triangles: Vec<[usize; 3]>,
}

/// Returns true when `apex` lies on the geographic north or south pole.
pub fn is_geographic_pole_apex(apex: [f32; 3]) -> bool {
    let dir = normalize(apex);
    dir[1] <= -1.0 + POLE_APEX_EPS || dir[1] >= 1.0 - POLE_APEX_EPS
}

/// Returns true when two directions on the unit sphere are effectively the same.
pub fn coincident_on_sphere(a: [f32; 3], b: [f32; 3]) -> bool {
    normalize(a)
        .iter()
        .zip(normalize(b))
        .map(|(left, right)| left * right)
        .sum::<f32>()
        > 1.0 - COINCIDENT_DOT_EPS
}

/// Drop the geographic pole from the boundary ring when it duplicates the fan apex.
pub fn mesh_boundary_ring(
    fan_apex: [f32; 3],
    boundary: &[[f32; 3]],
    surface_radius: f32,
) -> Vec<[f32; 3]> {
    let strip_pole = is_geographic_pole_apex(fan_apex);
    boundary
        .iter()
        .filter(|point| {
            !(strip_pole && coincident_on_sphere(scale(normalize(**point), surface_radius), fan_apex))
        })
        .map(|point| scale(normalize(*point), surface_radius))
        .collect()
}

/// Signed sum used to detect boundary winding relative to the fan apex.
pub fn boundary_winding_sign(apex: [f32; 3], boundary: &[[f32; 3]]) -> f32 {
    let apex_dir = normalize(apex);
    let mut sum = 0.0;
    for edge in 0..boundary.len() {
        let next = (edge + 1) % boundary.len();
        let a = normalize(boundary[edge]);
        let b = normalize(boundary[next]);
        let ax = a[0] - apex_dir[0];
        let ay = a[1] - apex_dir[1];
        let az = a[2] - apex_dir[2];
        let bx = b[0] - apex_dir[0];
        let by = b[1] - apex_dir[1];
        let bz = b[2] - apex_dir[2];
        let nx = ay * bz - az * by;
        let ny = az * bx - ax * bz;
        let nz = ax * by - ay * bx;
        sum += nx * apex_dir[0] + ny * apex_dir[1] + nz * apex_dir[2];
    }
    sum
}

/// Build a fan-triangulated Voronoi cell mesh.
///
/// Boundary winding is corrected so fan triangles point outward from the sphere.
/// Set [`VoronoiFanMeshOptions::flip_winding`] for Godot (`CULL_BACK` + CCW front faces).
pub fn build_voronoi_cell_fan_mesh(
    fan_apex: [f32; 3],
    boundary: &[[f32; 3]],
    radius: f32,
    options: VoronoiFanMeshOptions,
) -> Option<VoronoiFanMesh> {
    if boundary.len() < 3 {
        return None;
    }

    let surface_radius = radius * options.surface_inset;
    let apex_vertex = scale(normalize(fan_apex), surface_radius);
    let mut ring = mesh_boundary_ring(apex_vertex, boundary, surface_radius);
    if ring.len() < 3 {
        return None;
    }

    if boundary_winding_sign(apex_vertex, &ring) < 0.0 {
        ring.reverse();
    }

    let strip_pole = is_geographic_pole_apex(apex_vertex);
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    vertices.push(apex_vertex);

    for edge in 0..ring.len() {
        let next = (edge + 1) % ring.len();
        let start = ring[edge];
        let end = ring[next];
        let mut segment = vec![start];
        if options.edge_subdivisions > 1 {
            for step in 1..options.edge_subdivisions {
                let t = step as f32 / options.edge_subdivisions as f32;
                segment.push(slerp_on_sphere(start, end, t, surface_radius));
            }
        }
        segment.push(end);

        for window in segment.windows(2) {
            let left = window[0];
            let right = window[1];
            if strip_pole
                && (coincident_on_sphere(left, right)
                    || coincident_on_sphere(left, apex_vertex)
                    || coincident_on_sphere(right, apex_vertex))
            {
                continue;
            }

            let left_index = vertices.len();
            vertices.push(left);
            let right_index = vertices.len();
            vertices.push(right);
            if options.flip_winding {
                triangles.push([0, right_index, left_index]);
            } else {
                triangles.push([0, left_index, right_index]);
            }
        }
    }

    if triangles.is_empty() {
        None
    } else {
        Some(VoronoiFanMesh { vertices, triangles })
    }
}

fn slerp_on_sphere(start: [f32; 3], end: [f32; 3], t: f32, radius: f32) -> [f32; 3] {
    let a = normalize(start);
    let b = normalize(end);
    let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    let omega = dot.acos();
    if omega <= 1e-6 {
        return scale(a, radius);
    }
    let sin_omega = omega.sin();
    let weight_a = ((1.0 - t) * omega).sin() / sin_omega;
    let weight_b = (t * omega).sin() / sin_omega;
    scale(
        [
            a[0] * weight_a + b[0] * weight_b,
            a[1] * weight_a + b[1] * weight_b,
            a[2] * weight_a + b[2] * weight_b,
        ],
        radius,
    )
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    if len_sq <= f32::EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        let inv = len_sq.sqrt().recip();
        [v[0] * inv, v[1] * inv, v[2] * inv]
    }
}

fn scale(v: [f32; 3], factor: f32) -> [f32; 3] {
    [v[0] * factor, v[1] * factor, v[2] * factor]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fan_mesh_has_triangles_for_square_boundary() {
        let radius = 1.0;
        let apex = [0.0, 1.0, 0.0];
        let boundary = [
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
        ];
        let mesh = build_voronoi_cell_fan_mesh(apex, &boundary, radius, VoronoiFanMeshOptions::default())
            .expect("fan mesh");
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.triangles.len(), 4);
    }
}
