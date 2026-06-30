//! Point cloud spawning and Delaunay wireframe cache.

use bevy::{
    prelude::*,
    render::{
        mesh::Indices,
        render_asset::RenderAssetUsages,
        render_resource::PrimitiveTopology,
    },
};
use fibonacci_sphere::{
    build_combined_terrain_mesh, spherical_voronoi_border_segments, CombinedTerrainMesh,
    CombinedTerrainMeshOptions, PerlinNoiseConfig, SphereLattice, TerrainType,
    VoronoiFanMeshOptions,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::camera::OrbitCamera;
use crate::settings::{brightness_at_distance, fade_color, LatticeSyncKey, LatticeSyncState, VizSettings};

/// Root entity for all lattice point meshes.
#[derive(Component)]
pub struct LatticeRoot;

/// Per-vertex material state for distance-based dimming.
#[derive(Component)]
pub struct LatticePoint {
    base_color: Color,
    material: Handle<StandardMaterial>,
}

/// Combined Voronoi terrain surface with distance-based dimming.
#[derive(Component)]
pub struct ShadedTerrainSurface {
    base_color: Color,
    material: Handle<StandardMaterial>,
    fade_center: Vec3,
}

/// Cached Delaunay edge segments in world space for wireframe drawing.
#[derive(Resource, Default)]
pub struct DelaunayWireframe {
    /// Line segments as world-space start/end pairs.
    pub segments: Vec<(Vec3, Vec3)>,
}

/// Cached Voronoi border segments in world space.
#[derive(Resource, Default)]
pub struct VoronoiBorderWireframe {
    /// Line segments as world-space start/end pairs.
    pub segments: Vec<(Vec3, Vec3)>,
}

/// Fraction of mean Delaunay edge length used for node sphere radius in each mode.
const NODE_RADIUS_FRACTION: f32 = 0.16;
const NODE_RADIUS_FRACTION_SHADED: f32 = 0.12;
const MIN_NODE_RADIUS_FRACTION: f32 = 0.05;
const MAX_NODE_RADIUS_FRACTION: f32 = 0.24;

pub fn terrain_color(terrain: TerrainType) -> Color {
    match terrain {
        TerrainType::Land => Color::srgb(0.18, 0.88, 0.24),
        TerrainType::Water => Color::srgb(0.22, 0.52, 0.95),
        TerrainType::DeepWater => Color::srgb(0.04, 0.12, 0.45),
        TerrainType::Mountain => Color::srgb(0.85, 0.22, 0.18),
        TerrainType::Ice => Color::srgb(0.82, 0.92, 0.98),
        TerrainType::IceMountain => Color::srgb(0.62, 0.78, 0.92),
    }
}

fn terrain_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        emissive: LinearRgba::from(color),
        unlit: true,
        ..default()
    }
}

fn shaded_cell_material(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        emissive: LinearRgba::from(color),
        unlit: true,
        double_sided: false,
        cull_mode: Some(bevy::render::render_resource::Face::Back),
        ..default()
    }
}

fn node_dot_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(0.02, 0.02, 0.02),
        emissive: LinearRgba::new(0.15, 0.15, 0.15, 1.0),
        unlit: true,
        ..default()
    }
}

fn generate_terrain(
    lattice: &SphereLattice,
    settings: &VizSettings,
    rng: &mut StdRng,
) -> fibonacci_sphere::TerrainMap {
    lattice.generate_terrain(
        PerlinNoiseConfig {
            mountain_threshold: f64::from(settings.perlin_mountain_threshold),
            deep_water_threshold: f64::from(settings.perlin_deep_water_threshold),
            spacing_factor: f64::from(settings.perlin_spacing_factor),
            seed: Some(settings.terrain_seed as u32),
            north_polar_ice_distance: f64::from(settings.north_polar_ice_distance),
            south_polar_ice_distance: f64::from(settings.south_polar_ice_distance),
            polar_ice_mountain_resistance: f64::from(settings.polar_ice_mountain_resistance),
            polar_ice_land_resistance: f64::from(settings.polar_ice_land_resistance),
            polar_ice_water_resistance: f64::from(settings.polar_ice_water_resistance),
            polar_ice_deep_water_resistance: f64::from(settings.polar_ice_deep_water_resistance),
            polar_ice_latitude_cost: f64::from(settings.polar_ice_latitude_cost),
        },
        rng,
    )
}

