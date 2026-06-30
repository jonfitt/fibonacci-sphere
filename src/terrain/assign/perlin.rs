//! Perlin-noise terrain assignment on the unit sphere.

use noise::{NoiseFn, Perlin};
use rand::RngCore;

use crate::SphereLattice;
use crate::topology::SurfaceGraph;

use super::super::types::{ElevationBand, TerrainType};
use super::TerrainAssigner;
use super::polar_flood::{
    DEFAULT_POLAR_ICE_DEEP_WATER_RESISTANCE, DEFAULT_POLAR_ICE_LAND_RESISTANCE,
    DEFAULT_POLAR_ICE_LATITUDE_COST, DEFAULT_POLAR_ICE_MOUNTAIN_RESISTANCE,
    DEFAULT_POLAR_ICE_WATER_RESISTANCE, PolarIceFloodParams, apply_polar_ice_flood,
};

/// Configuration for [`PerlinNoiseAssigner`].
#[derive(Debug, Clone, Copy)]
pub struct PerlinNoiseConfig {
    /// Fraction of the above-sea-level elevation range where mountains begin.
    ///
    /// Only splits land and mountain: `0.0` makes every dry vertex mountain,
    /// `1.0` makes every dry vertex land. The sea level (`0.0` on the noise
    /// sample) is not affected by this value.
    pub mountain_threshold: f64,
    /// Fraction of the below-sea-level elevation range where deep water begins.
    ///
    /// Only splits shallow and deep water: `0.0` makes almost all ocean shallow,
    /// `1.0` makes almost all ocean deep. The sea level (`0.0` on the noise
    /// sample) is not affected by this value.
    pub deep_water_threshold: f64,
    /// Scales Perlin frequency relative to average vertex spacing. Default: `1.0`.
    ///
    /// Higher values produce finer detail (more variation between neighboring vertices).
    /// Lower values produce broader continents and mountain ranges.
    pub spacing_factor: f64,
    /// Fixed Perlin seed. When `None`, a seed is taken from the generation RNG.
    pub seed: Option<u32>,
    /// Maximum angular reach of the north polar ice cap (radians). `0.0` disables it.
    pub north_polar_ice_distance: f64,
    /// Maximum angular reach of the south polar ice cap (radians). `0.0` disables it.
    pub south_polar_ice_distance: f64,
    /// Flood-fill traversal resistance on mountains (lower = spidrier arms).
    pub polar_ice_mountain_resistance: f64,
    /// Flood-fill traversal resistance on land.
    pub polar_ice_land_resistance: f64,
    /// Flood-fill traversal resistance on shallow water.
    pub polar_ice_water_resistance: f64,
    /// Flood-fill traversal resistance on deep water.
    pub polar_ice_deep_water_resistance: f64,
    /// Added cost per unit geodesic edge during flood fill (higher = rounder caps).
    pub polar_ice_latitude_cost: f64,
}

impl Default for PerlinNoiseConfig {
    fn default() -> Self {
        Self {
            mountain_threshold: 0.7,
            deep_water_threshold: 0.5,
            spacing_factor: 1.0,
            seed: None,
            north_polar_ice_distance: 0.0,
            south_polar_ice_distance: 0.0,
            polar_ice_mountain_resistance: DEFAULT_POLAR_ICE_MOUNTAIN_RESISTANCE,
            polar_ice_land_resistance: DEFAULT_POLAR_ICE_LAND_RESISTANCE,
            polar_ice_water_resistance: DEFAULT_POLAR_ICE_WATER_RESISTANCE,
            polar_ice_deep_water_resistance: DEFAULT_POLAR_ICE_DEEP_WATER_RESISTANCE,
            polar_ice_latitude_cost: DEFAULT_POLAR_ICE_LATITUDE_COST,
        }
    }
}

impl PerlinNoiseConfig {
    /// Polar ice flood-fill parameters derived from this config.
    pub fn polar_flood_params(self) -> PolarIceFloodParams {
        PolarIceFloodParams {
            mountain_resistance: self.polar_ice_mountain_resistance,
            land_resistance: self.polar_ice_land_resistance,
            water_resistance: self.polar_ice_water_resistance,
            deep_water_resistance: self.polar_ice_deep_water_resistance,
            latitude_cost: self.polar_ice_latitude_cost,
        }
    }
}

