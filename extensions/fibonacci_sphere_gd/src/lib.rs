//! Godot 4 GDExtension for [`fibonacci_sphere`].

#![allow(clippy::too_many_arguments)] // `#[func]` signatures mirror GDScript call sites.

use fibonacci_sphere::{
    AreaBorderKind, CombinedTerrainMesh, CombinedTerrainMeshOptions, DistributionMethod,
    LineRibbonMesh, PerlinNoiseConfig, SphereLattice, SurfaceGraph, TerrainAreaPolygon, TerrainMap,
    TerrainType, VoronoiFanMeshOptions, build_combined_terrain_mesh, build_line_ribbon_mesh,
    build_voronoi_cell_fan_mesh, classify_area_border, classify_site, coastline_segment_positions,
    is_coastline_border, outward_lift, terrain_rng_from_godot_seed, voronoi_cell_fan_apex,
};
use godot::builtin::{
    Array, Color, PackedColorArray, PackedInt32Array, PackedVector3Array, Transform3D, Variant,
    Vector3,
};
use godot::classes::MultiMesh;
use godot::prelude::*;

struct FibonacciSphereExtension;

#[gdextension]
unsafe impl ExtensionLibrary for FibonacciSphereExtension {}

/// Terrain type indices for lattice vertices and routing filters.
///
/// Use with `get_terrain_type_at`, `shortest_surface_path_*_with_allowed_terrain`, and border helpers.
/// Indices match `FibonacciTerrainType.LAND`, `WATER`, and so on.
#[derive(GodotClass)]
#[class(no_init, base = RefCounted)]
pub struct FibonacciTerrainType;

#[godot_api]
impl FibonacciTerrainType {
    /// Temperate land (above sea level, below mountain threshold).
    #[constant]
    const LAND: i32 = 0;
    /// Shallow water.
    #[constant]
    const WATER: i32 = 1;
    /// Deep water.
    #[constant]
    const DEEP_WATER: i32 = 2;
    /// Mountain (high elevation).
    #[constant]
    const MOUNTAIN: i32 = 3;
    /// Ice within a polar cap flood region.
    #[constant]
    const ICE: i32 = 4;
    /// Ice-covered mountain within a polar cap flood region.
    #[constant]
    const ICE_MOUNTAIN: i32 = 5;

    /// Number of terrain types (currently 6).
    #[func]
    fn get_count() -> i32 {
        TerrainType::ALL.len() as i32
    }
}

/// Border classification for one edge of a Voronoi terrain area polygon.
///
/// Returned by `classify_border_between_terrain_types` and `FibonacciTerrainArea.get_edge_border_kinds`.
#[derive(GodotClass)]
#[class(no_init, base = RefCounted)]
pub struct FibonacciAreaBorderKind;

#[godot_api]
impl FibonacciAreaBorderKind {
    /// Both adjacent cells share the same terrain class grouping.
    #[constant]
    const SAME_TYPE: i32 = 0;
    /// Edge crosses sea level (land/mountain vs water/deep water).
    #[constant]
    const COASTLINE: i32 = 1;
    /// Edge separates shallow and deep water.
    #[constant]
    const SHALLOW_DEEP_WATER: i32 = 2;
    /// Edge separates land and mountain (no sea-level crossing).
    #[constant]
    const LAND_MOUNTAIN: i32 = 3;
}

/// One Voronoi terrain cell on the sphere: boundary loop, neighbor indices, and per-edge border kinds.
///
/// Produced by `FibonacciSphere.get_terrain_area_polygons` after terrain generation.
#[derive(GodotClass)]
#[class(no_init, base = RefCounted)]
pub struct FibonacciTerrainArea {
    site_index: i32,
    terrain_type: i32,
    boundary: PackedVector3Array,
    boundary_neighbors: PackedInt32Array,
    edge_border_kinds: PackedInt32Array,
}

#[godot_api]
impl FibonacciTerrainArea {
    /// Lattice vertex index that owns this Voronoi cell (the generator site).
    #[func]
    fn get_site_index(&self) -> i32 {
        self.site_index
    }

    /// Terrain type index for this cell (`FibonacciTerrainType.LAND`, etc.).
    #[func]
    fn get_terrain_type(&self) -> i32 {
        self.terrain_type
    }

    /// Closed boundary polyline in world space (Y-up, sphere radius already applied).
    #[func]
    fn get_boundary(&self) -> PackedVector3Array {
        self.boundary.clone()
    }

    /// Lattice vertex index for each boundary vertex (parallel to `get_boundary`).
    #[func]
    fn get_boundary_neighbors(&self) -> PackedInt32Array {
        self.boundary_neighbors.clone()
    }

