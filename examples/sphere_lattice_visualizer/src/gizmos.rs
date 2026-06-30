//! Debug gizmo drawing for axes and wireframe.

use bevy::prelude::*;
use fibonacci_sphere::{outward_lift, polar_cap_circle_segments};

use crate::camera::OrbitCamera;
use crate::lattice::{DelaunayWireframe, VoronoiBorderWireframe};
use crate::settings::{VizSettings, brightness_at_distance, fade_color, fade_color_with_floor};

const WIREFRAME_COLOR: Color = Color::srgba(0.35, 0.75, 1.0, 0.85);
const WIREFRAME_COLOR_SHADED: Color = Color::srgba(0.98, 0.98, 0.98, 1.0);
const WIREFRAME_SHADED_MIN_BRIGHTNESS: f32 = 0.55;
const VORONOI_BORDER_COLOR: Color = Color::srgba(0.95, 0.45, 1.0, 0.9);
const POLAR_ICE_CIRCLE_SEGMENTS: usize = 96;
const POLAR_ICE_LIFT_FRACTION: f32 = 0.004;
const NORTH_POLAR_ICE_COLOR: Color = Color::srgba(0.55, 0.92, 1.0, 0.95);
const SOUTH_POLAR_ICE_COLOR: Color = Color::srgba(0.45, 0.78, 1.0, 0.95);

/// Draws RGB axis lines (Y-up).
pub fn draw_axes(settings: Res<VizSettings>, mut gizmos: Gizmos) {
    let length = settings.radius * 1.5 + 0.15;
    let origin = Vec3::ZERO;

    gizmos.line(origin, Vec3::X * length, Color::srgb(0.95, 0.25, 0.25));
    gizmos.line(origin, Vec3::Y * length, Color::srgb(0.25, 0.9, 0.3));
    gizmos.line(origin, Vec3::Z * length, Color::srgb(0.35, 0.55, 1.0));
}

/// Draws cached Delaunay wireframe segments.
pub fn draw_delaunay_wireframe(
    settings: Res<VizSettings>,
    wireframe: Res<DelaunayWireframe>,
    camera: Query<(&Transform, &OrbitCamera)>,
    mut gizmos: Gizmos,
) {
    if !settings.show_wireframe {
        return;
    }

    let Ok((camera_transform, orbit)) = camera.single() else {
        return;
    };
    let camera_position = camera_transform.translation;
    let shaded = settings.show_voronoi_cell_shading;
    let base_wireframe = if shaded {
        WIREFRAME_COLOR_SHADED
    } else {
        WIREFRAME_COLOR
    };

    for (start, end) in &wireframe.segments {
        let start_distance = start.distance(camera_position);
        let end_distance = end.distance(camera_position);
        let distance = (start_distance + end_distance) * 0.5;
        let brightness = brightness_at_distance(distance, orbit.distance, settings.radius);
        let color = if shaded {
            fade_color_with_floor(base_wireframe, brightness, WIREFRAME_SHADED_MIN_BRIGHTNESS)
        } else {
            fade_color(base_wireframe, brightness)
        };
        gizmos.line(*start, *end, color);
    }
}

/// Draws polar ice cap boundary circles for configured north/south radii and morphology limits.
pub fn draw_polar_ice_circles(
    settings: Res<VizSettings>,
    camera: Query<(&Transform, &OrbitCamera)>,
    mut gizmos: Gizmos,
) {
    let Ok((camera_transform, orbit)) = camera.single() else {
        return;
    };

    let camera_position = camera_transform.translation;
    let lift = settings.radius * POLAR_ICE_LIFT_FRACTION;
    let shaded = settings.show_voronoi_cell_shading;

    let mut draw_circle = |south: bool, angular_distance: f32, base_color: Color| {
        let angular_distance = angular_distance.clamp(0.0, std::f32::consts::FRAC_PI_2);
        if angular_distance <= f32::EPSILON {
            return;
        }

        let segments = polar_cap_circle_segments(
            south,
            f64::from(angular_distance),
            settings.radius,
            POLAR_ICE_CIRCLE_SEGMENTS,
        );
        for (start, end) in segments {
            let start = Vec3::from_array(outward_lift(start, lift));
            let end = Vec3::from_array(outward_lift(end, lift));
            let distance = (start.distance(camera_position) + end.distance(camera_position)) * 0.5;
            let brightness = brightness_at_distance(distance, orbit.distance, settings.radius);
            let color = if shaded {
                fade_color_with_floor(base_color, brightness, WIREFRAME_SHADED_MIN_BRIGHTNESS)
            } else {
                fade_color(base_color, brightness)
            };
            gizmos.line(start, end, color);
        }
    };

    draw_circle(
        false,
        settings.north_polar_ice_distance,
        NORTH_POLAR_ICE_COLOR,
    );
    draw_circle(
        true,
        settings.south_polar_ice_distance,
        SOUTH_POLAR_ICE_COLOR,
    );
}

/// Draws borders between Voronoi cells.
pub fn draw_voronoi_borders(
    settings: Res<VizSettings>,
    borders: Res<VoronoiBorderWireframe>,
    camera: Query<(&Transform, &OrbitCamera)>,
    mut gizmos: Gizmos,
) {
    if !settings.show_voronoi_borders {
        return;
    }

    let Ok((camera_transform, orbit)) = camera.single() else {
        return;
    };
    let camera_position = camera_transform.translation;

    for (start, end) in &borders.segments {
        let start_distance = start.distance(camera_position);
        let end_distance = end.distance(camera_position);
        let distance = (start_distance + end_distance) * 0.5;
        let brightness = brightness_at_distance(distance, orbit.distance, settings.radius);
        let color = fade_color(VORONOI_BORDER_COLOR, brightness);
        gizmos.line(*start, *end, color);
    }
}
