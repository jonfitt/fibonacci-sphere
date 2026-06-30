//! Shared mesh builders for renderers (Godot, Bevy, etc.).

mod ribbon;

#[cfg(feature = "terrain")]
mod terrain_mesh;

pub use ribbon::{LineRibbonMesh, build_line_ribbon_mesh, outward_lift};
#[cfg(feature = "terrain")]
pub use terrain_mesh::{
    CombinedTerrainMesh, CombinedTerrainMeshOptions, build_combined_terrain_mesh,
    build_combined_terrain_mesh_from_lattice, coastline_segment_positions, terrain_type_rgba,
};