fn average_edge_length(segments: &[(Vec3, Vec3)]) -> f32 {
    if segments.is_empty() {
        return 0.0;
    }

    let total = segments
        .iter()
        .map(|(start, end)| start.distance(*end))
        .sum::<f32>();
    total / segments.len() as f32
}

fn node_sphere_radius(mean_edge_length: f32, shaded: bool) -> f32 {
    if mean_edge_length <= f32::EPSILON {
        return 0.02;
    }

    let target_fraction = if shaded {
        NODE_RADIUS_FRACTION_SHADED
    } else {
        NODE_RADIUS_FRACTION
    };
    let radius = mean_edge_length * target_fraction;
    let min_radius = mean_edge_length * MIN_NODE_RADIUS_FRACTION;
    let max_radius = mean_edge_length * MAX_NODE_RADIUS_FRACTION;
    radius.clamp(min_radius, max_radius)
}

/// Radial offset so gizmo lines and nodes sit above shaded cell geometry.
fn surface_outward_lift(mean_edge_length: f32, shaded: bool) -> f32 {
    if !shaded || mean_edge_length <= f32::EPSILON {
        return 0.0;
    }

    mean_edge_length * 0.4
}

fn outward_lift(point: Vec3, distance: f32) -> Vec3 {
    if distance <= f32::EPSILON {
        return point;
    }

    let normal = if point.length_squared() > f32::EPSILON {
        point.normalize()
    } else {
        Vec3::Y
    };
    point + normal * distance
}

fn push_voronoi_borders(
    segments: &mut Vec<(Vec3, Vec3)>,
    positions: &[[f32; 3]],
    mesh: &fibonacci_sphere::SphericalMesh,
) {
    let radius = positions
        .first()
        .map(|position| Vec3::from_array(*position).length())
        .unwrap_or(1.0);
    for (start, end) in spherical_voronoi_border_segments(positions, mesh) {
        segments.push((
            Vec3::from_array(start) * radius,
            Vec3::from_array(end) * radius,
        ));
    }
}

const CELL_SURFACE_INSET: f32 = 0.997;
const EDGE_SUBDIVISIONS: u32 = 3;

fn build_combined_bevy_mesh(combined: &CombinedTerrainMesh) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, combined.vertices.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, combined.normals.clone());
    mesh.insert_indices(Indices::U32(combined.indices.clone()));
    mesh
}

fn spawn_combined_terrain_shading(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    lattice: &SphereLattice,
    terrain: &fibonacci_sphere::TerrainMap,
) {
    let options = CombinedTerrainMeshOptions {
        fan_mesh: VoronoiFanMeshOptions {
            surface_inset: CELL_SURFACE_INSET,
            edge_subdivisions: EDGE_SUBDIVISIONS,
            flip_winding: false,
        },
    };

    let positions = lattice.position_arrays();
    let polygons = lattice.terrain_area_polygons(terrain);
    let radius = lattice.radius() as f32;

    for terrain_type in TerrainType::ALL {
        let filtered: Vec<_> = polygons
            .iter()
            .filter(|polygon| polygon.terrain_type == terrain_type)
            .cloned()
            .collect();
        if filtered.is_empty() {
            continue;
        }

        let combined = build_combined_terrain_mesh(&filtered, &positions, radius, options);
        if combined.vertices.is_empty() {
            continue;
        }

        let fade_center = combined
            .vertices
            .iter()
            .fold(Vec3::ZERO, |sum, vertex| sum + Vec3::from_array(*vertex))
            / combined.vertices.len() as f32;
        let base_color = terrain_color(terrain_type);
        let material = materials.add(shaded_cell_material(base_color));

        parent.spawn((
            Mesh3d(meshes.add(build_combined_bevy_mesh(&combined))),
            MeshMaterial3d(material.clone()),
            Transform::default(),
            ShadedTerrainSurface {
                base_color,
                material,
                fade_center,
            },
        ));
    }
}

