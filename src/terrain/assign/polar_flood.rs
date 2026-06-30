//! Polar ice caps grown by least-cost flood fill from each pole on the surface graph.

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use crate::geography::{angular_distance_to_north_pole, angular_distance_to_south_pole};
use crate::topology::SurfaceGraph;

use super::super::types::TerrainType;

#[derive(Eq, PartialEq)]
struct FloodQueueEntry {
    cost_bits: u64,
    node: usize,
}

impl Ord for FloodQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cost_bits
            .cmp(&other.cost_bits)
            .then(self.node.cmp(&other.node))
    }
}

impl PartialOrd for FloodQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn cost_to_queue_bits(cost: f64) -> u64 {
    cost.to_bits()
}

/// Resistance and latitude costs for polar ice flood fill.
#[derive(Debug, Clone, Copy)]
pub struct PolarIceFloodParams {
    /// Traversal cost multiplier on mountain vertices (low = spidery mountain arms).
    pub mountain_resistance: f64,
    /// Traversal cost multiplier on land vertices.
    pub land_resistance: f64,
    /// Traversal cost multiplier on shallow-water vertices.
    pub water_resistance: f64,
    /// Traversal cost multiplier on deep-water vertices.
    pub deep_water_resistance: f64,
    /// Added cost per unit geodesic edge length (high = rounder caps).
    pub latitude_cost: f64,
}

/// Default mountain traversal resistance for polar flood fill.
pub const DEFAULT_POLAR_ICE_MOUNTAIN_RESISTANCE: f64 = 0.25;
/// Default land traversal resistance for polar flood fill.
pub const DEFAULT_POLAR_ICE_LAND_RESISTANCE: f64 = 1.0;
/// Default shallow-water traversal resistance for polar flood fill.
pub const DEFAULT_POLAR_ICE_WATER_RESISTANCE: f64 = 2.5;
/// Default deep-water traversal resistance for polar flood fill.
pub const DEFAULT_POLAR_ICE_DEEP_WATER_RESISTANCE: f64 = 5.0;
/// Default latitude cost for polar flood fill.
pub const DEFAULT_POLAR_ICE_LATITUDE_COST: f64 = 2.0;

impl Default for PolarIceFloodParams {
    fn default() -> Self {
        Self {
            mountain_resistance: DEFAULT_POLAR_ICE_MOUNTAIN_RESISTANCE,
            land_resistance: DEFAULT_POLAR_ICE_LAND_RESISTANCE,
            water_resistance: DEFAULT_POLAR_ICE_WATER_RESISTANCE,
            deep_water_resistance: DEFAULT_POLAR_ICE_DEEP_WATER_RESISTANCE,
            latitude_cost: DEFAULT_POLAR_ICE_LATITUDE_COST,
        }
    }
}

impl PolarIceFloodParams {
    /// Land-equivalent per-radian budget used to convert cap distance into a flood cost limit.
    ///
    /// Latitude cost is excluded so raising it makes each step more expensive without
    /// also increasing the total budget.
    pub fn reference_cost_per_radian(self) -> f64 {
        self.land_resistance.max(0.05)
    }

    /// Maximum cumulative flood cost for a cap with the given angular reach.
    pub fn max_flood_cost_for_distance(self, max_pole_distance: f64) -> f64 {
        if max_pole_distance <= 0.0 {
            return 0.0;
        }
        max_pole_distance * self.reference_cost_per_radian()
    }
}

/// Returns the temperate terrain traversal resistance for polar flood fill.
pub fn polar_ice_terrain_resistance(terrain: TerrainType, params: PolarIceFloodParams) -> f64 {
    match terrain {
        TerrainType::Mountain => params.mountain_resistance,
        TerrainType::Land => params.land_resistance,
        TerrainType::Water => params.water_resistance,
        TerrainType::DeepWater => params.deep_water_resistance,
        TerrainType::Ice | TerrainType::IceMountain => params.land_resistance,
    }
    .max(0.01)
}

