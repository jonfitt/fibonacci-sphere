//! Closest-neighbor queries for lattice analysis.

use crate::lattice::SphereLattice;

/// A neighboring sample point and its chord distance.
#[derive(Debug, Clone, PartialEq)]
pub struct Neighbor {
    /// Index of the neighboring point in the lattice.
    pub index: usize,
    /// Euclidean distance between the two point positions.
    pub distance: f64,
}

/// A group of neighbors whose distances fall within a tolerance band.
#[derive(Debug, Clone, PartialEq)]
pub struct DistanceBin {
    /// Representative distance for this bin (typically the first member).
    pub representative: f64,
    /// Neighbors grouped into this bin.
    pub members: Vec<Neighbor>,
}

/// Query closest neighbors and bin distances by similarity.
pub trait NeighborQuery {
    /// Return the `k` closest neighbors to the point at `index`.
    fn closest_neighbors(&self, index: usize, k: usize) -> Vec<Neighbor>;

    /// Group neighbors of `index` into bins where distances differ by at most `tolerance`.
    fn bin_distances(&self, index: usize, tolerance: f64) -> Vec<DistanceBin>;
}

impl NeighborQuery for SphereLattice {
    fn closest_neighbors(&self, index: usize, k: usize) -> Vec<Neighbor> {
        if index >= self.len() || k == 0 {
            return Vec::new();
        }

        let origin = self.points()[index].position;
        let mut neighbors: Vec<Neighbor> = self
            .points()
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(i, point)| Neighbor {
                index: i,
                distance: euclidean_distance(origin, point.position),
            })
            .collect();

        neighbors.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        neighbors.truncate(k.min(neighbors.len()));
        neighbors
    }

    fn bin_distances(&self, index: usize, tolerance: f64) -> Vec<DistanceBin> {
        if index >= self.len() || tolerance <= 0.0 {
            return Vec::new();
        }

        let mut neighbors = self.closest_neighbors(index, self.len().saturating_sub(1));
        if neighbors.is_empty() {
            return Vec::new();
        }

        let mut bins = Vec::new();
        let mut current = DistanceBin {
            representative: neighbors[0].distance,
            members: vec![neighbors.remove(0)],
        };

        for neighbor in neighbors {
            if (neighbor.distance - current.representative).abs() <= tolerance {
                current.members.push(neighbor);
            } else {
                bins.push(current);
                current = DistanceBin {
                    representative: neighbor.distance,
                    members: vec![neighbor],
                };
            }
        }

        bins.push(current);
        bins
    }
}

fn euclidean_distance(a: [f32; 3], b: [f32; 3]) -> f64 {
    let dx = f64::from(a[0] - b[0]);
    let dy = f64::from(a[1] - b[1]);
    let dz = f64::from(a[2] - b[2]);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::DistributionMethod;
    use crate::SphereLattice;

    #[test]
    fn closest_neighbors_excludes_self_and_respects_k() {
        let lattice = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 10, 1.0).unwrap();
        let neighbors = lattice.closest_neighbors(0, 3);
        assert_eq!(neighbors.len(), 3);
        assert!(neighbors.iter().all(|neighbor| neighbor.index != 0));
        for window in neighbors.windows(2) {
            assert!(window[0].distance <= window[1].distance);
        }
    }

    #[test]
    fn closest_neighbors_out_of_range_returns_empty() {
        let lattice = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 5, 1.0).unwrap();
        assert!(lattice.closest_neighbors(99, 3).is_empty());
        assert!(lattice.closest_neighbors(0, 0).is_empty());
    }

    #[test]
    fn bin_distances_groups_by_tolerance() {
        let lattice = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 20, 1.0).unwrap();
        let bins = lattice.bin_distances(0, 0.05);
        assert!(!bins.is_empty());
        for bin in &bins {
            assert!(!bin.members.is_empty());
        }
    }
}