/// Assigns terrain from 3D Perlin noise sampled on normalized sphere positions.
///
/// Noise is sampled in roughly `[-1, 1]` and classified as temperate landforms first.
/// Optional polar ice caps are grown afterward by least-cost flood fill from each
/// pole across the surface graph. Lower traversal resistance on mountains and higher
/// resistance on water yield spidery caps; a higher [`PerlinNoiseConfig::polar_ice_latitude_cost`]
/// pushes caps toward a round boundary.
///
/// The mountain cutoff is derived only from samples at or above sea level:
/// `min_positive + mountain_threshold * (max_positive - min_positive)`.
/// The deep-water cutoff is derived only from samples below sea level:
/// `min_negative + deep_water_threshold * (max_negative - min_negative)`.
/// Changing either threshold therefore redistributes types within that elevation
/// band without moving coastlines.
#[derive(Debug, Clone)]
pub struct PerlinNoiseAssigner {
    positions: Vec<[f32; 3]>,
    config: PerlinNoiseConfig,
}

impl PerlinNoiseAssigner {
    /// Create an assigner from explicit vertex positions.
    pub fn new(positions: Vec<[f32; 3]>, config: PerlinNoiseConfig) -> Self {
        Self { positions, config }
    }

    /// Create an assigner from a generated lattice.
    pub fn from_lattice(lattice: &SphereLattice, config: PerlinNoiseConfig) -> Self {
        Self::new(lattice.position_arrays(), config)
    }

    fn noise_frequency(&self, graph: &SurfaceGraph) -> f64 {
        let average_spacing = average_neighbor_spacing(graph);
        if average_spacing <= f64::EPSILON {
            return self.config.spacing_factor;
        }
        self.config.spacing_factor / average_spacing
    }

    fn sample_noise(&self, position: [f32; 3], perlin: &Perlin, frequency: f64) -> f64 {
        let [x, y, z] = position;
        let length = f64::from((x * x + y * y + z * z).sqrt());
        if length <= f64::EPSILON {
            return 0.0;
        }

        let ux = f64::from(x) / length;
        let uy = f64::from(y) / length;
        let uz = f64::from(z) / length;
        perlin.get([ux * frequency, uy * frequency, uz * frequency])
    }

    fn classify(&self, sample: f64, mountain_cutoff: f64, deep_water_cutoff: f64) -> TerrainType {
        const SEA_LEVEL: f64 = 0.0;
        if sample < SEA_LEVEL {
            if sample < deep_water_cutoff {
                TerrainType::DeepWater
            } else {
                TerrainType::Water
            }
        } else if sample < mountain_cutoff {
            TerrainType::Land
        } else {
            TerrainType::Mountain
        }
    }

    fn build_assignment(
        &self,
        graph: &SurfaceGraph,
        rng: &mut dyn RngCore,
    ) -> (Vec<f64>, Vec<TerrainType>) {
        let node_count = graph.len();
        if node_count == 0 {
            return (Vec::new(), Vec::new());
        }

        debug_assert_eq!(
            self.positions.len(),
            node_count,
            "PerlinNoiseAssigner positions must match graph vertex count"
        );

        let seed = self.config.seed.unwrap_or_else(|| rng.next_u32());
        let perlin = Perlin::new(seed);
        let frequency = self.noise_frequency(graph);

        let samples: Vec<f64> = self
            .positions
            .iter()
            .take(node_count)
            .map(|&position| self.sample_noise(position, &perlin, frequency))
            .collect();
        let mountain_cutoff =
            mountain_cutoff_from_samples(&samples, self.config.mountain_threshold);
        let deep_water_cutoff =
            deep_water_cutoff_from_samples(&samples, self.config.deep_water_threshold);

        let temperate: Vec<TerrainType> = samples
            .iter()
            .copied()
            .map(|sample| self.classify(sample, mountain_cutoff, deep_water_cutoff))
            .collect();

        let terrain = if self.config.north_polar_ice_distance > 0.0
            || self.config.south_polar_ice_distance > 0.0
        {
            apply_polar_ice_flood(
                &temperate,
                &self.positions[..node_count],
                graph,
                self.config.north_polar_ice_distance,
                self.config.south_polar_ice_distance,
                self.config.polar_flood_params(),
            )
        } else {
            temperate
        };

        (samples, terrain)
    }
}

