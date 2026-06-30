//! Terrain type assignment for lattice vertices.

mod areas;
mod assign;
mod borders;
mod generator;
mod polygons;
mod reassign;
mod rng;
mod types;

pub use areas::{
    area_border_edges, build_voronoi_areas, classify_site, AreaKind, TerrainArea, TerrainAreaMap,
};
pub use assign::{
    apply_polar_ice_flood, flood_polar_cap_membership, polar_ice_terrain_resistance,
    ClusterAssigner, PerlinNoiseAssigner, PerlinNoiseConfig, PolarIceFloodParams, RandomAssigner,
    TerrainAssigner, DEFAULT_POLAR_ICE_DEEP_WATER_RESISTANCE, DEFAULT_POLAR_ICE_LAND_RESISTANCE,
    DEFAULT_POLAR_ICE_LATITUDE_COST, DEFAULT_POLAR_ICE_MOUNTAIN_RESISTANCE,
    DEFAULT_POLAR_ICE_WATER_RESISTANCE,
};
pub use borders::{classify_area_border, is_coastline_border, AreaBorderKind};
pub use generator::TerrainGenerator;
pub use polygons::{build_terrain_area_polygons, TerrainAreaPolygon};
pub use reassign::{AdjacentTypeReassigner, BandPreservingReassigner, TerrainReassigner};
pub use rng::{terrain_rng_from_godot_seed, terrain_rng_from_seed};
pub use types::{ElevationBand, TerrainMap, TerrainType};