/// Rebuilds point meshes and wireframe cache when lattice-affecting settings change.
#[allow(clippy::too_many_arguments)]
pub fn sync_lattice(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<VizSettings>,
    mut sync_state: ResMut<LatticeSyncState>,
    mut wireframe: ResMut<DelaunayWireframe>,
    mut area_borders: ResMut<VoronoiBorderWireframe>,
    roots: Query<Entity, With<LatticeRoot>>,
) {
    let key = LatticeSyncKey::from(&*settings);
    if sync_state.last.as_ref() == Some(&key) {
        return;
    }
    sync_state.last = Some(key);

    for entity in &roots {
        commands.entity(entity).despawn();
    }

    let method = settings.method();
    let lattice =
        SphereLattice::generate(method, settings.point_count, settings.radius as f64).unwrap();

    wireframe.segments.clear();
    area_borders.segments.clear();
    let points = lattice.position_arrays();
    let mesh = lattice.spherical_mesh();
    let shaded = settings.show_voronoi_cell_shading;

    let mut raw_wireframe = Vec::new();
    for [a, b] in lattice.wireframe_edges() {
        raw_wireframe.push((
            Vec3::from_array(points[a]),
            Vec3::from_array(points[b]),
        ));
    }

    let mean_edge_length = average_edge_length(&raw_wireframe);
    let surface_lift = surface_outward_lift(mean_edge_length, shaded);
    for (start, end) in raw_wireframe {
        wireframe.segments.push((
            outward_lift(start, surface_lift),
            outward_lift(end, surface_lift),
        ));
    }

    let mut rng = StdRng::seed_from_u64(settings.terrain_seed);
    let terrain = generate_terrain(&lattice, &settings, &mut rng);
    push_voronoi_borders(&mut area_borders.segments, &points, &mesh);

    let node_radius = node_sphere_radius(mean_edge_length, shaded);
    let dot_mesh = meshes.add(Sphere::new(node_radius).mesh().ico(2).unwrap());
    let black_dot_material = materials.add(node_dot_material());

    commands
        .spawn((Transform::default(), Visibility::default(), LatticeRoot))
        .with_children(|parent| {
            if shaded {
                spawn_combined_terrain_shading(
                    parent,
                    &mut meshes,
                    &mut materials,
                    &lattice,
                    &terrain,
                );
            }

            for point in lattice.iter() {
                let base_color = terrain_color(terrain.get(point.index));
                let material = if shaded {
                    black_dot_material.clone()
                } else {
                    materials.add(terrain_material(base_color))
                };
                let position = Vec3::from_array(point.position);
                let position = outward_lift(position, surface_lift);
                parent.spawn((
                    Mesh3d(dot_mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(position),
                    LatticePoint {
                        base_color: if shaded {
                            Color::srgb(0.02, 0.02, 0.02)
                        } else {
                            base_color
                        },
                        material,
                    },
                ));
            }
        });
}

/// Dims vertex colors based on distance from the orbit camera.
pub fn apply_distance_fade(
    settings: Res<VizSettings>,
    camera: Query<(&Transform, &OrbitCamera)>,
    points: Query<(&GlobalTransform, &LatticePoint)>,
    cells: Query<(&ShadedTerrainSurface,)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((camera_transform, orbit)) = camera.single() else {
        return;
    };
    let camera_position = camera_transform.translation;
    let shaded = settings.show_voronoi_cell_shading;

    for (transform, point) in &points {
        let world_position = transform.translation();
        let distance = world_position.distance(camera_position);
        let brightness = brightness_at_distance(distance, orbit.distance, settings.radius);
        let faded = if shaded {
            fade_color(Color::srgb(0.02, 0.02, 0.02), brightness)
        } else {
            fade_color(point.base_color, brightness)
        };

        if let Some(material) = materials.get_mut(&point.material) {
            material.base_color = faded;
            material.emissive = LinearRgba::from(faded);
        }
    }

    for (surface,) in &cells {
        let distance = surface.fade_center.distance(camera_position);
        let brightness = brightness_at_distance(distance, orbit.distance, settings.radius);
        let faded = fade_color(surface.base_color, brightness);

        if let Some(material) = materials.get_mut(&surface.material) {
            material.base_color = faded;
            material.emissive = LinearRgba::from(faded);
        }
    }
}
