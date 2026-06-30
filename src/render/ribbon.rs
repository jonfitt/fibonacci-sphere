//! Line ribbon mesh generation for thick wireframe-style overlays.

const COINCIDENT_EPS: f32 = 1e-9;

/// Triangle mesh approximating line segments as view-facing ribbons.
#[derive(Debug, Clone, PartialEq)]
pub struct LineRibbonMesh {
    /// Ribbon corner vertices in world space.
    pub vertices: Vec<[f32; 3]>,
    /// Triangle corner indices into [`Self::vertices`].
    pub indices: Vec<u32>,
}

/// Offset a point radially outward from the sphere center.
pub fn outward_lift(point: [f32; 3], distance: f32) -> [f32; 3] {
    if distance <= f32::EPSILON {
        return point;
    }

    let len_sq = point[0] * point[0] + point[1] * point[1] + point[2] * point[2];
    if len_sq <= f32::EPSILON {
        return point;
    }

    let inv_len = len_sq.sqrt().recip();
    let normal = [point[0] * inv_len, point[1] * inv_len, point[2] * inv_len];
    [
        point[0] + normal[0] * distance,
        point[1] + normal[1] * distance,
        point[2] + normal[2] * distance,
    ]
}

/// Build a ribbon triangle mesh from paired segment endpoints (`[start, end, ...]`).
pub fn build_line_ribbon_mesh(segments: &[[f32; 3]], width: f32, lift: f32) -> LineRibbonMesh {
    let mut mesh = LineRibbonMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
    };

    if segments.len() < 2 || width <= f32::EPSILON {
        return mesh;
    }

    let half_width = width * 0.5;
    for pair in segments.chunks_exact(2) {
        append_segment_ribbon(
            outward_lift(pair[0], lift),
            outward_lift(pair[1], lift),
            half_width,
            &mut mesh.vertices,
            &mut mesh.indices,
        );
    }

    mesh
}

fn append_segment_ribbon(
    start: [f32; 3],
    end: [f32; 3],
    half_width: f32,
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
) {
    if coincident(start, end) {
        return;
    }

    let tangent = normalize(sub(end, start));
    let start_offset = ribbon_offset(start, tangent, half_width);
    let end_offset = ribbon_offset(end, tangent, half_width);
    let base_index = vertices.len() as u32;

    vertices.push(add(start, start_offset));
    vertices.push(sub(start, start_offset));
    vertices.push(sub(end, end_offset));
    vertices.push(add(end, end_offset));
    indices.extend_from_slice(&[
        base_index,
        base_index + 1,
        base_index + 2,
        base_index,
        base_index + 2,
        base_index + 3,
    ]);
}

fn ribbon_offset(point: [f32; 3], tangent: [f32; 3], half_width: f32) -> [f32; 3] {
    let normal = normalize(point);
    let along = sub(tangent, scale(normal, dot(tangent, normal)));
    let along = if length_sq(along) <= COINCIDENT_EPS {
        let fallback = cross(normal, [1.0, 0.0, 0.0]);
        if length_sq(fallback) <= COINCIDENT_EPS {
            cross(normal, [0.0, 0.0, 1.0])
        } else {
            fallback
        }
    } else {
        along
    };
    scale(normalize(cross(along, normal)), half_width)
}

fn coincident(a: [f32; 3], b: [f32; 3]) -> bool {
    sub(a, b).iter().all(|component| component.abs() <= COINCIDENT_EPS)
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
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

fn length_sq(v: [f32; 3]) -> f32 {
    dot(v, v)
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len_sq = length_sq(v);
    if len_sq <= f32::EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        scale(v, len_sq.sqrt().recip())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ribbon_mesh_has_triangles_for_one_segment() {
        let mesh = build_line_ribbon_mesh(&[[0.0, 1.0, 0.0], [1.0, 0.0, 0.0]], 0.1, 0.0);
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
    }
}
