use std::sync::OnceLock;

use crate::error::SphereError;
use crate::methods::DistributionMethod;
use crate::point::SpherePoint;
use crate::topology::{SphericalMesh, SurfaceGraph, spherical_delaunay_mesh};

#[derive(Debug, Clone)]
struct TopologyCache {
    positions: Vec<[f32; 3]>,
    mesh: SphericalMesh,
    graph: SurfaceGraph,
}

/// A generated set of points on a sphere.
#[derive(Debug)]
pub struct SphereLattice {
    points: Vec<SpherePoint>,
    method: DistributionMethod,
    radius: f64,
    topology: OnceLock<TopologyCache>,
}

impl Clone for SphereLattice {
    fn clone(&self) -> Self {
        Self {
            points: self.points.clone(),
            method: self.method,
            radius: self.radius,
            topology: OnceLock::new(),
        }
    }
}

impl SphereLattice {
    /// Generate a new lattice with the given method, point count, and radius.
    pub fn generate(
        method: DistributionMethod,
        n: usize,
        radius: f64,
    ) -> Result<Self, SphereError> {
        let points = method.generate(n, radius)?;
        Ok(Self {
            points,
            method,
            radius,
            topology: OnceLock::new(),
        })
    }

    /// Distribution method used to create this lattice.
    pub fn method(&self) -> DistributionMethod {
        self.method
    }

    /// Sphere radius used for generation.
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Number of sample points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns true when the lattice contains no points.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// All generated sample points.
    pub fn points(&self) -> &[SpherePoint] {
        &self.points
    }

    /// Iterate over sample points.
    pub fn iter(&self) -> impl Iterator<Item = &SpherePoint> {
        self.points.iter()
    }

    /// Flat `[x0, y0, z0, x1, y1, z1, ...]` suitable for Godot `PackedVector3Array` FFI.
    pub fn positions_flat(&self) -> Vec<f32> {
        self.points.iter().flat_map(|p| p.position).collect()
    }

    /// Cartesian coordinates as `[x, y, z]` arrays (allocates).
    pub fn position_arrays(&self) -> Vec<[f32; 3]> {
        self.topology()
            .map(|cache| cache.positions.clone())
            .unwrap_or_else(|| self.points.iter().map(|p| p.position).collect())
    }

    /// Spherical Delaunay mesh (triangle faces + wireframe edges).
    pub fn spherical_mesh(&self) -> SphericalMesh {
        self.topology()
            .map(|cache| cache.mesh.clone())
            .unwrap_or_else(|| {
                spherical_delaunay_mesh(&self.points.iter().map(|p| p.position).collect::<Vec<_>>())
            })
    }

    /// Delaunay triangle faces (connectivity mesh, not area partition).
    pub fn delaunay_triangles(&self) -> Vec<[usize; 3]> {
        self.spherical_mesh().triangles
    }

    /// Undirected edges of the spherical Delaunay triangulation.
    pub fn wireframe_edges(&self) -> Vec<[usize; 2]> {
        self.spherical_mesh().edges
    }

    /// Line segment endpoints for wireframe rendering (`[start, end, start, end, ...]`).
    pub fn wireframe_segment_positions(&self) -> Vec<[f32; 3]> {
        if let Some(cache) = self.topology() {
            crate::topology::wireframe_segment_positions(&cache.positions, &cache.mesh.edges)
        } else {
            let positions: Vec<[f32; 3]> = self.points.iter().map(|p| p.position).collect();
            let edges = self.wireframe_edges();
            crate::topology::wireframe_segment_positions(&positions, &edges)
        }
    }

    /// Delaunay mesh graph for surface pathfinding.
    pub fn surface_graph(&self) -> SurfaceGraph {
        self.topology()
            .map(|cache| cache.graph.clone())
            .unwrap_or_else(|| SurfaceGraph::from_positions(&self.position_arrays()))
    }

    /// Shortest path along mesh edges between two vertex indices (geodesic weights).
    pub fn shortest_surface_path(
        &self,
        from_index: usize,
        to_index: usize,
    ) -> Result<crate::topology::SurfacePath, SphereError> {
        self.surface_graph().shortest_path(from_index, to_index)
    }