impl TerrainAssigner for PerlinNoiseAssigner {
    fn assign(&self, graph: &SurfaceGraph, rng: &mut dyn RngCore) -> Vec<TerrainType> {
        self.build_assignment(graph, rng).1
    }

    fn assign_with_elevation_bands(
        &self,
        graph: &SurfaceGraph,
        rng: &mut dyn RngCore,
    ) -> (Vec<TerrainType>, Vec<ElevationBand>) {
        let (samples, terrain) = self.build_assignment(graph, rng);
        let bands = samples
            .iter()
            .map(|&sample| ElevationBand::from_sample(sample))
            .collect();
        (terrain, bands)
    }
}

const SEA_LEVEL: f64 = 0.0;

fn mountain_cutoff_from_samples(samples: &[f64], mountain_threshold: f64) -> f64 {
    let mut min_positive = f64::INFINITY;
    let mut max_positive = f64::NEG_INFINITY;

    for &sample in samples {
        if sample >= SEA_LEVEL {
            min_positive = min_positive.min(sample);
            max_positive = max_positive.max(sample);
        }
    }

    if !min_positive.is_finite() || !max_positive.is_finite() || max_positive <= min_positive {
        return f64::INFINITY;
    }

    let threshold = mountain_threshold.clamp(0.0, 1.0);
    min_positive + threshold * (max_positive - min_positive)
}

fn deep_water_cutoff_from_samples(samples: &[f64], deep_water_threshold: f64) -> f64 {
    let mut min_negative = f64::INFINITY;
    let mut max_negative = f64::NEG_INFINITY;

    for &sample in samples {
        if sample < SEA_LEVEL {
            min_negative = min_negative.min(sample);
            max_negative = max_negative.max(sample);
        }
    }

    if !min_negative.is_finite() || !max_negative.is_finite() || max_negative <= min_negative {
        return f64::NEG_INFINITY;
    }

    let threshold = deep_water_threshold.clamp(0.0, 1.0);
    min_negative + threshold * (max_negative - min_negative)
}

