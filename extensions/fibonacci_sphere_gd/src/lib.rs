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

/// Terrain type indices exposed to Godot (`Land`=0, `Water`=1, `DeepWater`=2, `Mountain`=3, `Ice`=4, `IceMountain`=5).
#[derive(GodotClass)]
#[class(no_init, base = RefCounted)]
pub struct FibonacciTerrainType;

#[godot_api]
impl FibonacciTerrainType {
    #[constant]
    const LAND: i32 = 0;
    #[constant]
    const WATER: i32 = 1;
    #[constant]
    const DEEP_WATER: i32 = 2;
    #[constant]
    const MOUNTAIN: i32 = 3;
    #[constant]
    const ICE: i32 = 4;
    #[constant]
    const ICE_MOUNTAIN: i32 = 5;

    #[func]
    fn get_count() -> i32 {
        TerrainType::ALL.len() as i32
    }
}

/// Border kind indices for terrain area polygon edges.
#[derive(GodotClass)]
#[class(no_init, base = RefCounted)]
pub struct FibonacciAreaBorderKind;

#[godot_api]
impl FibonacciAreaBorderKind {
    #[constant]
    const SAME_TYPE: i32 = 0;
    #[constant]
    const COASTLINE: i32 = 1;
    #[constant]
    const SHALLOW_DEEP_WATER: i32 = 2;
    #[constant]
    const LAND_MOUNTAIN: i32 = 3;
}

/// One Voronoi terrain area polygon for meshing and texturing in Godot.
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
    #[func]
    fn get_site_index(&self) -> i32 {
        self.site_index
    }

    #[func]
    fn get_terrain_type(&self) -> i32 {
        self.terrain_type
    }

    #[func]
    fn get_boundary(&self) -> PackedVector3Array {
        self.boundary.clone()
    }

    #[func]
    fn get_boundary_neighbors(&self) -> PackedInt32Array {
        self.boundary_neighbors.clone()
    }

    #[func]
    fn get_edge_border_kinds(&self) -> PackedInt32Array {
        self.edge_border_kinds.clone()
    }

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

