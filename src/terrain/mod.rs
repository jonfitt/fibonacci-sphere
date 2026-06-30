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
    AreaKind, TerrainArea, TerrainAreaMap, area_border_edges, build_voronoi_areas, classify_site,
};
pub use assign::{
    ClusterAssigner, DEFAULT_POLAR_ICE_DEEP_WATER_RESISTANCE, DEFAULT_POLAR_ICE_LAND_RESISTANCE,
    DEFAULT_POLAR_ICE_LATITUDE_COST, DEFAULT_POLAR_ICE_MOUNTAIN_RESISTANCE,
    DEFAULT_POLAR_ICE_WATER_RESISTANCE, PerlinNoiseAssigner, PerlinNoiseConfig,
    PolarIceFloodParams, RandomAssigner, TerrainAssigner, apply_polar_ice_flood,
    flood_polar_cap_membership, polar_ice_terrain_resistance,
};
pub use borders::{AreaBorderKind, classify_area_border, is_coastline_border};
pub use generator::TerrainGenerator;
pub use polygons::{TerrainAreaPolygon, build_terrain_area_polygons};
pub use reassign::{AdjacentTypeReassigner, BandPreservingReassigner, TerrainReassigner};
pub use rng::{terrain_rng_from_godot_seed, terrain_rng_from_seed};
pub use types::{ElevationBand, TerrainMap, TerrainType};