    /// `FibonacciAreaBorderKind` index for each boundary edge (parallel to boundary segments).
    #[func]
    fn get_edge_border_kinds(&self) -> PackedInt32Array {
        self.edge_border_kinds.clone()
    }

    /// Returns true when `edge_index` is a coastline edge (`FibonacciAreaBorderKind.COASTLINE`).
    ///
    /// Returns false for negative or out-of-range indices.
    #[func]
    fn is_coastline_edge(&self, edge_index: i32) -> bool {
        if edge_index < 0 {
            return false;
        }
        self.edge_border_kinds
            .get(edge_index as usize)
            .is_some_and(|kind| kind == AreaBorderKind::Coastline.godot_index())
    }
}

/// Stateful Fibonacci sphere generator for Godot 4.
///
/// Generates evenly distributed points on a sphere, builds a spherical Delaunay wireframe for
/// routing, assigns Perlin terrain, and exposes batch mesh helpers.
///
/// Coordinates are **Y-up, right-handed** and match Godot 4's default 3D frame without conversion.
///
/// Typical workflow: `generate_with_terrain` (or `generate` + `generate_terrain`), then
/// `get_terrain_mesh_data`, `populate_point_multimesh`, and path queries.
#[derive(GodotClass)]
#[class(init, base = RefCounted)]
pub struct FibonacciSphere {
    #[init(val = None)]
    lattice: Option<SphereLattice>,
    #[init(val = None)]
    surface_graph: Option<SurfaceGraph>,
    #[init(val = None)]
    terrain: Option<TerrainMap>,
    #[init(val = None)]
    derived_cache: Option<DerivedRenderCache>,
}

/// Cached terrain polygons and prebuilt render buffers.
#[derive(Clone)]
struct DerivedRenderCache {
    polygons: Vec<TerrainAreaPolygon>,
    terrain_mesh: CombinedTerrainMesh,
    coastline_segments: Vec<[f32; 3]>,
}

#[godot_api]
impl FibonacciSphere {
    /// Generate a point lattice and cache its surface graph for routing.
    ///
    /// `method` is a distribution index `0..5` (see `get_method_description`).
    /// `n` is the point count (at least 1). `radius` is the sphere radius in Godot units (positive).
    ///
    /// Clears any previously cached terrain. On failure, logs an error and returns an empty array.
    /// Use `get_positions` later without regenerating.
    #[func]
    fn generate(&mut self, method: i32, n: i32, radius: f32) -> PackedVector3Array {
        if !self.try_generate_lattice(method, n, radius) {
            return PackedVector3Array::new();
        }

        self.lattice
            .as_ref()
            .map(lattice_to_packed_vector3_array)
            .unwrap_or_default()
    }

    /// Generate a lattice and Perlin terrain in one call (recommended for games and the demo).
    ///
    /// Distribution: `method` (`0..5`), `n` (point count), `radius` (sphere radius).
    ///
    /// Terrain: `mountain_threshold`, `deep_water_threshold`, and `spacing_factor` are Perlin bands
    /// in `0.0..1.0` (spacing typically `0.01..4.0`). Pass `seed < 0` for a random terrain seed.
    ///
    /// Polar ice: `north_polar_ice_distance` and `south_polar_ice_distance` are maximum angular
    /// reach from each pole in radians (`0` disables). Resistance values control how easily ice
    /// flood fill crosses mountains, land, water, and deep water. `polar_ice_latitude_cost` adds
    /// uniform cost per geodesic edge (higher → rounder caps).
    ///
    /// Returns vertex positions on success, or an empty array on error.
    #[func]
    fn generate_with_terrain(
        &mut self,
        method: i32,
        n: i32,
        radius: f32,
        seed: i32,
        mountain_threshold: f64,
        deep_water_threshold: f64,
        spacing_factor: f64,
        north_polar_ice_distance: f64,
        south_polar_ice_distance: f64,
        polar_ice_mountain_resistance: f64,
        polar_ice_land_resistance: f64,
        polar_ice_water_resistance: f64,
        polar_ice_deep_water_resistance: f64,
        polar_ice_latitude_cost: f64,
    ) -> PackedVector3Array {
        if !self.try_generate_lattice(method, n, radius) {
            return PackedVector3Array::new();
        }

        if !self.try_generate_terrain(
            seed,
            mountain_threshold,
            deep_water_threshold,
            spacing_factor,
            north_polar_ice_distance,
            south_polar_ice_distance,
            polar_ice_mountain_resistance,
            polar_ice_land_resistance,
            polar_ice_water_resistance,
            polar_ice_deep_water_resistance,
            polar_ice_latitude_cost,
        ) {
            return PackedVector3Array::new();
        }

        self.lattice
            .as_ref()
            .map(lattice_to_packed_vector3_array)
            .unwrap_or_default()
    }