/// Generates and caches Fibonacci sphere lattices for Godot scenes.
///
/// Coordinates are Y-up and match Godot 4's default 3D frame without conversion.
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

    /// Generate a lattice and Perlin terrain in one call.
    ///
    /// Pass `seed` < 0 to pick a seed from the internal RNG. Returns an empty array on error.
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

    /// Generate Perlin terrain for the cached lattice.
    ///
    /// Pass `seed` < 0 to pick a seed from the internal RNG. Returns `false` on error.
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

    /// Returns positions from the last successful [`Self::generate`] call.
    #[func]
    fn get_positions(&self) -> PackedVector3Array {
        self.lattice
            .as_ref()
            .map(lattice_to_packed_vector3_array)
            .unwrap_or_default()
    }

    /// Line segment endpoints for wireframe rendering (`[start, end, ...]`).
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

    /// Terrain type at a vertex index (`-1` when terrain is not generated or index is invalid).
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

    /// Whether terrain has been generated for the cached lattice.
    #[func]
    fn has_terrain(&self) -> bool {
        self.terrain.is_some()
    }

    /// Voronoi terrain area polygons for texturing (`[]` when terrain or lattice is missing).
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

    /// Combined terrain mesh arrays for Godot rendering.
    ///
    /// Returns `[vertices, colors, normals, indices]` or an empty array on failure.
    #[func]
    fn get_terrain_mesh_data(&mut self) -> Array<Variant> {
        self.rebuild_derived_cache();
        let Some(cache) = self.derived_cache.as_ref() else {
            return Array::new();
        };
        combined_mesh_to_godot(&cache.terrain_mesh)
    }

    /// Coastline segment endpoints (`[start, end, ...]`) deduplicated in Rust.
    #[func]
    fn get_coastline_segments(&mut self) -> PackedVector3Array {
        self.rebuild_derived_cache();
        let Some(cache) = self.derived_cache.as_ref() else {
            return PackedVector3Array::new();
        };
        positions_to_packed(&cache.coastline_segments)
    }

    /// Build a ribbon triangle mesh from paired segment endpoints.
    ///
    /// Returns `[vertices, indices]` or an empty array when input is invalid.
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

    /// Configure a [`MultiMesh`] with one instance per lattice vertex.
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

    /// Update highlight colors on an already populated point [`MultiMesh`].
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

    /// Classify the border between two terrain type indices.
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

    /// Returns true when the border between two terrain types crosses sea level.
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

    /// Angular distance from a vertex to the north pole, in radians (`-1.0` on error).
    #[func]
    fn angular_distance_to_north_pole(&self, vertex_index: i32) -> f32 {
        self.angular_distance_at(vertex_index, |lattice, index| {
            lattice.angular_distance_to_north_pole_at(index)
        })
    }

    /// Angular distance from a vertex to the south pole, in radians (`-1.0` on error).
    #[func]
    fn angular_distance_to_south_pole(&self, vertex_index: i32) -> f32 {
        self.angular_distance_at(vertex_index, |lattice, index| {
            lattice.angular_distance_to_south_pole_at(index)
        })
    }

    /// Angular distance from a vertex to the equator, in radians (`-1.0` on error).
    #[func]
    fn angular_distance_to_equator(&self, vertex_index: i32) -> f32 {
        self.angular_distance_at(vertex_index, |lattice, index| {
            lattice.angular_distance_to_equator_at(index)
        })
    }

    /// Vertex indices within `max_angle` radians of the north pole.
    #[func]
    fn vertices_within_north_polar_distance(&self, max_angle: f64) -> PackedInt32Array {
        self.lattice
            .as_ref()
            .map(|lattice| {
                indices_to_packed(lattice.vertices_within_north_polar_distance(max_angle))
            })
            .unwrap_or_default()
    }

    /// Vertex indices within `max_angle` radians of the south pole.
    #[func]
    fn vertices_within_south_polar_distance(&self, max_angle: f64) -> PackedInt32Array {
        self.lattice
            .as_ref()
            .map(|lattice| {
                indices_to_packed(lattice.vertices_within_south_polar_distance(max_angle))
            })
            .unwrap_or_default()
    }

    /// Vertex indices within `max_angle` radians of the equator.
    #[func]
    fn vertices_within_equatorial_distance(&self, max_angle: f64) -> PackedInt32Array {
        self.lattice
            .as_ref()
            .map(|lattice| {
                indices_to_packed(lattice.vertices_within_equatorial_distance(max_angle))
            })
            .unwrap_or_default()
    }

    /// Index of the lattice vertex nearest to a world-space position (`-1` on error).
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

    /// World-space fan apex for triangulating a Voronoi terrain cell mesh.
    ///
    /// Polar-extreme cells use the geographic pole; all others use the generator site.
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

    /// Vertex indices along the shortest surface path (`[]` on error).
    #[func]
    fn shortest_surface_path_indices(&self, from_index: i32, to_index: i32) -> PackedInt32Array {
        self.shortest_surface_path_indices_with_allowed_terrain(
            from_index,
            to_index,
            PackedInt32Array::new(),
        )
    }

    /// Vertex indices along the shortest surface path restricted to allowed terrain types.
    ///
    /// Pass an empty `allowed_terrain_types` array to allow every terrain type.
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

    /// World positions along the shortest surface path (`[]` on error).
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

    /// World positions along the shortest surface path restricted to allowed terrain types.
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

    /// Total geodesic length of the shortest surface path (`-1.0` on error).
    #[func]
    fn shortest_surface_path_length(&self, from_index: i32, to_index: i32) -> f32 {
        self.shortest_surface_path_length_with_allowed_terrain(
            from_index,
            to_index,
            PackedInt32Array::new(),
        )
    }

    /// Total geodesic length of the shortest surface path restricted to allowed terrain types.
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

    /// Build fan-triangulated mesh data for one Voronoi cell.
    ///
    /// Returns `[vertices: PackedVector3Array, indices: PackedInt32Array]` or an empty array on failure.
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

    /// Number of points in the cached lattice (0 if not generated yet).
    #[func]
    fn get_point_count(&self) -> i32 {
        self.lattice.as_ref().map(|l| l.len() as i32).unwrap_or(0)
    }

    /// Radius used for the cached lattice.
    #[func]
    fn get_radius(&self) -> f32 {
        self.lattice
            .as_ref()
            .map(|l| l.radius() as f32)
            .unwrap_or(0.0)
    }

    /// Godot method index of the cached lattice (`-1` if not generated).
    #[func]
    fn get_method_index(&self) -> i32 {
        self.lattice
            .as_ref()
            .map(|l| l.method().to_godot_index())
            .unwrap_or(-1)
    }

    /// Clears the cached lattice and terrain.
    #[func]
    fn clear(&mut self) {
        self.lattice = None;
        self.surface_graph = None;
        self.terrain = None;
        self.derived_cache = None;
    }

    /// One-shot helper: generate and return positions without retaining state.
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

    /// Multi-line literature-backed description for a method index.
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
