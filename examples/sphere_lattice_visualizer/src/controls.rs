//! Keyboard controls for method, count, radius, and wireframe toggling.

use bevy::prelude::*;

use crate::settings::{METHODS, VizSettings};

const MOUNTAIN_THRESHOLD_STEP: f32 = 0.05;
const DEEP_WATER_THRESHOLD_STEP: f32 = 0.05;
const SPACING_FACTOR_STEP: f32 = 0.1;
const POLAR_ICE_DISTANCE_STEP: f32 = 0.05;
const POLAR_ICE_RESISTANCE_STEP: f32 = 0.1;
const POLAR_ICE_LATITUDE_STEP: f32 = 0.25;
const MAX_POLAR_ICE_DISTANCE: f32 = std::f32::consts::FRAC_PI_2;

/// Handles visualization hotkeys.
pub fn keyboard_controls(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<VizSettings>) {
    if keys.just_pressed(KeyCode::KeyR) {
        settings.terrain_seed = settings.terrain_seed.wrapping_add(1);
        return;
    }

    if keys.just_pressed(KeyCode::KeyH) {
        settings.show_wireframe = !settings.show_wireframe;
        return;
    }

    if keys.just_pressed(KeyCode::KeyB) {
        settings.show_voronoi_borders = !settings.show_voronoi_borders;
        return;
    }

    if keys.just_pressed(KeyCode::KeyC) {
        settings.show_voronoi_cell_shading = !settings.show_voronoi_cell_shading;
        return;
    }

    if keys.just_pressed(KeyCode::Comma) {
        settings.perlin_mountain_threshold =
            (settings.perlin_mountain_threshold - MOUNTAIN_THRESHOLD_STEP).clamp(0.05, 0.95);
        return;
    }
    if keys.just_pressed(KeyCode::Period) {
        settings.perlin_mountain_threshold =
            (settings.perlin_mountain_threshold + MOUNTAIN_THRESHOLD_STEP).clamp(0.05, 0.95);
        return;
    }
    if keys.just_pressed(KeyCode::Digit9) {
        settings.perlin_deep_water_threshold =
            (settings.perlin_deep_water_threshold - DEEP_WATER_THRESHOLD_STEP).clamp(0.05, 0.95);
        return;
    }
    if keys.just_pressed(KeyCode::Digit0) {
        settings.perlin_deep_water_threshold =
            (settings.perlin_deep_water_threshold + DEEP_WATER_THRESHOLD_STEP).clamp(0.05, 0.95);
        return;
    }
    if keys.just_pressed(KeyCode::Semicolon) {
        settings.perlin_spacing_factor =
            (settings.perlin_spacing_factor - SPACING_FACTOR_STEP).clamp(0.1, 5.0);
        return;
    }
    if keys.just_pressed(KeyCode::Quote) {
        settings.perlin_spacing_factor =
            (settings.perlin_spacing_factor + SPACING_FACTOR_STEP).clamp(0.1, 5.0);
        return;
    }
    if keys.just_pressed(KeyCode::Digit1) {
        settings.north_polar_ice_distance = (settings.north_polar_ice_distance
            - POLAR_ICE_DISTANCE_STEP)
            .clamp(0.0, MAX_POLAR_ICE_DISTANCE);
        return;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        settings.north_polar_ice_distance = (settings.north_polar_ice_distance
            + POLAR_ICE_DISTANCE_STEP)
            .clamp(0.0, MAX_POLAR_ICE_DISTANCE);
        return;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        settings.south_polar_ice_distance = (settings.south_polar_ice_distance
            - POLAR_ICE_DISTANCE_STEP)
            .clamp(0.0, MAX_POLAR_ICE_DISTANCE);
        return;
    }
    if keys.just_pressed(KeyCode::Digit4) {
        settings.south_polar_ice_distance = (settings.south_polar_ice_distance
            + POLAR_ICE_DISTANCE_STEP)
            .clamp(0.0, MAX_POLAR_ICE_DISTANCE);
        return;
    }
    if keys.just_pressed(KeyCode::Digit5) {
        settings.polar_ice_mountain_resistance =
            (settings.polar_ice_mountain_resistance - POLAR_ICE_RESISTANCE_STEP).clamp(0.05, 3.0);
        return;
    }
    if keys.just_pressed(KeyCode::Digit6) {
        settings.polar_ice_mountain_resistance =
            (settings.polar_ice_mountain_resistance + POLAR_ICE_RESISTANCE_STEP).clamp(0.05, 3.0);
        return;
    }
    if keys.just_pressed(KeyCode::Digit7) {
        settings.polar_ice_water_resistance =
            (settings.polar_ice_water_resistance - POLAR_ICE_RESISTANCE_STEP).clamp(0.5, 12.0);
        return;
    }
    if keys.just_pressed(KeyCode::Digit8) {
        settings.polar_ice_water_resistance =
            (settings.polar_ice_water_resistance + POLAR_ICE_RESISTANCE_STEP).clamp(0.5, 12.0);
        return;
    }
    if keys.just_pressed(KeyCode::KeyZ) {
        settings.polar_ice_latitude_cost =
            (settings.polar_ice_latitude_cost - POLAR_ICE_LATITUDE_STEP).clamp(0.0, 12.0);
        return;
    }
    if keys.just_pressed(KeyCode::KeyX) {
        settings.polar_ice_latitude_cost =
            (settings.polar_ice_latitude_cost + POLAR_ICE_LATITUDE_STEP).clamp(0.0, 12.0);
        return;
    }

    if keys.just_pressed(KeyCode::KeyM) {
        settings.method_index = (settings.method_index + 1) % METHODS.len();
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        settings.point_count = (settings.point_count + 10).min(5000);
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        settings.point_count = settings.point_count.saturating_sub(10).max(4);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        settings.radius = (settings.radius + 0.1).min(5.0);
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        settings.radius = (settings.radius - 0.1).max(0.2);
    }
}
