//! Strategies for changing terrain on invalid vertices.

use std::collections::HashSet;

use rand::RngCore;

use crate::topology::SurfaceGraph;

use super::rng::choose;
use super::types::{ElevationBand, TerrainType};

/// Chooses a replacement terrain type when a vertex fails a post-processing rule.
pub trait TerrainReassigner {
    /// Pick a new terrain type for `node` that currently has `current`.
    fn reassign(
        &self,
        node: usize,
        current: TerrainType,
        terrain: &[TerrainType],
        graph: &SurfaceGraph,
        rng: &mut dyn RngCore,
    ) -> TerrainType;
}

/// Randomly picks one of the adjacent neighbor terrain types that differs from `current`.
///
/// If every neighbor shares `current`, falls back to any other terrain variant.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdjacentTypeReassigner;

impl TerrainReassigner for AdjacentTypeReassigner {
    fn reassign(
        &self,
        node: usize,
        current: TerrainType,
        terrain: &[TerrainType],
        graph: &SurfaceGraph,
        rng: &mut dyn RngCore,
    ) -> TerrainType {
        let mut candidates: Vec<TerrainType> = graph
            .neighbors(node)
            .iter()
            .map(|&(neighbor, _)| terrain[neighbor])
            .filter(|&terrain_type| terrain_type != current)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if candidates.is_empty() {
            candidates = TerrainType::ALL
                .iter()
                .copied()
                .filter(|&terrain_type| terrain_type != current)
                .collect();
        }

        choose(&candidates, rng)
    }
}

/// Reassigns invalid vertices without crossing elevation bands.
///
/// When a component is too small, vertices keep their intrinsic band (noise
/// sample sign for Perlin terrain). Above-sea vertices pick land or mountain;
/// below-sea vertices pick among below-sea types only (water today).
#[derive(Debug, Clone)]
pub struct BandPreservingReassigner {
    bands: Vec<ElevationBand>,
}

impl BandPreservingReassigner {
    /// Build a reassigner from per-vertex elevation bands frozen at assignment time.
    pub fn new(bands: Vec<ElevationBand>) -> Self {
        Self { bands }
    }

    fn band_at(&self, node: usize) -> ElevationBand {
        self.bands[node]
    }
}

impl TerrainReassigner for BandPreservingReassigner {
    fn reassign(
        &self,
        node: usize,
        current: TerrainType,
        terrain: &[TerrainType],
        graph: &SurfaceGraph,
        rng: &mut dyn RngCore,
    ) -> TerrainType {
        let node_band = self.band_at(node);

        let mut candidates: Vec<TerrainType> = graph
            .neighbors(node)
            .iter()
            .filter(|&&(neighbor, _)| self.band_at(neighbor) == node_band)
            .map(|&(neighbor, _)| terrain[neighbor])
            .filter(|&terrain_type| terrain_type != current)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if candidates.is_empty() {
            candidates = node_band
                .terrain_types()
                .iter()
                .copied()
                .filter(|&terrain_type| terrain_type != current)
                .collect();
        }

        if candidates.is_empty() {
            return current;
        }

        choose(&candidates, rng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::SurfaceGraph;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn triangle_graph() -> SurfaceGraph {
        SurfaceGraph::from_adjacency(vec![
            vec![(1, 1.0), (2, 1.0)],
            vec![(0, 1.0), (2, 1.0)],
            vec![(0, 1.0), (1, 1.0)],
        ])
    }

    #[test]
    fn above_sea_vertex_never_reassigned_to_water() {
        let graph = triangle_graph();
        let bands = vec![
            ElevationBand::AboveSeaLevel,
            ElevationBand::BelowSeaLevel,
            ElevationBand::BelowSeaLevel,
        ];
        let reassigner = BandPreservingReassigner::new(bands);
        let terrain = [
            TerrainType::Mountain,
            TerrainType::Water,
            TerrainType::Water,
        ];
        let mut rng = StdRng::seed_from_u64(1);

        let replacement =
            reassigner.reassign(0, TerrainType::Mountain, &terrain, &graph, &mut rng);

        assert_ne!(replacement, TerrainType::Water);
        assert!(matches!(
            replacement,
            TerrainType::Land
                | TerrainType::Mountain
                | TerrainType::Ice
                | TerrainType::IceMountain
        ));
    }

    #[test]
    fn prefers_same_band_neighbor_type() {
        let graph = triangle_graph();
        let bands = vec![
            ElevationBand::AboveSeaLevel,
            ElevationBand::AboveSeaLevel,
            ElevationBand::BelowSeaLevel,
        ];
        let reassigner = BandPreservingReassigner::new(bands);
        let terrain = [
            TerrainType::Mountain,
            TerrainType::Land,
            TerrainType::Water,
        ];
        let mut rng = StdRng::seed_from_u64(3);

        let replacement =
            reassigner.reassign(0, TerrainType::Mountain, &terrain, &graph, &mut rng);

        assert_eq!(replacement, TerrainType::Land);
    }
}
