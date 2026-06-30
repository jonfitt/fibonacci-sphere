//! Initial terrain assignment strategies.

mod perlin;
mod polar_flood;

use std::collections::VecDeque;

use rand::RngCore;

use crate::topology::SurfaceGraph;

use super::rng::{choose, random_index, random_terrain};
use super::types::{ElevationBand, TerrainType};

pub use perlin::{PerlinNoiseAssigner, PerlinNoiseConfig};
pub use polar_flood::{
    apply_polar_ice_flood, flood_polar_cap_membership, polar_ice_terrain_resistance,
    PolarIceFloodParams, DEFAULT_POLAR_ICE_DEEP_WATER_RESISTANCE,
    DEFAULT_POLAR_ICE_LAND_RESISTANCE, DEFAULT_POLAR_ICE_LATITUDE_COST,
    DEFAULT_POLAR_ICE_MOUNTAIN_RESISTANCE, DEFAULT_POLAR_ICE_WATER_RESISTANCE,
};

/// Assigns a terrain type to each lattice vertex.
pub trait TerrainAssigner {
    /// Produce one terrain type per vertex using the surface graph topology.
    fn assign(&self, graph: &SurfaceGraph, rng: &mut dyn RngCore) -> Vec<TerrainType>;

    /// Assign terrain and return a fixed elevation band per vertex.
    ///
    /// The default derives bands from the initial terrain types. Noise assigners
    /// override this so bands follow sample sign.
    fn assign_with_elevation_bands(
        &self,
        graph: &SurfaceGraph,
        rng: &mut dyn RngCore,
    ) -> (Vec<TerrainType>, Vec<ElevationBand>) {
        let terrain = self.assign(graph, rng);
        let bands = terrain
            .iter()
            .map(|terrain_type| terrain_type.elevation_band())
            .collect();
        (terrain, bands)
    }
}

/// Picks a random terrain type independently for each vertex.
#[derive(Debug, Clone, Copy, Default)]
pub struct RandomAssigner;

impl TerrainAssigner for RandomAssigner {
    fn assign(&self, graph: &SurfaceGraph, rng: &mut dyn RngCore) -> Vec<TerrainType> {
        (0..graph.len())
            .map(|_| random_terrain(rng))
            .collect()
    }
}

/// Grows three random continents via multi-source BFS, one per terrain type.
///
/// Produces large contiguous regions that survive the enclosure rule on closed
/// surfaces better than independent random assignment.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClusterAssigner;

impl TerrainAssigner for ClusterAssigner {
    fn assign(&self, graph: &SurfaceGraph, rng: &mut dyn RngCore) -> Vec<TerrainType> {
        let node_count = graph.len();
        if node_count == 0 {
            return Vec::new();
        }

        let seed_count = TerrainType::ALL.len().min(node_count);
        let mut seeds = Vec::with_capacity(seed_count);
        while seeds.len() < seed_count {
            let candidate = random_index(rng, node_count);
            if !seeds.contains(&candidate) {
                seeds.push(candidate);
            }
        }

        let mut terrain = vec![TerrainType::Land; node_count];
        let mut distances = vec![usize::MAX; node_count];
        let mut queue = VecDeque::new();

        for (&seed_node, &terrain_type) in seeds.iter().zip(TerrainType::ALL.iter()) {
            terrain[seed_node] = terrain_type;
            distances[seed_node] = 0;
            queue.push_back(seed_node);
        }

        while let Some(node) = queue.pop_front() {
            let next_distance = distances[node] + 1;
            for &(neighbor, _) in graph.neighbors(node) {
                if next_distance < distances[neighbor] {
                    distances[neighbor] = next_distance;
                    terrain[neighbor] = terrain[node];
                    queue.push_back(neighbor);
                } else if next_distance == distances[neighbor] && terrain[neighbor] != terrain[node]
                {
                    terrain[neighbor] = choose(&[terrain[neighbor], terrain[node]], rng);
                }
            }
        }

        for (node, distance) in distances.iter().enumerate() {
            if *distance == usize::MAX {
                terrain[node] = random_terrain(rng);
            }
        }

        terrain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn linear_graph(count: usize) -> SurfaceGraph {
        let mut adjacency = vec![Vec::new(); count];
        for index in 0..count.saturating_sub(1) {
            adjacency[index].push((index + 1, 1.0));
            adjacency[index + 1].push((index, 1.0));
        }
        SurfaceGraph::from_adjacency(adjacency)
    }

    #[test]
    fn cluster_assigner_covers_every_vertex() {
        let graph = linear_graph(12);
        let mut rng = StdRng::seed_from_u64(3);
        let terrain = ClusterAssigner.assign(&graph, &mut rng);
        assert_eq!(terrain.len(), graph.len());
        assert!(terrain.iter().all(|terrain_type| {
            TerrainType::ALL.contains(terrain_type)
        }));
    }

    #[test]
    fn cluster_assigner_includes_all_types_on_long_chain() {
        let graph = linear_graph(30);
        let mut rng = StdRng::seed_from_u64(11);
        let terrain = ClusterAssigner.assign(&graph, &mut rng);
        for terrain_type in TerrainType::ALL {
            assert!(
                terrain.contains(&terrain_type),
                "missing {terrain_type:?}"
            );
        }
    }
}