    /// Shortest path that only traverses vertices whose terrain type is in `allowed`.
    ///
    /// When `allowed` is empty, all terrain types are permitted.
    #[cfg(feature = "terrain")]
    pub fn shortest_surface_path_with_allowed_terrain(
        &self,
        from_index: usize,
        to_index: usize,
        terrain: &crate::terrain::TerrainMap,
        allowed: &[crate::terrain::TerrainType],
    ) -> Result<crate::topology::SurfacePath, SphereError> {
        self.surface_graph().shortest_path_with_allowed_terrain(
            from_index,
            to_index,
            terrain.as_slice(),
            allowed,
        )
    }

    /// World positions along the shortest surface path between two vertex indices.
    pub fn shortest_surface_path_positions(
        &self,
        from_index: usize,
        to_index: usize,
    ) -> Result<Vec<[f32; 3]>, SphereError> {
        let path = self.shortest_surface_path(from_index, to_index)?;
        Ok(path.positions(self.points()))
    }

    /// Index of the lattice vertex nearest to a world-space position on the sphere.
    pub fn nearest_vertex_index(&self, position: [f32; 3]) -> Result<usize, SphereError> {
        crate::topology::nearest_vertex_index(&self.position_arrays(), position)
    }

    /// Build Voronoi-cell terrain areas from a generated terrain map.
    #[cfg(feature = "terrain")]
    pub fn terrain_areas(
        &self,
        terrain: &crate::terrain::TerrainMap,
    ) -> crate::terrain::TerrainAreaMap {
        let cache = self.topology().expect("topology cache");
        crate::terrain::build_voronoi_areas(terrain, &cache.mesh, &cache.graph)
    }

    /// Voronoi terrain area polygons in world space for meshing and texturing.
    #[cfg(feature = "terrain")]
    pub fn terrain_area_polygons(
        &self,
        terrain: &crate::terrain::TerrainMap,
    ) -> Vec<crate::terrain::TerrainAreaPolygon> {
        let cache = self.topology().expect("topology cache");
        crate::terrain::build_terrain_area_polygons(
            &cache.positions,
            &cache.mesh,
            terrain,
            &cache.graph,
        )
    }

    /// Combined vertex-colored mesh for all terrain area polygons.
    #[cfg(feature = "terrain")]
    pub fn combined_terrain_mesh(
        &self,
        terrain: &crate::terrain::TerrainMap,
        options: crate::render::CombinedTerrainMeshOptions,
    ) -> crate::render::CombinedTerrainMesh {
        crate::render::build_combined_terrain_mesh_from_lattice(self, terrain, options)
    }

    /// Undirected coastline segment endpoints (`[start, end, ...]`).
    #[cfg(feature = "terrain")]
    pub fn coastline_segment_positions(
        &self,
        terrain: &crate::terrain::TerrainMap,
    ) -> Vec<[f32; 3]> {
        crate::render::coastline_segment_positions(&self.terrain_area_polygons(terrain))
    }

    /// Generate terrain types from Perlin noise for every lattice vertex.
    #[cfg(feature = "terrain")]
    pub fn generate_terrain(
        &self,
        config: crate::terrain::PerlinNoiseConfig,
        rng: &mut dyn rand::RngCore,
    ) -> crate::terrain::TerrainMap {
        let cache = self.topology().expect("topology cache");
        crate::terrain::TerrainGenerator::perlin_noise(&cache.positions, config)
            .generate(&cache.graph, rng)
    }

    /// Generate terrain types with a custom generator pipeline.
    #[cfg(feature = "terrain")]
    pub fn generate_terrain_with(
        &self,
        generator: &crate::terrain::TerrainGenerator,
        rng: &mut dyn rand::RngCore,
    ) -> crate::terrain::TerrainMap {
        let cache = self.topology().expect("topology cache");
        generator.generate(&cache.graph, rng)
    }

    /// Angular distance from a lattice vertex to the north pole, in radians.
    pub fn angular_distance_to_north_pole_at(&self, index: usize) -> Result<f64, SphereError> {
        validate_vertex_index(index, self.len())?;
        Ok(crate::geography::angular_distance_to_north_pole(
            self.points()[index].position,
        ))
    }

    /// Angular distance from a lattice vertex to the south pole, in radians.
    pub fn angular_distance_to_south_pole_at(&self, index: usize) -> Result<f64, SphereError> {
        validate_vertex_index(index, self.len())?;
        Ok(crate::geography::angular_distance_to_south_pole(
            self.points()[index].position,
        ))
    }

