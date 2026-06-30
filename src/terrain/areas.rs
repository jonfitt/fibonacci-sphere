//! Voronoi-cell terrain areas.
//!
//! Each lattice vertex generates one Voronoi cell centered on that site. Cell
//! classification follows the terrain type at the site. Delaunay edges provide
//! connectivity and routing; Voronoi dual edges (via circumcenters) form area
//! boundaries.

use std::collections::HashSet;

use crate::topology::{SphericalMesh, SurfaceGraph};

use super::types::{TerrainMap, TerrainType};

/// Terrain classification for a Voronoi cell area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AreaKind {
    /// Shallow below-sea cell region.
    Water,
    /// Deep below-sea cell region.
    DeepWater,
    /// Dry lowland cell region.
    Land,
    /// Dry highland cell region.
    Mountain,
    /// Polar lowland ice.
    Ice,
    /// Polar highland ice.
    IceMountain,
}

impl AreaKind {
    /// Map to the terrain type for this area kind.
    pub fn terrain_type(self) -> TerrainType {
        match self {
            Self::Water => TerrainType::Water,
            Self::DeepWater => TerrainType::DeepWater,
            Self::Land => TerrainType::Land,
            Self::Mountain => TerrainType::Mountain,
            Self::Ice => TerrainType::Ice,
            Self::IceMountain => TerrainType::IceMountain,
        }
    }
}

/// One Voronoi cell area centered on a lattice site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainArea {
    /// Stable area identifier (equal to [`Self::site_index`]).
    pub id: usize,
    /// Generator vertex / cell center.
    pub site_index: usize,
    /// Classification at the generator site.
    pub kind: AreaKind,
}

/// Voronoi partition of the sphere into one cell per lattice site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainAreaMap {
    /// All areas in site-index order.
    pub areas: Vec<TerrainArea>,
    /// Site index → area id.
    pub site_to_area: Vec<usize>,
}

/// Classify a Voronoi cell from its generator site's terrain type.
pub fn classify_site(terrain_type: TerrainType) -> AreaKind {
    match terrain_type {
        TerrainType::Water => AreaKind::Water,
        TerrainType::DeepWater => AreaKind::DeepWater,
        TerrainType::Land => AreaKind::Land,
        TerrainType::Mountain => AreaKind::Mountain,
        TerrainType::Ice => AreaKind::Ice,
        TerrainType::IceMountain => AreaKind::IceMountain,
    }
}

/// Build one Voronoi cell area per lattice site.
pub fn build_voronoi_areas(
    terrain: &TerrainMap,
    _mesh: &SphericalMesh,
    _graph: &SurfaceGraph,
) -> TerrainAreaMap {
    let site_count = terrain.len();
    let slice = terrain.as_slice();

    let areas = (0..site_count)
        .map(|site_index| {
            let kind = classify_site(slice[site_index]);
            TerrainArea {
                id: site_index,
                site_index,
                kind,
            }
        })
        .collect();

    let site_to_area = (0..site_count).collect();

    TerrainAreaMap {
        areas,
        site_to_area,
    }
}

/// Delaunay edges separating sites with different area classifications.
///
/// These are routing-mesh edges that cross a Voronoi area border.
pub fn area_border_edges(mesh: &SphericalMesh, area_map: &TerrainAreaMap) -> Vec<[usize; 2]> {
    let mut borders = HashSet::new();

    for &[left, right] in &mesh.edges {
        let left_kind = area_map.areas[area_map.site_to_area[left]].kind;
        let right_kind = area_map.areas[area_map.site_to_area[right]].kind;
        if left_kind != right_kind {
            let (a, b) = normalized_pair(left, right);
            borders.insert([a, b]);
        }
    }

    borders.into_iter().collect()
}

fn normalized_pair(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_site_matches_terrain_type() {
        assert_eq!(classify_site(TerrainType::Water), AreaKind::Water);
        assert_eq!(classify_site(TerrainType::DeepWater), AreaKind::DeepWater);
        assert_eq!(classify_site(TerrainType::Land), AreaKind::Land);
        assert_eq!(classify_site(TerrainType::Mountain), AreaKind::Mountain);
        assert_eq!(classify_site(TerrainType::Ice), AreaKind::Ice);
        assert_eq!(
            classify_site(TerrainType::IceMountain),
            AreaKind::IceMountain
        );
    }

    #[test]
    fn voronoi_areas_cover_every_site() {
        use crate::SphereLattice;
        use crate::methods::DistributionMethod;

        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 80, 1.0).unwrap();
        let terrain = TerrainMap::new(vec![TerrainType::Land; lattice.len()]);
        let area_map = build_voronoi_areas(
            &terrain,
            &lattice.spherical_mesh(),
            &lattice.surface_graph(),
        );
        assert_eq!(area_map.areas.len(), lattice.len());
        assert!(
            area_map
                .areas
                .iter()
                .enumerate()
                .all(|(index, area)| area.site_index == index)
        );
    }
}