    /// Assign Perlin terrain to the cached lattice (call after `generate`).
    ///
    /// Same parameters as the terrain portion of `generate_with_terrain`. Pass `seed < 0` for a
    /// random seed. Invalidates cached mesh data until the next `get_terrain_mesh_data` call.
    ///
    /// Returns `false` when no lattice exists or parameters are invalid.
    #[func]
    fn generate_terrain(
        &mut self,
        seed: i32,
        mountain_threshold: f64,
        deep_water_threshold: f64,
        spacing_factor: f64,
        north_polar_ice_distance: f64,
        south_polar_ice_distance: f64,
        polar_ice_mountain_resistance: f64,
        polar_ice_land_resistance: f64,
        polar_ice_water_resistance: f64,
        polar_ice_deep_water_resistance: f64,
        polar_ice_latitude_cost: f64,
    ) -> bool {
        self.try_generate_terrain(
            seed,
            mountain_threshold,
            deep_water_threshold,
            spacing_factor,
            north_polar_ice_distance,
            south_polar_ice_distance,
            polar_ice_mountain_resistance,
            polar_ice_land_resistance,
            polar_ice_water_resistance,
            polar_ice_deep_water_resistance,
            polar_ice_latitude_cost,
        )
    }

    /// World-space vertex positions from the last successful `generate` or `generate_with_terrain`.
    ///
    /// Returns an empty array when no lattice is cached.
    #[func]
    fn get_positions(&self) -> PackedVector3Array {
        self.lattice
            .as_ref()
            .map(lattice_to_packed_vector3_array)
            .unwrap_or_default()
    }