/// Mean geodesic edge length over the Delaunay mesh (each undirected edge counted once).
pub(crate) fn average_neighbor_spacing(graph: &SurfaceGraph) -> f64 {
    if graph.is_empty() {
        return 1.0;
    }

    let mut total = 0.0;
    let mut edge_count = 0usize;
    for node in 0..graph.len() {
        for &(neighbor, weight) in graph.neighbors(node) {
            if neighbor > node {
                total += weight;
                edge_count += 1;
            }
        }
    }

    if edge_count == 0 {
        1.0
    } else {
        total / edge_count as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::DistributionMethod;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn classifies_noise_bands() {
        let assigner =
            PerlinNoiseAssigner::new(vec![[1.0, 0.0, 0.0]], PerlinNoiseConfig::default());

        assert_eq!(assigner.classify(-0.6, 0.5, -0.3), TerrainType::DeepWater);
        assert_eq!(assigner.classify(-0.1, 0.5, -0.3), TerrainType::Water);
        assert_eq!(assigner.classify(0.2, 0.5, -0.3), TerrainType::Land);
        assert_eq!(assigner.classify(0.8, 0.5, -0.3), TerrainType::Mountain);
    }

    #[test]
    fn deep_water_threshold_only_splits_shallow_and_deep_water() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 160, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let mut rng = StdRng::seed_from_u64(11);

        let low_threshold = PerlinNoiseAssigner::from_lattice(
            &lattice,
            PerlinNoiseConfig {
                deep_water_threshold: 0.2,
                seed: Some(5),
                ..Default::default()
            },
        )
        .assign(&graph, &mut rng);

        let high_threshold = PerlinNoiseAssigner::from_lattice(
            &lattice,
            PerlinNoiseConfig {
                deep_water_threshold: 0.9,
                seed: Some(5),
                ..Default::default()
            },
        )
        .assign(&graph, &mut rng);

        let count_below_sea = |terrain: &[TerrainType]| {
            terrain
                .iter()
                .filter(|&&terrain_type| {
                    matches!(terrain_type, TerrainType::Water | TerrainType::DeepWater)
                })
                .count()
        };

        assert_eq!(
            count_below_sea(&low_threshold),
            count_below_sea(&high_threshold),
            "below-sea vertex count must not change with deep-water threshold"
        );
        assert!(
            low_threshold
                .iter()
                .filter(|&&terrain_type| terrain_type == TerrainType::DeepWater)
                .count()
                < high_threshold
                    .iter()
                    .filter(|&&terrain_type| terrain_type == TerrainType::DeepWater)
                    .count(),
            "higher deep-water threshold should classify more ocean as deep"
        );
    }

    #[test]
    fn mountain_threshold_only_splits_land_and_mountain() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 160, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let mut rng = StdRng::seed_from_u64(11);

        let low_threshold = PerlinNoiseAssigner::from_lattice(
            &lattice,
            PerlinNoiseConfig {
                mountain_threshold: 0.2,
                seed: Some(5),
                ..Default::default()
            },
        )
        .assign(&graph, &mut rng);

        let high_threshold = PerlinNoiseAssigner::from_lattice(
            &lattice,
            PerlinNoiseConfig {
                mountain_threshold: 0.9,
                seed: Some(5),
                ..Default::default()
            },
        )
        .assign(&graph, &mut rng);

        let count_below_sea = |terrain: &[TerrainType]| {
            terrain
                .iter()
                .filter(|&&terrain_type| {
                    matches!(terrain_type, TerrainType::Water | TerrainType::DeepWater)
                })
                .count()
        };

        assert_eq!(
            count_below_sea(&low_threshold),
            count_below_sea(&high_threshold),
            "mountain threshold must not change below-sea vertex count"
        );
        assert!(
            low_threshold
                .iter()
                .filter(|&&terrain_type| terrain_type == TerrainType::Mountain)
                .count()
                > high_threshold
                    .iter()
                    .filter(|&&terrain_type| terrain_type == TerrainType::Mountain)
                    .count(),
            "lower threshold should produce more mountains, not more water"
        );
    }

    #[test]
    fn frequency_scales_with_vertex_density() {
        let sparse = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 80, 1.0)
            .unwrap()
            .surface_graph();
        let dense = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 320, 1.0)
            .unwrap()
            .surface_graph();

        let assigner = PerlinNoiseAssigner::new(vec![], PerlinNoiseConfig::default());
        let sparse_frequency = assigner.noise_frequency(&sparse);
        let dense_frequency = assigner.noise_frequency(&dense);

        assert!(
            dense_frequency > sparse_frequency,
            "denser mesh should use higher noise frequency (sparse={sparse_frequency}, dense={dense_frequency})"
        );
    }

    #[test]
    fn frequency_scales_inversely_with_radius() {
        let radius_ratio = 4.0;
        let small = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 120, 1.0)
            .unwrap()
            .surface_graph();
        let large =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 120, radius_ratio)
                .unwrap()
                .surface_graph();

        let assigner = PerlinNoiseAssigner::new(vec![], PerlinNoiseConfig::default());
        let small_frequency = assigner.noise_frequency(&small);
        let large_frequency = assigner.noise_frequency(&large);

        let observed_ratio = small_frequency / large_frequency;
        assert!(
            (observed_ratio - radius_ratio).abs() < 1e-3,
            "expected frequency ratio {radius_ratio}, got {observed_ratio}"
        );
    }

    #[test]
    fn polar_flood_maps_mountains_to_ice_mountain_and_wet_to_ice() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 120, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let assigner = PerlinNoiseAssigner::from_lattice(
            &lattice,
            PerlinNoiseConfig {
                seed: Some(9),
                north_polar_ice_distance: 0.55,
                south_polar_ice_distance: 0.55,
                mountain_threshold: 0.15,
                polar_ice_latitude_cost: 0.5,
                ..Default::default()
            },
        );
        let mut rng = StdRng::seed_from_u64(3);
        let terrain = assigner.assign(&graph, &mut rng);

        assert!(terrain.contains(&TerrainType::Ice));
        assert!(terrain.contains(&TerrainType::IceMountain));
        assert!(
            terrain.iter().any(|&terrain_type| matches!(
                terrain_type,
                TerrainType::Water | TerrainType::DeepWater
            )),
            "flood fill should leave temperate ocean outside the cap"
        );
    }

    #[test]
    fn polar_flood_fill_has_no_interior_water_lakes() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 240, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let assigner = PerlinNoiseAssigner::from_lattice(
            &lattice,
            PerlinNoiseConfig {
                seed: Some(21),
                north_polar_ice_distance: 0.45,
                south_polar_ice_distance: 0.45,
                polar_ice_latitude_cost: 1.0,
                ..Default::default()
            },
        );
        let mut rng = StdRng::seed_from_u64(2);
        let terrain = assigner.assign(&graph, &mut rng);

        for node in 0..graph.len() {
            if !matches!(terrain[node], TerrainType::Water | TerrainType::DeepWater) {
                continue;
            }
            let surrounded_by_ice = graph.neighbors(node).iter().all(|&(neighbor, _)| {
                matches!(
                    terrain[neighbor],
                    TerrainType::Ice | TerrainType::IceMountain
                )
            });
            assert!(
                !surrounded_by_ice,
                "polar flood fill should not leave enclosed water at node {node}"
            );
        }
    }

    #[test]
    fn polar_caps_produce_ice_types_on_sphere() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 200, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let assigner = PerlinNoiseAssigner::from_lattice(
            &lattice,
            PerlinNoiseConfig {
                spacing_factor: 1.2,
                seed: Some(42),
                north_polar_ice_distance: 0.55,
                south_polar_ice_distance: 0.55,
                mountain_threshold: 0.15,
                polar_ice_latitude_cost: 0.5,
                ..Default::default()
            },
        );
        let mut rng = StdRng::seed_from_u64(1);
        let terrain = assigner.assign(&graph, &mut rng);

        assert!(terrain.contains(&TerrainType::Ice));
        assert!(terrain.contains(&TerrainType::IceMountain));
    }

    #[test]
    fn includes_all_terrain_types_on_sphere() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 200, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let assigner = PerlinNoiseAssigner::from_lattice(
            &lattice,
            PerlinNoiseConfig {
                spacing_factor: 1.2,
                seed: Some(42),
                north_polar_ice_distance: 0.55,
                south_polar_ice_distance: 0.55,
                mountain_threshold: 0.15,
                polar_ice_latitude_cost: 0.5,
                ..Default::default()
            },
        );
        let mut rng = StdRng::seed_from_u64(1);
        let terrain = assigner.assign(&graph, &mut rng);

        for terrain_type in TerrainType::ALL {
            assert!(terrain.contains(&terrain_type), "missing {terrain_type:?}");
        }
    }

    #[test]
    fn higher_spacing_factor_increases_type_transitions() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 180, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let positions = lattice.position_arrays();
        let mut rng = StdRng::seed_from_u64(9);

        let smooth = PerlinNoiseAssigner::new(
            positions.clone(),
            PerlinNoiseConfig {
                spacing_factor: 0.35,
                seed: Some(7),
                ..Default::default()
            },
        )
        .assign(&graph, &mut rng);

        let detailed = PerlinNoiseAssigner::new(
            positions,
            PerlinNoiseConfig {
                spacing_factor: 2.5,
                seed: Some(7),
                ..Default::default()
            },
        )
        .assign(&graph, &mut rng);

        assert!(
            count_type_transitions(&detailed, &graph) > count_type_transitions(&smooth, &graph),
            "finer noise should produce more terrain boundaries"
        );
    }

    fn count_type_transitions(terrain: &[TerrainType], graph: &SurfaceGraph) -> usize {
        let mut transitions = 0usize;
        for node in 0..graph.len() {
            for &(neighbor, _) in graph.neighbors(node) {
                if neighbor > node && terrain[node] != terrain[neighbor] {
                    transitions += 1;
                }
            }
        }
        transitions
    }
}
