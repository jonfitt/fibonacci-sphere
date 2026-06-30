//! Fibonacci sphere point distribution for game engines.
//!
//! Generates evenly distributed sample points on a sphere using several
//! Fibonacci-lattice variants. Coordinates are **Y-up, right-handed**,
//! compatible with Godot 4.
//!
//! # Crate layout
//!
//! - [`methods`] — distribution algorithms and method metadata
//! - [`point`] — [`SpherePoint`] and golden-ratio constants
//! - [`SphereLattice`] — generated point set and flat position export
//! - [`neighbors`] — closest-neighbor queries for analysis
//! - [`topology`] — spherical Delaunay connectivity (wireframe, routing) and Voronoi cells
//! - [`terrain`] (feature `terrain`) — Perlin terrain and Voronoi areas
//! - [`render`] — combined terrain meshes, coastline segments, line ribbons
//!
//! # Example
//!
//! ```
//! use fibonacci_sphere::{DistributionMethod, SphereLattice};
//!
//! let lattice = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 100, 1.0).unwrap();
//! assert_eq!(lattice.len(), 100);
//! let flat = lattice.positions_flat();
//! assert_eq!(flat.len(), 300);
//! ```

#![deny(missing_docs)]

/// Error types for lattice generation failures.
pub mod error;
/// Sphere point distribution algorithms.
pub mod methods;
/// Core point types and constants.
pub mod point;

mod geography;
mod lattice;
mod neighbors;
mod validation;

/// Shared mesh builders for renderers.
pub mod render;

#[cfg(test)]
mod test_helpers;

/// Surface topology helpers for sphere lattices.
pub mod topology;

/// Terrain type assignment for lattice vertices (requires `terrain` feature).
#[cfg(feature = "terrain")]
pub mod terrain;

pub use error::SphereError;
pub use geography::{
    angular_distance_to_equator, angular_distance_to_north_pole, angular_distance_to_south_pole,
    polar_cap_circle_segments, vertices_within_equatorial_distance,
    vertices_within_north_polar_distance, vertices_within_south_polar_distance,
};
pub use lattice::SphereLattice;
pub use methods::{Distribution, DistributionMethod, MethodInfo, OptimizationGoal};
pub use neighbors::{DistanceBin, Neighbor, NeighborQuery};
pub use point::{GOLDEN_RATIO, SpherePoint};
#[cfg(feature = "terrain")]
pub use render::{
    CombinedTerrainMesh, CombinedTerrainMeshOptions, build_combined_terrain_mesh,
    build_combined_terrain_mesh_from_lattice, coastline_segment_positions, terrain_type_rgba,
};
pub use render::{LineRibbonMesh, build_line_ribbon_mesh, outward_lift};
#[cfg(feature = "terrain")]
pub use terrain::{
    AdjacentTypeReassigner, AreaBorderKind, AreaKind, BandPreservingReassigner, ClusterAssigner,
    DEFAULT_POLAR_ICE_DEEP_WATER_RESISTANCE, DEFAULT_POLAR_ICE_LAND_RESISTANCE,
    DEFAULT_POLAR_ICE_LATITUDE_COST, DEFAULT_POLAR_ICE_MOUNTAIN_RESISTANCE,
    DEFAULT_POLAR_ICE_WATER_RESISTANCE, ElevationBand, PerlinNoiseAssigner, PerlinNoiseConfig,
    PolarIceFloodParams, RandomAssigner, TerrainArea, TerrainAreaMap, TerrainAreaPolygon,
    TerrainAssigner, TerrainGenerator, TerrainMap, TerrainReassigner, TerrainType,
    apply_polar_ice_flood, area_border_edges, build_terrain_area_polygons, build_voronoi_areas,
    classify_area_border, classify_site, flood_polar_cap_membership, is_coastline_border,
    polar_ice_terrain_resistance, terrain_rng_from_godot_seed, terrain_rng_from_seed,
};
pub use topology::{
    SphericalMesh, SurfaceGraph, SurfacePath, VoronoiCell, VoronoiFanMesh, VoronoiFanMeshOptions,
    WireframeEdge, build_voronoi_cell_fan_mesh, geodesic_distance, spherical_delaunay_edges,
    spherical_delaunay_mesh, spherical_delaunay_triangles, spherical_voronoi_border_segments,
    spherical_voronoi_cells, voronoi_cell_fan_apex,
};