    /// Angular distance from a lattice vertex to the equator, in radians.
    pub fn angular_distance_to_equator_at(&self, index: usize) -> Result<f64, SphereError> {
        validate_vertex_index(index, self.len())?;
        Ok(crate::geography::angular_distance_to_equator(
            self.points()[index].position,
        ))
    }

    /// Vertex indices within `max_angle` radians of the north pole.
    pub fn vertices_within_north_polar_distance(&self, max_angle: f64) -> Vec<usize> {
        crate::geography::vertices_within_north_polar_distance(&self.position_arrays(), max_angle)
    }

    /// Vertex indices within `max_angle` radians of the south pole.
    pub fn vertices_within_south_polar_distance(&self, max_angle: f64) -> Vec<usize> {
        crate::geography::vertices_within_south_polar_distance(&self.position_arrays(), max_angle)
    }

    /// Vertex indices within `max_angle` radians of the equator.
    pub fn vertices_within_equatorial_distance(&self, max_angle: f64) -> Vec<usize> {
        crate::geography::vertices_within_equatorial_distance(&self.position_arrays(), max_angle)
    }

    fn topology(&self) -> Option<&TopologyCache> {
        if self.points.len() < 2 {
            return None;
        }

        Some(self.topology.get_or_init(|| {
            let positions: Vec<[f32; 3]> = self.points.iter().map(|p| p.position).collect();
            let mesh = spherical_delaunay_mesh(&positions);
            let graph = SurfaceGraph::from_edges(&positions, &mesh.edges);
            TopologyCache {
                positions,
                mesh,
                graph,
            }
        }))
    }
}

fn validate_vertex_index(index: usize, count: usize) -> Result<(), SphereError> {
    if index >= count {
        return Err(SphereError::InvalidVertexIndex { index, count });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{assert_on_sphere, assert_unique_positions};

    #[test]
    fn generate_all_methods() {
        for method in DistributionMethod::ALL {
            let lattice = SphereLattice::generate(method, 50, 2.0).unwrap();
            assert_eq!(lattice.len(), 50);
            assert_eq!(lattice.method(), method);
            assert_eq!(lattice.radius(), 2.0);
            assert!(!lattice.is_empty());
            assert_on_sphere(lattice.points(), 2.0);
        }
    }

    #[test]
    fn positions_flat_layout() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 2, 1.0).unwrap();
        let flat = lattice.positions_flat();
        assert_eq!(flat.len(), 6);
        assert_eq!(flat[0..3], lattice.points()[0].position);
        assert_eq!(flat[3..6], lattice.points()[1].position);
    }

    #[test]
    fn iter_yields_same_points() {
        let lattice = SphereLattice::generate(DistributionMethod::Canonical, 5, 1.0).unwrap();
        let collected: Vec<_> = lattice.iter().map(|p| p.index).collect();
        assert_eq!(collected, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn clone_preserves_data() {
        let lattice = SphereLattice::generate(DistributionMethod::OffsetPacking, 8, 1.5).unwrap();
        let cloned = lattice.clone();
        assert_eq!(cloned.method(), lattice.method());
        assert_eq!(cloned.radius(), lattice.radius());
        assert_eq!(cloned.points(), lattice.points());
    }

    #[test]
    fn topology_cache_reuses_wireframe_and_graph() {
        let lattice =
            SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 40, 1.0).unwrap();
        let first_edges = lattice.wireframe_edges();
        let second_edges = lattice.wireframe_edges();
        assert_eq!(first_edges, second_edges);
        assert_eq!(lattice.surface_graph().len(), lattice.len());
    }

    #[test]
    fn rejects_zero_count() {
        assert!(matches!(
            SphereLattice::generate(DistributionMethod::Canonical, 0, 1.0),
            Err(SphereError::InvalidPointCount { n: 0 })
        ));
    }

    #[test]
    fn rejects_invalid_radius() {
        assert!(matches!(
            SphereLattice::generate(DistributionMethod::Canonical, 5, 0.0),
            Err(SphereError::InvalidRadius { radius: 0.0 })
        ));
    }

    #[test]
    fn uniqueness_across_methods() {
        for method in DistributionMethod::ALL {
            let lattice = SphereLattice::generate(method, 40, 1.0).unwrap();
            assert_unique_positions(lattice.points());
        }
    }
}
