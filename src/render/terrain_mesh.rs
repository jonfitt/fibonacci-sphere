//! Combined terrain area meshes and coastline segment extraction.

use std::collections::HashSet;

use crate::topology::{build_voronoi_cell_fan_mesh, voronoi_cell_fan_apex, VoronoiFanMeshOptions};
use crate::SphereLattice;

use crate::terrain::{AreaBorderKind, TerrainAreaPolygon, TerrainMap, TerrainType};

/// RGBA vertex color for a terrain type (matches Godot demo and Bevy visualizer).
pub fn terrain_type_rgba(terrain_type: TerrainType) -> [f32; 4] {
    match terrain_type {
        TerrainType::Land => [0.18, 0.88, 0.24, 1.0],
        TerrainType::Water => [0.22, 0.52, 0.95, 1.0],
        TerrainType::DeepWater => [0.04, 0.12, 0.45, 1.0],
        TerrainType::Mountain => [0.85, 0.22, 0.18, 1.0],
        TerrainType::Ice => [0.82, 0.92, 0.98, 1.0],
        TerrainType::IceMountain => [0.62, 0.78, 0.92, 1.0],
    }
}

/// Options for [`build_combined_terrain_mesh`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CombinedTerrainMeshOptions {
    /// Fan triangulation options applied to every Voronoi cell.
    pub fan_mesh: VoronoiFanMeshOptions,
}

/// Vertex-colored triangle mesh covering all terrain area polygons.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CombinedTerrainMesh {
    /// Triangle vertices in world space.
    pub vertices: Vec<[f32; 3]>,
    /// Per-vertex RGBA colors.
    pub colors: Vec<[f32; 4]>,
    /// Outward-pointing unit normals.
    pub normals: Vec<[f32; 3]>,
    /// Triangle corner indices into [`Self::vertices`].
    pub indices: Vec<u32>,
}

/// Build a combined terrain mesh from precomputed area polygons.
pub fn build_combined_terrain_mesh(
    polygons: &[TerrainAreaPolygon],
    positions: &[[f32; 3]],
    radius: f32,
    options: CombinedTerrainMeshOptions,
) -> CombinedTerrainMesh {
    let mut mesh = CombinedTerrainMesh::default();

    for polygon in polygons {
        if polygon.boundary.len() < 3 {
            continue;
        }

        let apex_dir = voronoi_cell_fan_apex(polygon.site_index, positions);
        let fan_apex = [
            apex_dir[0] * radius,
            apex_dir[1] * radius,
            apex_dir[2] * radius,
        ];

        let Some(cell_mesh) =
            build_voronoi_cell_fan_mesh(fan_apex, &polygon.boundary, radius, options.fan_mesh)
        else {
            continue;
        };

        let color = terrain_type_rgba(polygon.terrain_type);
        let base_index = mesh.vertices.len() as u32;
        for vertex in cell_mesh.vertices {
            mesh.vertices.push(vertex);
            mesh.colors.push(color);
            mesh.normals.push(outward_normal(vertex));
        }
        for triangle in cell_mesh.triangles {
            mesh.indices.extend(triangle.map(|index| base_index + index as u32));
        }
    }

    mesh
}

/// Build a combined terrain mesh from a lattice and terrain map.
pub fn build_combined_terrain_mesh_from_lattice(
    lattice: &SphereLattice,
    terrain: &TerrainMap,
    options: CombinedTerrainMeshOptions,
) -> CombinedTerrainMesh {
    let positions = lattice.position_arrays();
    let polygons = lattice.terrain_area_polygons(terrain);
    build_combined_terrain_mesh(
        &polygons,
        &positions,
        lattice.radius() as f32,
        options,
    )
}

/// Undirected coastline segment endpoints (`[start, end, ...]`) deduplicated by site pair.
pub fn coastline_segment_positions(polygons: &[TerrainAreaPolygon]) -> Vec<[f32; 3]> {
    let mut seen = HashSet::new();
    let mut segments = Vec::new();

    for polygon in polygons {
        let edge_count = polygon.boundary.len();
        if edge_count < 3 {
            continue;
        }

        for edge_index in 0..edge_count {
            if polygon.edge_border_kinds[edge_index] != AreaBorderKind::Coastline {
                continue;
            }

            let neighbor = polygon.boundary_neighbors[edge_index];
            let key = if polygon.site_index < neighbor {
                (polygon.site_index, neighbor)
            } else {
                (neighbor, polygon.site_index)
            };
            if !seen.insert(key) {
                continue;
            }

            segments.push(polygon.boundary[edge_index]);
            segments.push(polygon.boundary[(edge_index + 1) % edge_count]);
        }
    }

    segments
}

fn outward_normal(vertex: [f32; 3]) -> [f32; 3] {
    let len_sq = vertex[0] * vertex[0] + vertex[1] * vertex[1] + vertex[2] * vertex[2];
    if len_sq <= f32::EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        let inv_len = len_sq.sqrt().recip();
        [vertex[0] * inv_len, vertex[1] * inv_len, vertex[2] * inv_len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::DistributionMethod;
    use crate::terrain::PerlinNoiseConfig;
    use crate::SphereLattice;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn combined_terrain_mesh_is_non_empty_for_small_lattice() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 32, 1.0).unwrap();
        let mut rng = StdRng::seed_from_u64(7);
        let terrain = lattice.generate_terrain(
            PerlinNoiseConfig {
                seed: Some(7),
                ..PerlinNoiseConfig::default()
            },
            &mut rng,
        );
        let mesh = build_combined_terrain_mesh_from_lattice(
            &lattice,
            &terrain,
            CombinedTerrainMeshOptions::default(),
        );
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.colors.len());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert!(!mesh.indices.is_empty());
    }

    #[test]
    fn coastline_segments_are_pairwise() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 32, 1.0).unwrap();
        let mut rng = StdRng::seed_from_u64(11);
        let terrain = lattice.generate_terrain(
            PerlinNoiseConfig {
                seed: Some(11),
                ..PerlinNoiseConfig::default()
            },
            &mut rng,
        );
        let polygons = lattice.terrain_area_polygons(&terrain);
        let segments = coastline_segment_positions(&polygons);
        assert_eq!(segments.len() % 2, 0);
    }
}
