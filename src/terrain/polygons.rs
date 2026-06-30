//! Voronoi terrain area polygons for meshing and texturing.

use crate::topology::{SphericalMesh, VoronoiCell, spherical_voronoi_cells};

use super::areas::TerrainAreaMap;
use super::borders::{AreaBorderKind, classify_area_border};
use super::types::{TerrainMap, TerrainType};

/// One terrain area as a spherical polygon suitable for Godot meshing.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainAreaPolygon {
    /// Generator vertex / cell center.
    pub site_index: usize,
    /// Terrain type at the generator site.
    pub terrain_type: TerrainType,
    /// Boundary vertices in world space (same units as lattice radius).
    pub boundary: Vec<[f32; 3]>,
    /// Neighbor site across boundary edge `i` (from vertex `i` to `(i + 1) % n`).
    pub boundary_neighbors: Vec<usize>,
    /// Semantic border kind for each boundary edge.
    pub edge_border_kinds: Vec<AreaBorderKind>,
}

/// Build terrain area polygons from a terrain map and Voronoi cells.
pub fn build_terrain_area_polygons(
    positions: &[[f32; 3]],
    mesh: &SphericalMesh,
    terrain: &TerrainMap,
    graph: &crate::topology::SurfaceGraph,
) -> Vec<TerrainAreaPolygon> {
    let radius = infer_radius(positions);
    let cells = spherical_voronoi_cells(positions, mesh);
    let area_map = super::areas::build_voronoi_areas(terrain, mesh, graph);

    cells
        .into_iter()
        .filter(|cell| cell.boundary.len() >= 3)
        .map(|cell| polygon_from_cell(&cell, radius, terrain, &area_map))
        .collect()
}

fn polygon_from_cell(
    cell: &VoronoiCell,
    radius: f32,
    terrain: &TerrainMap,
    area_map: &TerrainAreaMap,
) -> TerrainAreaPolygon {
    let site_kind = area_map.areas[cell.site_index].kind;
    let terrain_type = terrain.get(cell.site_index);
    let boundary = cell
        .boundary
        .iter()
        .map(|point| scale_point(*point, radius))
        .collect::<Vec<_>>();

    let edge_border_kinds = cell
        .boundary_neighbors
        .iter()
        .map(|&neighbor| {
            let neighbor_kind = area_map.areas[neighbor].kind;
            classify_area_border(site_kind, neighbor_kind)
        })
        .collect();

    TerrainAreaPolygon {
        site_index: cell.site_index,
        terrain_type,
        boundary,
        boundary_neighbors: cell.boundary_neighbors.clone(),
        edge_border_kinds,
    }
}

fn infer_radius(positions: &[[f32; 3]]) -> f32 {
    positions
        .first()
        .map(|position| {
            (position[0] * position[0] + position[1] * position[1] + position[2] * position[2])
                .sqrt()
        })
        .unwrap_or(1.0)
}

fn scale_point(point: [f32; 3], radius: f32) -> [f32; 3] {
    let len = (point[0] * point[0] + point[1] * point[1] + point[2] * point[2]).sqrt();
    if len <= f32::EPSILON {
        return [0.0, radius, 0.0];
    }
    let scale = radius / len;
    [point[0] * scale, point[1] * scale, point[2] * scale]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SphereLattice;
    use crate::methods::DistributionMethod;
    use crate::terrain::PerlinNoiseConfig;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn terrain_polygons_cover_all_sites_with_boundary() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 80, 1.0).unwrap();
        let terrain = lattice.generate_terrain(
            PerlinNoiseConfig {
                seed: Some(1),
                ..Default::default()
            },
            &mut StdRng::seed_from_u64(1),
        );
        let mesh = lattice.spherical_mesh();
        let graph = lattice.surface_graph();
        let polygons =
            build_terrain_area_polygons(&lattice.position_arrays(), &mesh, &terrain, &graph);
        assert!(!polygons.is_empty());
        assert!(polygons.iter().all(|polygon| polygon.boundary.len() >= 3));
        assert!(
            polygons
                .iter()
                .all(|polygon| polygon.edge_border_kinds.len() == polygon.boundary.len())
        );
    }
}
