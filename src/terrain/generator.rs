//! Terrain generation pipeline: Perlin assign only.

use rand::RngCore;

use crate::topology::SurfaceGraph;

use super::assign::{PerlinNoiseAssigner, PerlinNoiseConfig, TerrainAssigner};
use super::types::TerrainMap;

/// Configures how terrain is assigned for a surface graph.
pub struct TerrainGenerator {
    assigner: Box<dyn TerrainAssigner>,
}

impl TerrainGenerator {
    /// Create a generator with a custom assigner.
    pub fn new(assigner: Box<dyn TerrainAssigner>) -> Self {
        Self { assigner }
    }

    /// Generate terrain for every vertex in `graph`.
    ///
    /// Terrain types come only from the assigner (typically Perlin noise). No post-processing
    /// relabels vertices after assignment.
    pub fn generate(&self, graph: &SurfaceGraph, rng: &mut dyn RngCore) -> TerrainMap {
        let (terrain, _) = self.assigner.assign_with_elevation_bands(graph, rng);
        TerrainMap::new(terrain)
    }
}

impl TerrainGenerator {
    /// Creates a Perlin-noise terrain pipeline (the default assigner).
    pub fn perlin_noise(positions: &[[f32; 3]], config: PerlinNoiseConfig) -> Self {
        Self::new(Box::new(PerlinNoiseAssigner::new(
            positions.to_vec(),
            config,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SphereLattice;
    use crate::methods::DistributionMethod;
    use crate::terrain::assign::PerlinNoiseAssigner;
    use crate::terrain::types::TerrainType;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn perlin_generator(lattice: &SphereLattice, seed: u64) -> TerrainGenerator {
        TerrainGenerator::perlin_noise(
            &lattice.position_arrays(),
            PerlinNoiseConfig {
                seed: Some(seed as u32),
                spacing_factor: 1.2,
                north_polar_ice_distance: 0.55,
                south_polar_ice_distance: 0.55,
                mountain_threshold: 0.15,
                polar_ice_latitude_cost: 0.5,
                ..Default::default()
            },
        )
    }

    #[test]
    fn perlin_generator_includes_all_terrain_types_for_visualizer_seed() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 200, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let mut rng = StdRng::seed_from_u64(1);
        let terrain = perlin_generator(&lattice, 1).generate(&graph, &mut rng);
        let slice = terrain.as_slice();

        for terrain_type in TerrainType::ALL {
            assert!(
                slice.contains(&terrain_type),
                "missing {terrain_type:?} for seed 1"
            );
        }
    }

    #[test]
    fn generator_returns_assigner_output_unchanged() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 320, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let positions = lattice.position_arrays();

        for seed in 0..20u64 {
            let config = PerlinNoiseConfig {
                seed: Some(seed as u32),
                ..Default::default()
            };
            let mut rng = StdRng::seed_from_u64(seed);
            let assigner = PerlinNoiseAssigner::new(positions.clone(), config);
            let (assigned, _) = assigner.assign_with_elevation_bands(&graph, &mut rng);

            let mut rng = StdRng::seed_from_u64(seed);
            let generated =
                TerrainGenerator::perlin_noise(&positions, config).generate(&graph, &mut rng);

            assert_eq!(
                assigned,
                generated.as_slice(),
                "seed {seed} must not relabel terrain after Perlin assignment"
            );
        }
    }

    #[test]
    fn perlin_pipeline_preserves_water_count_across_mountain_threshold() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 200, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let positions = lattice.position_arrays();
        let mut rng = StdRng::seed_from_u64(21);

        let low = TerrainGenerator::perlin_noise(
            &positions,
            PerlinNoiseConfig {
                mountain_threshold: 0.1,
                seed: Some(8),
                ..Default::default()
            },
        )
        .generate(&graph, &mut rng);

        let high = TerrainGenerator::perlin_noise(
            &positions,
            PerlinNoiseConfig {
                mountain_threshold: 0.95,
                seed: Some(8),
                ..Default::default()
            },
        )
        .generate(&graph, &mut rng);

        let count_below_sea = |terrain: &TerrainMap| {
            terrain
                .as_slice()
                .iter()
                .filter(|&&terrain_type| {
                    matches!(terrain_type, TerrainType::Water | TerrainType::DeepWater)
                })
                .count()
        };

        assert_eq!(
            count_below_sea(&low),
            count_below_sea(&high),
            "mountain threshold must not change below-sea vertex count"
        );
    }
}