/// Marks vertices reached by least-cost flood fill from the nearest vertex to a pole.
///
/// `temperate` must be the pre-polar terrain assignment. A vertex is marked when its
/// cumulative cost from the pole seed is at most [`PolarIceFloodParams::max_flood_cost_for_distance`]
/// and its angular distance to the pole is within `max_pole_distance`.
pub fn flood_polar_cap_membership(
    graph: &SurfaceGraph,
    positions: &[[f32; 3]],
    temperate: &[TerrainType],
    south: bool,
    max_pole_distance: f64,
    params: PolarIceFloodParams,
) -> Vec<bool> {
    let node_count = graph.len();
    let mut membership = vec![false; node_count];
    if node_count == 0 || max_pole_distance <= 0.0 {
        return membership;
    }

    debug_assert_eq!(
        positions.len(),
        node_count,
        "polar flood positions must match graph vertex count"
    );
    debug_assert_eq!(
        temperate.len(),
        node_count,
        "polar flood temperate terrain must match graph vertex count"
    );

    let pole_distance = |position: [f32; 3]| {
        if south {
            angular_distance_to_south_pole(position)
        } else {
            angular_distance_to_north_pole(position)
        }
    };

    let Some(seed) = positions
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            pole_distance(**a)
                .partial_cmp(&pole_distance(**b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
    else {
        return membership;
    };

    let max_cost = params.max_flood_cost_for_distance(max_pole_distance);
    if max_cost <= 0.0 {
        return membership;
    }

    let mut best_cost = vec![f64::INFINITY; node_count];
    let mut heap = BinaryHeap::new();
    best_cost[seed] = 0.0;
    heap.push(Reverse(FloodQueueEntry {
        cost_bits: cost_to_queue_bits(0.0),
        node: seed,
    }));

    while let Some(Reverse(FloodQueueEntry { cost_bits, node })) = heap.pop() {
        let cost = f64::from_bits(cost_bits);
        if cost > best_cost[node] + 1e-9 {
            continue;
        }
        if cost > max_cost + 1e-9 {
            continue;
        }

        membership[node] = true;

        for &(neighbor, edge_weight) in graph.neighbors(node) {
            if edge_weight <= f64::EPSILON {
                continue;
            }

            let neighbor_pole_distance = pole_distance(positions[neighbor]);
            if neighbor_pole_distance > max_pole_distance + 1e-9 {
                continue;
            }

            let step = edge_weight
                * (polar_ice_terrain_resistance(temperate[neighbor], params) + params.latitude_cost);
            let next_cost = cost + step;
            if next_cost + 1e-9 < best_cost[neighbor] && next_cost <= max_cost + 1e-9 {
                best_cost[neighbor] = next_cost;
                heap.push(Reverse(FloodQueueEntry {
                    cost_bits: cost_to_queue_bits(next_cost),
                    node: neighbor,
                }));
            }
        }
    }

    membership
}

/// Applies polar ice types to temperate terrain using north/south flood fills.
pub fn apply_polar_ice_flood(
    temperate: &[TerrainType],
    positions: &[[f32; 3]],
    graph: &SurfaceGraph,
    north_max_distance: f64,
    south_max_distance: f64,
    params: PolarIceFloodParams,
) -> Vec<TerrainType> {
    let node_count = temperate.len();
    if node_count == 0 {
        return Vec::new();
    }

    let north = flood_polar_cap_membership(
        graph,
        positions,
        temperate,
        false,
        north_max_distance,
        params,
    );
    let south = flood_polar_cap_membership(
        graph,
        positions,
        temperate,
        true,
        south_max_distance,
        params,
    );

    temperate
        .iter()
        .enumerate()
        .map(|(index, &terrain)| {
            if !north[index] && !south[index] {
                return terrain;
            }
            match terrain {
                TerrainType::Mountain => TerrainType::IceMountain,
                _ => TerrainType::Ice,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::DistributionMethod;
    use crate::SphereLattice;

    #[test]
    fn deep_water_costs_more_than_shallow_water() {
        let params = PolarIceFloodParams::default();
        assert!(
            polar_ice_terrain_resistance(TerrainType::DeepWater, params)
                > polar_ice_terrain_resistance(TerrainType::Water, params)
        );
        assert!(
            polar_ice_terrain_resistance(TerrainType::Water, params)
                > polar_ice_terrain_resistance(TerrainType::Land, params)
        );
        assert!(
            polar_ice_terrain_resistance(TerrainType::Land, params)
                > polar_ice_terrain_resistance(TerrainType::Mountain, params)
        );
    }

    #[test]
    fn high_latitude_cost_produces_rounder_cap_than_low() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 220, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let positions = lattice.position_arrays();
        let temperate = vec![TerrainType::Land; graph.len()];

        let tight = flood_polar_cap_membership(
            &graph,
            &positions,
            &temperate,
            false,
            0.35,
            PolarIceFloodParams {
                latitude_cost: 8.0,
                ..Default::default()
            },
        );
        let loose = flood_polar_cap_membership(
            &graph,
            &positions,
            &temperate,
            false,
            0.35,
            PolarIceFloodParams {
                latitude_cost: 0.2,
                ..Default::default()
            },
        );

        let count = |mask: &[bool]| mask.iter().filter(|&&v| v).count();
        assert!(
            count(&tight) < count(&loose),
            "higher latitude cost should keep the uniform-land cap smaller"
        );
    }

    #[test]
    fn flood_prefers_mountain_corridors_over_water() {
        let params = PolarIceFloodParams {
            latitude_cost: 0.15,
            mountain_resistance: 0.1,
            water_resistance: 12.0,
            deep_water_resistance: 20.0,
            ..Default::default()
        };

        let mountain_step = polar_ice_terrain_resistance(TerrainType::Mountain, params);
        let water_step = polar_ice_terrain_resistance(TerrainType::Water, params);
        assert!(mountain_step < water_step);

        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 260, 1.0).unwrap();
        let graph = lattice.surface_graph();
        let positions = lattice.position_arrays();
        let mut temperate = vec![TerrainType::Land; graph.len()];

        let seed = positions
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                angular_distance_to_north_pole(**a)
                    .partial_cmp(&angular_distance_to_north_pole(**b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
            .unwrap();

        let mut mountain_chain = vec![seed];
        let mut current = seed;
        for _ in 0..6 {
            let Some(&(next, _)) = graph
                .neighbors(current)
                .iter()
                .filter(|&&(neighbor, _)| {
                    angular_distance_to_north_pole(positions[neighbor])
                        >= angular_distance_to_north_pole(positions[current])
                })
                .max_by(|a, b| {
                    angular_distance_to_north_pole(positions[a.0])
                        .partial_cmp(&angular_distance_to_north_pole(positions[b.0]))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            else {
                break;
            };
            mountain_chain.push(next);
            current = next;
        }

        if mountain_chain.len() < 3 {
            return;
        }

        for &index in &mountain_chain {
            temperate[index] = TerrainType::Mountain;
        }

        let spidery = flood_polar_cap_membership(
            &graph,
            &positions,
            &temperate,
            false,
            0.45,
            params,
        );
        let circular = flood_polar_cap_membership(
            &graph,
            &positions,
            &temperate,
            false,
            0.45,
            PolarIceFloodParams {
                latitude_cost: 10.0,
                ..params
            },
        );

        assert!(
            count_ice(&spidery) >= count_ice(&circular),
            "lower latitude cost should reach at least as many vertices"
        );
    }

    fn count_ice(mask: &[bool]) -> usize {
        mask.iter().filter(|&&v| v).count()
    }

    #[test]
    fn apply_polar_ice_flood_maps_mountains_to_ice_mountain() {
        let terrain = apply_polar_ice_flood(
            &[TerrainType::Mountain, TerrainType::Water, TerrainType::Land],
            &[[0.0, 1.0, 0.0], [0.0, 0.9, 0.1], [1.0, 0.0, 0.0]],
            &SurfaceGraph::from_adjacency(vec![
                vec![(1, 0.1)],
                vec![(0, 0.1), (2, 1.0)],
                vec![(1, 1.0)],
            ]),
            0.5,
            0.0,
            PolarIceFloodParams::default(),
        );
        assert_eq!(terrain[0], TerrainType::IceMountain);
        assert_eq!(terrain[1], TerrainType::Ice);
    }
}
