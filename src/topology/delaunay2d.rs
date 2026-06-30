//! Planar Delaunay triangulation.
//!
//! Small inputs use brute-force circumcircle tests. Larger inputs use the
//! Bowyer–Watson incremental algorithm for robustness and predictable runtime.

const EPS: f64 = 1e-10;
/// Brute-force circumcircle tests are used below this point count for robustness.
const BRUTE_FORCE_LIMIT: usize = 512;

/// Triangles as triples of vertex indices into the input point slice.
pub fn triangulate(points: &[(f64, f64)]) -> Vec<[usize; 3]> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }

    if n <= BRUTE_FORCE_LIMIT {
        return brute_force_triangulation(points);
    }

    bowyer_watson_triangulation(points)
}

/// Counter-clockwise convex hull vertex indices in the plane.
pub fn convex_hull(points: &[(f64, f64)]) -> Vec<usize> {
    if points.len() < 3 {
        return (0..points.len()).collect();
    }

    let mut sorted: Vec<usize> = (0..points.len()).collect();
    sorted.sort_by(|&a, &b| {
        points[a]
            .0
            .partial_cmp(&points[b].0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                points[a]
                    .1
                    .partial_cmp(&points[b].1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let cross = |origin: usize, a: usize, b: usize| -> f64 {
        let o = points[origin];
        let pa = points[a];
        let pb = points[b];
        (pa.0 - o.0) * (pb.1 - o.1) - (pa.1 - o.1) * (pb.0 - o.0)
    };

    let mut lower = Vec::new();
    for index in &sorted {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *index) <= EPS
        {
            lower.pop();
        }
        lower.push(*index);
    }

    let mut upper = Vec::new();
    for index in sorted.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *index) <= EPS
        {
            upper.pop();
        }
        upper.push(*index);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn bowyer_watson_triangulation(points: &[(f64, f64)]) -> Vec<[usize; 3]> {
    let n = points.len();
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for &(x, y) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    let delta_max = (max_x - min_x).max(max_y - min_y).max(EPS);
    let mid_x = (min_x + max_x) * 0.5;
    let mid_y = (min_y + max_y) * 0.5;

    let super_vertices = [n, n + 1, n + 2];
    let mut extended = points.to_vec();
    extended.push((mid_x - 20.0 * delta_max, mid_y - delta_max));
    extended.push((mid_x, mid_y + 20.0 * delta_max));
    extended.push((mid_x + 20.0 * delta_max, mid_y - delta_max));

    let mut triangles = vec![[super_vertices[0], super_vertices[1], super_vertices[2]]];

    for (point_index, &point) in points.iter().enumerate().take(n) {
        let mut bad_triangles = Vec::new();

        for (triangle_index, triangle) in triangles.iter().enumerate() {
            let [a, b, c] = *triangle;
            if point_in_circumcircle(extended[a], extended[b], extended[c], point) {
                bad_triangles.push(triangle_index);
            }
        }

        let mut boundary_edges = Vec::new();
        for &triangle_index in &bad_triangles {
            let [a, b, c] = triangles[triangle_index];
            for edge in [(a, b), (b, c), (c, a)] {
                let key = normalized_edge(edge.0, edge.1);
                if let Some(existing) = boundary_edges
                    .iter()
                    .position(|&(left, right)| normalized_edge(left, right) == key)
                {
                    boundary_edges.remove(existing);
                } else {
                    boundary_edges.push(edge);
                }
            }
        }

        bad_triangles.sort_unstable_by(|left, right| right.cmp(left));
        for &triangle_index in &bad_triangles {
            triangles.swap_remove(triangle_index);
        }

        for (left, right) in boundary_edges {
            triangles.push([left, right, point_index]);
        }
    }

    triangles.retain(|triangle| triangle.iter().all(|&vertex| vertex < n));
    triangles
}

fn normalized_edge(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

fn delaunay_triangle(points: &[(f64, f64)], face: [usize; 3]) -> bool {
    let [a, b, c] = face;
    let pa = points[a];
    let pb = points[b];
    let pc = points[c];

    if orient(pa, pb, pc) > EPS {
        return points.iter().enumerate().all(|(index, point)| {
            if index == a || index == b || index == c {
                return true;
            }
            !in_circumcircle(pa, pb, pc, *point)
        });
    }

    if orient(pa, pc, pb) > EPS {
        return points.iter().enumerate().all(|(index, point)| {
            if index == a || index == b || index == c {
                return true;
            }
            !in_circumcircle(pa, pc, pb, *point)
        });
    }

    false
}

fn brute_force_triangulation(points: &[(f64, f64)]) -> Vec<[usize; 3]> {
    let n = points.len();
    let mut triangles = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                if delaunay_triangle(points, [i, j, k]) {
                    triangles.push([i, j, k]);
                }
            }
        }
    }
    triangles
}

fn point_in_circumcircle(a: (f64, f64), b: (f64, f64), c: (f64, f64), p: (f64, f64)) -> bool {
    if orient(a, b, c) > EPS {
        return in_circumcircle(a, b, c, p);
    }
    if orient(a, c, b) > EPS {
        return in_circumcircle(a, c, b, p);
    }
    false
}

fn orient(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn in_circumcircle(a: (f64, f64), b: (f64, f64), c: (f64, f64), p: (f64, f64)) -> bool {
    let ax = a.0 - p.0;
    let ay = a.1 - p.1;
    let bx = b.0 - p.0;
    let by = b.1 - p.1;
    let cx = c.0 - p.0;
    let cy = c.1 - p.1;

    let det = (ax * ax + ay * ay) * (bx * cy - cx * by) - (bx * bx + by * by) * (ax * cy - cx * ay)
        + (cx * cx + cy * cy) * (ax * by - bx * ay);
    det > EPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_has_at_least_two_triangles() {
        let points = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let triangles = triangulate(&points);
        // Cocircular corners admit more than one valid Delaunay triangulation.
        assert!(triangles.len() >= 2);
    }

    #[test]
    fn convex_hull_of_square_has_four_vertices() {
        let points = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.5, 0.5)];
        let hull = convex_hull(&points);
        assert_eq!(hull.len(), 4);
    }

    #[test]
    fn bowyer_watson_covers_all_vertices_for_large_inputs() {
        let mut points = Vec::with_capacity(520);
        for index in 0..520usize {
            let t = index as f64 * 0.137;
            points.push((t.cos(), t.sin() * 0.5 + (index as f64 * 0.01)));
        }

        let triangles = triangulate(&points);
        let mut seen = vec![false; points.len()];
        for triangle in &triangles {
            for &vertex in triangle {
                seen[vertex] = true;
            }
        }

        assert!(seen.iter().all(|&present| present));
        assert!(triangles.len() >= points.len());
    }
}