    /// Spherical Delaunay wireframe as paired segment endpoints: `[start, end, start, end, ...]`.
    ///
    /// Requires a cached lattice. Same edge set used by surface pathfinding.
    #[func]
    fn get_wireframe_segments(&self) -> PackedVector3Array {
        self.lattice
            .as_ref()
            .map(|lattice| {
                lattice
                    .wireframe_segment_positions()
                    .into_iter()
                    .map(|[x, y, z]| Vector3::new(x, y, z))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Terrain type index at `vertex_index` (`FibonacciTerrainType` constants).
    ///
    /// Returns `-1` when terrain is missing or the index is out of range.
    #[func]
    fn get_terrain_type_at(&self, vertex_index: i32) -> i32 {
        if vertex_index < 0 {
            return -1;
        }

        let Some(terrain) = self.terrain.as_ref() else {
            return -1;
        };

        let index = vertex_index as usize;
        if index >= terrain.len() {
            return -1;
        }

        terrain.get(index).godot_index()
    }

    /// True after a successful `generate_terrain` or `generate_with_terrain`.
    #[func]
    fn has_terrain(&self) -> bool {
        self.terrain.is_some()
    }

    /// Voronoi terrain area polygons for custom meshing or inspection.
    ///
    /// Requires generated terrain. Returns an empty array when the lattice or terrain is missing.
    /// Results are cached until the lattice or terrain changes.
    #[func]
    fn get_terrain_area_polygons(&self) -> Array<Gd<FibonacciTerrainArea>> {
        match self.derived_cache.as_ref() {
            Some(cache) => cache.polygons.iter().map(terrain_area_to_gd).collect(),
            None => {
                let Some(lattice) = self.lattice.as_ref() else {
                    godot_error!("fibonacci_sphere: no lattice generated yet");
                    return Array::new();
                };
                let Some(terrain) = self.terrain.as_ref() else {
                    godot_error!("fibonacci_sphere: no terrain generated yet");
                    return Array::new();
                };

                lattice
                    .terrain_area_polygons(terrain)
                    .iter()
                    .map(terrain_area_to_gd)
                    .collect()
            }
        }
    }

    /// Combined terrain mesh for one `ArrayMesh` build: `[vertices, colors, normals, indices]`.
    ///
    /// Each element is a `PackedVector3Array`, `PackedColorArray`, or `PackedInt32Array`.
    /// Requires terrain. Prefer this over per-cell `build_voronoi_cell_fan_mesh` loops.
    /// Returns an empty array on failure.
    #[func]
    fn get_terrain_mesh_data(&mut self) -> Array<Variant> {
        self.rebuild_derived_cache();
        let Some(cache) = self.derived_cache.as_ref() else {
            return Array::new();
        };
        combined_mesh_to_godot(&cache.terrain_mesh)
    }

    /// Deduplicated coastline segment pairs: `[start, end, ...]` across all terrain cells.
    ///
    /// Coastline edges cross sea level (land/mountain vs water/deep water). Requires terrain.
    /// Feed into `build_ribbon_line_mesh` for thick line rendering.
    #[func]
    fn get_coastline_segments(&mut self) -> PackedVector3Array {
        self.rebuild_derived_cache();
        let Some(cache) = self.derived_cache.as_ref() else {
            return PackedVector3Array::new();
        };
        positions_to_packed(&cache.coastline_segments)
    }

    /// Expand paired segment endpoints into a ribbon triangle mesh (static helper).
    ///
    /// `segments` is `[start, end, start, end, ...]`. `width` is the ribbon width in world units.
    /// `lift` radially offsets vertices outward from the origin (fraction of radius).
    ///
    /// Returns `[vertices: PackedVector3Array, indices: PackedInt32Array]` or an empty array when
    /// input is invalid.
    #[func]
    fn build_ribbon_line_mesh(
        segments: PackedVector3Array,
        width: f32,
        lift: f32,
    ) -> Array<Variant> {
        let points: Vec<[f32; 3]> = segments
            .to_vec()
            .into_iter()
            .map(|point| [point.x, point.y, point.z])
            .collect();
        let ribbon = build_line_ribbon_mesh(&points, width, lift);
        ribbon_mesh_to_godot(&ribbon)
    }

    /// Fill a `MultiMesh` with one instance per lattice vertex (efficient point clouds).
    ///
    /// Sets `use_colors`, `instance_count`, transforms, and a default color per instance.
    /// `lift` pushes points slightly above the sphere surface along the outward normal.
    ///
    /// Returns `false` when no lattice is cached.
    #[func]
    fn populate_point_multimesh(
        &self,
        mut multimesh: Gd<MultiMesh>,
        lift: f32,
        default_color: Color,
    ) -> bool {
        let Some(lattice) = self.lattice.as_ref() else {
            godot_error!("fibonacci_sphere: no lattice generated yet");
            return false;
        };

        let count = lattice.len();
        if multimesh.get_instance_count() == 0 {
            multimesh.set_use_colors(true);
        }
        multimesh.set_instance_count(count as i32);

        for (index, point) in lattice.points().iter().enumerate() {
            let lifted = outward_lift(point.position, lift);
            let mut transform = Transform3D::IDENTITY;
            transform.origin = Vector3::new(lifted[0], lifted[1], lifted[2]);
            multimesh.set_instance_transform(index as i32, transform);
            multimesh.set_instance_color(index as i32, default_color);
        }

        true
    }

    /// Highlight route endpoints on a `MultiMesh` populated by `populate_point_multimesh`.
    ///
    /// Sets `selected_color` on `from_index` and `to_index`; all other instances use `default_color`.
    /// Pass `-1` for an unused endpoint. Returns `false` when no lattice is cached.
    #[func]
    fn update_point_multimesh_highlights(
        &self,
        mut multimesh: Gd<MultiMesh>,
        from_index: i32,
        to_index: i32,
        default_color: Color,
        selected_color: Color,
    ) -> bool {
        let Some(lattice) = self.lattice.as_ref() else {
            return false;
        };

        let count = lattice.len() as i32;
        for index in 0..count {
            let color = if index == from_index || index == to_index {
                selected_color
            } else {
                default_color
            };
            multimesh.set_instance_color(index, color);
        }

        true
    }

    /// Classify the border between two `FibonacciTerrainType` indices.
    ///
    /// Returns a `FibonacciAreaBorderKind` constant, or `-1` for invalid type indices.
    #[func]
    fn classify_border_between_terrain_types(left: i32, right: i32) -> i32 {
        match (
            TerrainType::from_godot_index(left),
            TerrainType::from_godot_index(right),
        ) {
            (Some(left_type), Some(right_type)) => {
                classify_area_border(classify_site(left_type), classify_site(right_type))
                    .godot_index()
            }
            _ => -1,
        }
    }

    /// Returns true when the border between two terrain types crosses sea level (coastline).
    #[func]
    fn is_coastline_between_terrain_types(left: i32, right: i32) -> bool {
        match (
            TerrainType::from_godot_index(left),
            TerrainType::from_godot_index(right),
        ) {
            (Some(left_type), Some(right_type)) => {
                is_coastline_border(classify_site(left_type), classify_site(right_type))
            }
            _ => false,
        }
    }

    /// Geodesic angular distance from `vertex_index` to the north pole (+Y), in radians.
    ///
    /// Returns `-1.0` on error (no lattice or invalid index).
    #[func]
    fn angular_distance_to_north_pole(&self, vertex_index: i32) -> f32 {
        self.angular_distance_at(vertex_index, |lattice, index| {
            lattice.angular_distance_to_north_pole_at(index)
        })
    }

    /// Geodesic angular distance from `vertex_index` to the south pole (-Y), in radians.
    ///
    /// Returns `-1.0` on error.
    #[func]
    fn angular_distance_to_south_pole(&self, vertex_index: i32) -> f32 {
        self.angular_distance_at(vertex_index, |lattice, index| {
            lattice.angular_distance_to_south_pole_at(index)
        })
    }

    /// Angular distance from `vertex_index` to the equator (XZ plane), in radians.
    ///
    /// Returns `-1.0` on error.
    #[func]
    fn angular_distance_to_equator(&self, vertex_index: i32) -> f32 {
        self.angular_distance_at(vertex_index, |lattice, index| {
            lattice.angular_distance_to_equator_at(index)
        })
    }

    /// Lattice vertex indices within `max_angle` radians of the north pole.
    ///
    /// Returns an empty array when no lattice is cached.
    #[func]
    fn vertices_within_north_polar_distance(&self, max_angle: f64) -> PackedInt32Array {
        self.lattice
            .as_ref()
            .map(|lattice| {
                indices_to_packed(lattice.vertices_within_north_polar_distance(max_angle))
            })
            .unwrap_or_default()
    }

    /// Lattice vertex indices within `max_angle` radians of the south pole.
    #[func]
    fn vertices_within_south_polar_distance(&self, max_angle: f64) -> PackedInt32Array {
        self.lattice
            .as_ref()
            .map(|lattice| {
                indices_to_packed(lattice.vertices_within_south_polar_distance(max_angle))
            })
            .unwrap_or_default()
    }

    /// Lattice vertex indices within `max_angle` radians of the equator.
    #[func]
    fn vertices_within_equatorial_distance(&self, max_angle: f64) -> PackedInt32Array {
        self.lattice
            .as_ref()
            .map(|lattice| {
                indices_to_packed(lattice.vertices_within_equatorial_distance(max_angle))
            })
            .unwrap_or_default()
    }

    /// Closest lattice vertex to a world-space position on the sphere.
    ///
    /// Useful for mapping raycast hits to route endpoints. Returns `-1` on error.
    #[func]
    fn find_nearest_vertex_index(&self, position: Vector3) -> i32 {
        let Some(lattice) = self.lattice.as_ref() else {
            godot_error!("fibonacci_sphere: no lattice generated yet");
            return -1;
        };

        match lattice.nearest_vertex_index([position.x, position.y, position.z]) {
            Ok(index) => index as i32,
            Err(error) => {
                godot_error!("fibonacci_sphere: {error}");
                -1
            }
        }
    }

    /// Fan triangulation apex for one Voronoi terrain cell mesh.
    ///
    /// Polar-extreme cells use the geographic pole; all others use the generator site position.
    /// Returns `Vector3.ZERO` on error.
    #[func]
    fn get_voronoi_fan_apex_position(&self, site_index: i32) -> Vector3 {
        if site_index < 0 {
            return Vector3::ZERO;
        }

        let Some(lattice) = self.lattice.as_ref() else {
            godot_error!("fibonacci_sphere: no lattice generated yet");
            return Vector3::ZERO;
        };

        let positions = lattice.position_arrays();
        let index = site_index as usize;
        if index >= positions.len() {
            godot_error!("fibonacci_sphere: site index out of range");
            return Vector3::ZERO;
        }

        let apex = voronoi_cell_fan_apex(index, &positions);
        let radius = lattice.radius() as f32;
        Vector3::new(apex[0] * radius, apex[1] * radius, apex[2] * radius)
    }

    /// Shortest path vertex indices along Delaunay mesh edges (geodesic weights).
    ///
    /// Requires a cached lattice from `generate`. Returns an empty array on error.
    #[func]
    fn shortest_surface_path_indices(&self, from_index: i32, to_index: i32) -> PackedInt32Array {
        self.shortest_surface_path_indices_with_allowed_terrain(
            from_index,
            to_index,
            PackedInt32Array::new(),
        )
    }

    /// Shortest path indices restricted to vertices whose terrain type is in `allowed_terrain_types`.
    ///
    /// Pass an empty array to allow every terrain type (same as `shortest_surface_path_indices`).
    /// Terrain-filtered routing requires `generate_terrain` first.
    #[func]
    fn shortest_surface_path_indices_with_allowed_terrain(
        &self,
        from_index: i32,
        to_index: i32,
        allowed_terrain_types: PackedInt32Array,
    ) -> PackedInt32Array {
        if from_index < 0 || to_index < 0 {
            godot_error!("fibonacci_sphere: vertex indices must be non-negative");
            return PackedInt32Array::new();
        }

        match self.shortest_surface_path_with_allowed(from_index, to_index, &allowed_terrain_types)
        {
            Ok(path) => path
                .vertices
                .into_iter()
                .map(|index| index as i32)
                .collect(),
            Err(error) => {
                godot_error!("fibonacci_sphere: {error}");
                PackedInt32Array::new()
            }
        }
    }

    /// World positions along the shortest surface path between two vertex indices.
    ///
    /// Returns an empty array on error.
    #[func]
    fn shortest_surface_path_positions(
        &self,
        from_index: i32,
        to_index: i32,
    ) -> PackedVector3Array {
        self.shortest_surface_path_positions_with_allowed_terrain(
            from_index,
            to_index,
            PackedInt32Array::new(),
        )
    }

    /// World positions along the shortest path restricted to allowed terrain types.
    ///
    /// Pass an empty `allowed_terrain_types` array to allow every type.
    #[func]
    fn shortest_surface_path_positions_with_allowed_terrain(
        &self,
        from_index: i32,
        to_index: i32,
        allowed_terrain_types: PackedInt32Array,
    ) -> PackedVector3Array {
        if from_index < 0 || to_index < 0 {
            godot_error!("fibonacci_sphere: vertex indices must be non-negative");
            return PackedVector3Array::new();
        }

        let path = match self.shortest_surface_path_with_allowed(
            from_index,
            to_index,
            &allowed_terrain_types,
        ) {
            Ok(path) => path,
            Err(error) => {
                godot_error!("fibonacci_sphere: {error}");
                return PackedVector3Array::new();
            }
        };

        let Some(lattice) = self.lattice.as_ref() else {
            godot_error!("fibonacci_sphere: no lattice generated yet");
            return PackedVector3Array::new();
        };

        path.positions(lattice.points())
            .into_iter()
            .map(|[x, y, z]| Vector3::new(x, y, z))
            .collect()
    }

    /// Total geodesic length of the shortest surface path (same units as lattice radius).
    ///
    /// Returns `-1.0` on error.
    #[func]
    fn shortest_surface_path_length(&self, from_index: i32, to_index: i32) -> f32 {
        self.shortest_surface_path_length_with_allowed_terrain(
            from_index,
            to_index,
            PackedInt32Array::new(),
        )
    }

    /// Geodesic path length restricted to allowed terrain types.
    ///
    /// Returns `-1.0` on error.
    #[func]
    fn shortest_surface_path_length_with_allowed_terrain(
        &self,
        from_index: i32,
        to_index: i32,
        allowed_terrain_types: PackedInt32Array,
    ) -> f32 {
        if from_index < 0 || to_index < 0 {
            godot_error!("fibonacci_sphere: vertex indices must be non-negative");
            return -1.0;
        }

        match self.shortest_surface_path_with_allowed(from_index, to_index, &allowed_terrain_types)
        {
            Ok(path) => path.length as f32,
            Err(error) => {
                godot_error!("fibonacci_sphere: {error}");
                -1.0
            }
        }
    }

    /// Fan-triangulate one Voronoi cell for custom rendering.
    ///
    /// `fan_apex` is typically from `get_voronoi_fan_apex_position`. `boundary` is the cell loop.
    /// `radius` scales unit directions to world space.
    ///
    /// Returns `[vertices: PackedVector3Array, indices: PackedInt32Array]` or an empty array on
    /// failure. For the full sphere, prefer `get_terrain_mesh_data`.
    #[func]
    fn build_voronoi_cell_fan_mesh(
        &self,
        fan_apex: Vector3,
        boundary: PackedVector3Array,
        radius: f32,
    ) -> Array<Variant> {
        let boundary_points: Vec<[f32; 3]> = boundary
            .to_vec()
            .into_iter()
            .map(|point| [point.x, point.y, point.z])
            .collect();

        let Some(mesh) = build_voronoi_cell_fan_mesh(
            [fan_apex.x, fan_apex.y, fan_apex.z],
            &boundary_points,
            radius,
            VoronoiFanMeshOptions {
                flip_winding: true,
                ..VoronoiFanMeshOptions::default()
            },
        ) else {
            return Array::new();
        };

        let vertices: PackedVector3Array = mesh
            .vertices
            .into_iter()
            .map(|[x, y, z]| Vector3::new(x, y, z))
            .collect();
        let indices: PackedInt32Array = mesh
            .triangles
            .into_iter()
            .flat_map(|triangle| triangle.map(|index| index as i32))
            .collect();

        let mut result = Array::<Variant>::new();
        result.push(&vertices.to_variant());
        result.push(&indices.to_variant());
        result
    }

    /// Number of points in the cached lattice (`0` when not generated).
    #[func]
    fn get_point_count(&self) -> i32 {
        self.lattice.as_ref().map(|l| l.len() as i32).unwrap_or(0)
    }

    /// Sphere radius used by the cached lattice (`0.0` when not generated).
    #[func]
    fn get_radius(&self) -> f32 {
        self.lattice
            .as_ref()
            .map(|l| l.radius() as f32)
            .unwrap_or(0.0)
    }

    /// Distribution method index of the cached lattice (`0..5`, or `-1` when not generated).
    #[func]
    fn get_method_index(&self) -> i32 {
        self.lattice
            .as_ref()
            .map(|l| l.method().to_godot_index())
            .unwrap_or(-1)
    }

    /// Drop the cached lattice, surface graph, terrain, and derived mesh data.
    #[func]
    fn clear(&mut self) {
        self.lattice = None;
        self.surface_graph = None;
        self.terrain = None;
        self.derived_cache = None;
    }

    /// One-shot lattice generation without retaining state (static helper).
    ///
    /// Same parameters as `generate`. Useful for quick probes; use an instance for routing/terrain.
    #[func]
    fn generate_positions(method: i32, n: i32, radius: f32) -> PackedVector3Array {
        let mut generator = FibonacciSphere {
            lattice: None,
            surface_graph: None,
            terrain: None,
            derived_cache: None,
        };
        generator.generate(method, n, radius)
    }

    /// Number of distribution methods exposed to Godot (currently 6).
    #[func]
    fn get_method_count() -> i32 {
        DistributionMethod::ALL.len() as i32
    }

    /// Multi-line literature-backed description for a distribution method index.
    ///
    /// Returns an empty string for invalid indices. Suitable for HUD help text.
    #[func]
    fn get_method_description(method: i32) -> GString {
        DistributionMethod::from_godot_index(method)
            .map(|m| {
                let text = m.format_description();
                GString::from(&text)
            })
            .unwrap_or_default()
    }
}

impl FibonacciSphere {
    fn try_generate_lattice(&mut self, method: i32, n: i32, radius: f32) -> bool {
        let Some(distribution) = DistributionMethod::from_godot_index(method) else {
            godot_error!("fibonacci_sphere: invalid method index {method} (expected 0..=5)");
            return false;
        };

        if n < 1 {
            godot_error!("fibonacci_sphere: point count must be at least 1, got {n}");
            return false;
        }

        if radius <= 0.0 {
            godot_error!("fibonacci_sphere: radius must be positive, got {radius}");
            return false;
        }

        let lattice = match SphereLattice::generate(distribution, n as usize, f64::from(radius)) {
            Ok(lattice) => lattice,
            Err(error) => {
                godot_error!("fibonacci_sphere: {error}");
                return false;
            }
        };

        self.surface_graph = Some(lattice.surface_graph());
        self.lattice = Some(lattice);
        self.terrain = None;
        self.derived_cache = None;
        true
    }

    fn try_generate_terrain(
        &mut self,
        seed: i32,
        mountain_threshold: f64,
        deep_water_threshold: f64,
        spacing_factor: f64,
        north_polar_ice_distance: f64,
        south_polar_ice_distance: f64,
        polar_ice_mountain_resistance: f64,
        polar_ice_land_resistance: f64,
        polar_ice_water_resistance: f64,
        polar_ice_deep_water_resistance: f64,
        polar_ice_latitude_cost: f64,
    ) -> bool {
        let Some(lattice) = self.lattice.as_ref() else {
            godot_error!("fibonacci_sphere: no lattice generated yet");
            return false;
        };

        let config = PerlinNoiseConfig {
            mountain_threshold,
            deep_water_threshold,
            spacing_factor,
            seed: if seed < 0 { None } else { Some(seed as u32) },
            north_polar_ice_distance,
            south_polar_ice_distance,
            polar_ice_mountain_resistance,
            polar_ice_land_resistance,
            polar_ice_water_resistance,
            polar_ice_deep_water_resistance,
            polar_ice_latitude_cost,
        };

        let mut rng = terrain_rng_from_godot_seed(seed);
        self.derived_cache = None;
        self.terrain = Some(lattice.generate_terrain(config, &mut rng));
        self.rebuild_derived_cache();
        true
    }

    fn rebuild_derived_cache(&mut self) {
        if self.derived_cache.is_some() {
            return;
        }

        let Some(lattice) = self.lattice.as_ref() else {
            return;
        };
        let Some(terrain) = self.terrain.as_ref() else {
            return;
        };
        let positions = lattice.position_arrays();
        let polygons = lattice.terrain_area_polygons(terrain);
        let coastline_segments = coastline_segment_positions(&polygons);
        let terrain_mesh = build_combined_terrain_mesh(
            &polygons,
            &positions,
            lattice.radius() as f32,
            CombinedTerrainMeshOptions {
                fan_mesh: VoronoiFanMeshOptions {
                    flip_winding: true,
                    ..VoronoiFanMeshOptions::default()
                },
            },
        );

        self.derived_cache = Some(DerivedRenderCache {
            polygons,
            terrain_mesh,
            coastline_segments,
        });
    }

    fn angular_distance_at<F>(&self, vertex_index: i32, query: F) -> f32
    where
        F: FnOnce(&SphereLattice, usize) -> Result<f64, fibonacci_sphere::SphereError>,
    {
        if vertex_index < 0 {
            godot_error!("fibonacci_sphere: vertex indices must be non-negative");
            return -1.0;
        }

        let Some(lattice) = self.lattice.as_ref() else {
            godot_error!("fibonacci_sphere: no lattice generated yet");
            return -1.0;
        };

        match query(lattice, vertex_index as usize) {
            Ok(distance) => distance as f32,
            Err(error) => {
                godot_error!("fibonacci_sphere: {error}");
                -1.0
            }
        }
    }

    fn shortest_surface_path_with_allowed(
        &self,
        from_index: i32,
        to_index: i32,
        allowed_terrain_types: &PackedInt32Array,
    ) -> Result<fibonacci_sphere::SurfacePath, fibonacci_sphere::SphereError> {
        let Some(graph) = self.surface_graph.as_ref() else {
            godot_error!("fibonacci_sphere: no lattice generated yet");
            return Err(fibonacci_sphere::SphereError::InvalidPointCount { n: 0 });
        };

        let from = from_index as usize;
        let to = to_index as usize;
        let allowed = parse_allowed_terrain_types(allowed_terrain_types);

        if allowed.is_empty() {
            return graph.shortest_path(from, to);
        }

        let Some(terrain) = self.terrain.as_ref() else {
            godot_error!("fibonacci_sphere: terrain-restricted routing requires generated terrain");
            return Err(fibonacci_sphere::SphereError::TerrainNotGenerated);
        };

        graph.shortest_path_with_allowed_terrain(from, to, terrain.as_slice(), &allowed)
    }
}

fn lattice_to_packed_vector3_array(lattice: &SphereLattice) -> PackedVector3Array {
    lattice
        .points()
        .iter()
        .map(|point| Vector3::new(point.position[0], point.position[1], point.position[2]))
        .collect()
}

fn indices_to_packed(indices: Vec<usize>) -> PackedInt32Array {
    indices.into_iter().map(|index| index as i32).collect()
}

fn parse_allowed_terrain_types(values: &PackedInt32Array) -> Vec<TerrainType> {
    values
        .to_vec()
        .into_iter()
        .filter_map(TerrainType::from_godot_index)
        .collect()
}

fn terrain_area_to_gd(polygon: &TerrainAreaPolygon) -> Gd<FibonacciTerrainArea> {
    Gd::from_object(FibonacciTerrainArea {
        site_index: polygon.site_index as i32,
        terrain_type: polygon.terrain_type.godot_index(),
        boundary: polygon
            .boundary
            .iter()
            .map(|&[x, y, z]| Vector3::new(x, y, z))
            .collect(),
        boundary_neighbors: polygon
            .boundary_neighbors
            .iter()
            .map(|&index| index as i32)
            .collect(),
        edge_border_kinds: polygon
            .edge_border_kinds
            .iter()
            .map(|kind| kind.godot_index())
            .collect(),
    })
}

fn positions_to_packed(points: &[[f32; 3]]) -> PackedVector3Array {
    points
        .iter()
        .map(|&[x, y, z]| Vector3::new(x, y, z))
        .collect()
}

fn combined_mesh_to_godot(mesh: &CombinedTerrainMesh) -> Array<Variant> {
    let vertices: PackedVector3Array = mesh
        .vertices
        .iter()
        .map(|&[x, y, z]| Vector3::new(x, y, z))
        .collect();
    let colors: PackedColorArray = mesh
        .colors
        .iter()
        .map(|&[r, g, b, a]| Color::from_rgba(r, g, b, a))
        .collect();
    let normals: PackedVector3Array = mesh
        .normals
        .iter()
        .map(|&[x, y, z]| Vector3::new(x, y, z))
        .collect();
    let indices: PackedInt32Array = mesh.indices.iter().map(|&index| index as i32).collect();

    let mut result = Array::<Variant>::new();
    result.push(&vertices.to_variant());
    result.push(&colors.to_variant());
    result.push(&normals.to_variant());
    result.push(&indices.to_variant());
    result
}

fn ribbon_mesh_to_godot(mesh: &LineRibbonMesh) -> Array<Variant> {
    if mesh.vertices.is_empty() {
        return Array::new();
    }

    let vertices: PackedVector3Array = mesh
        .vertices
        .iter()
        .map(|&[x, y, z]| Vector3::new(x, y, z))
        .collect();
    let indices: PackedInt32Array = mesh.indices.iter().map(|&index| index as i32).collect();

    let mut result = Array::<Variant>::new();
    result.push(&vertices.to_variant());
    result.push(&indices.to_variant());
    result
}
