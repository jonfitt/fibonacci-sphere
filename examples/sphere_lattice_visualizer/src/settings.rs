//! Application settings and distribution method selection.

use bevy::prelude::*;
use fibonacci_sphere::DistributionMethod;

/// All distribution methods in display order.
pub const METHODS: [DistributionMethod; 6] = DistributionMethod::ALL;

/// User-adjustable visualization parameters.
#[derive(Resource)]
pub struct VizSettings {
    /// Index into [`METHODS`].
    pub method_index: usize,
    /// Number of sample points on the sphere.
    pub point_count: usize,
    /// Sphere radius in world units.
    pub radius: f32,
    /// Whether the Delaunay wireframe is drawn.
    pub show_wireframe: bool,
    /// Draw Voronoi cell border lines.
    pub show_voronoi_borders: bool,
    /// Fill Voronoi cells with terrain color; nodes render black and wireframe white.
    pub show_voronoi_cell_shading: bool,
    /// Seed for terrain generation; change to produce a new random layout.
    pub terrain_seed: u64,
    /// Perlin noise value at or above which vertices become mountain.
    pub perlin_mountain_threshold: f32,
    /// Perlin noise fraction below sea level where deep water begins.
    pub perlin_deep_water_threshold: f32,
    /// Perlin noise frequency scale relative to average vertex spacing.
    pub perlin_spacing_factor: f32,
    /// Maximum angular reach of the north polar ice cap (radians).
    pub north_polar_ice_distance: f32,
    /// Maximum angular reach of the south polar ice cap (radians).
    pub south_polar_ice_distance: f32,
    /// Flood-fill resistance on mountains (lower = spidrier caps).
    pub polar_ice_mountain_resistance: f32,
    /// Flood-fill resistance on land.
    pub polar_ice_land_resistance: f32,
    /// Flood-fill resistance on shallow water.
    pub polar_ice_water_resistance: f32,
    /// Flood-fill resistance on deep water.
    pub polar_ice_deep_water_resistance: f32,
    /// Added flood cost per geodesic edge (higher = rounder caps).
    pub polar_ice_latitude_cost: f32,
}

impl Default for VizSettings {
    fn default() -> Self {
        Self {
            method_index: 0,
            point_count: 6000,
            radius: 1.0,
            show_wireframe: true,
            show_voronoi_borders: false,
            show_voronoi_cell_shading: false,
            terrain_seed: 1,
            perlin_mountain_threshold: 0.55,
            perlin_deep_water_threshold: 0.5,
            perlin_spacing_factor: 0.2,
            north_polar_ice_distance: 0.25,
            south_polar_ice_distance: 0.25,
            polar_ice_mountain_resistance: 0.25,
            polar_ice_land_resistance: 1.0,
            polar_ice_water_resistance: 2.5,
            polar_ice_deep_water_resistance: 5.0,
            polar_ice_latitude_cost: 2.0,
        }
    }
}

impl VizSettings {
    /// Currently selected distribution method.
    pub fn method(&self) -> DistributionMethod {
        METHODS[self.method_index]
    }
}

/// Snapshot of [`VizSettings`] fields that require a lattice rebuild when changed.
#[derive(Clone, PartialEq, Eq)]
pub struct LatticeSyncKey {
    method_index: usize,
    point_count: usize,
    radius_bits: u32,
    show_voronoi_cell_shading: bool,
    terrain_seed: u64,
    perlin_mountain_threshold_bits: u32,
    perlin_deep_water_threshold_bits: u32,
    perlin_spacing_factor_bits: u32,
    north_polar_ice_distance_bits: u32,
    south_polar_ice_distance_bits: u32,
    polar_ice_mountain_resistance_bits: u32,
    polar_ice_land_resistance_bits: u32,
    polar_ice_water_resistance_bits: u32,
    polar_ice_deep_water_resistance_bits: u32,
    polar_ice_latitude_cost_bits: u32,
}

impl From<&VizSettings> for LatticeSyncKey {
    fn from(settings: &VizSettings) -> Self {
        Self {
            method_index: settings.method_index,
            point_count: settings.point_count,
            radius_bits: settings.radius.to_bits(),
            show_voronoi_cell_shading: settings.show_voronoi_cell_shading,
            terrain_seed: settings.terrain_seed,
            perlin_mountain_threshold_bits: settings.perlin_mountain_threshold.to_bits(),
            perlin_deep_water_threshold_bits: settings.perlin_deep_water_threshold.to_bits(),
            perlin_spacing_factor_bits: settings.perlin_spacing_factor.to_bits(),
            north_polar_ice_distance_bits: settings.north_polar_ice_distance.to_bits(),
            south_polar_ice_distance_bits: settings.south_polar_ice_distance.to_bits(),
            polar_ice_mountain_resistance_bits: settings.polar_ice_mountain_resistance.to_bits(),
            polar_ice_land_resistance_bits: settings.polar_ice_land_resistance.to_bits(),
            polar_ice_water_resistance_bits: settings.polar_ice_water_resistance.to_bits(),
            polar_ice_deep_water_resistance_bits: settings
                .polar_ice_deep_water_resistance
                .to_bits(),
            polar_ice_latitude_cost_bits: settings.polar_ice_latitude_cost.to_bits(),
        }
    }
}

/// Last lattice inputs applied by [`crate::lattice::sync_lattice`].
#[derive(Resource, Default)]
pub struct LatticeSyncState {
    pub last: Option<LatticeSyncKey>,
}

/// Brightness multiplier from world-space distance to the camera.
///
/// Maps the expected near/far distances for the current view so the near hemisphere
/// stays bright and the far side falls off toward [`MIN_BRIGHTNESS`].
pub fn brightness_at_distance(distance: f32, camera_distance: f32, sphere_radius: f32) -> f32 {
    const MIN_BRIGHTNESS: f32 = 0.08;
    let near = (camera_distance - sphere_radius * 1.1).max(0.05);
    let far = camera_distance + sphere_radius * 1.1;
    let t = ((distance - near) / (far - near).max(0.001)).clamp(0.0, 1.0);
    1.0 - t * (1.0 - MIN_BRIGHTNESS)
}

/// Scales an sRGB color by a brightness factor.
pub fn fade_color(color: Color, brightness: f32) -> Color {
    let linear = color.to_linear();
    Color::linear_rgba(
        linear.red * brightness,
        linear.green * brightness,
        linear.blue * brightness,
        linear.alpha * brightness,
    )
}

/// Scales an sRGB color by a brightness factor, clamped to a minimum multiplier.
pub fn fade_color_with_floor(color: Color, brightness: f32, min_brightness: f32) -> Color {
    fade_color(color, brightness.max(min_brightness))
}
