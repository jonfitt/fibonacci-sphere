//! Bevy plugin wiring for the Fibonacci sphere visualizer.

use bevy::prelude::*;

use crate::camera::{orbit_camera, setup_camera};
use crate::controls::keyboard_controls;
use crate::gizmos::{
    draw_axes, draw_delaunay_wireframe, draw_polar_ice_circles, draw_voronoi_borders,
};
use crate::hud::{setup_hud, update_hud};
use crate::lattice::{
    DelaunayWireframe, VoronoiBorderWireframe, apply_distance_fade, sync_lattice,
};
use crate::settings::{LatticeSyncState, VizSettings};

/// Registers resources, startup systems, and update systems for the visualizer.
pub struct VizPlugin;

impl Plugin for VizPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 500.0,
            ..default()
        })
        .init_resource::<VizSettings>()
        .init_resource::<LatticeSyncState>()
        .init_resource::<DelaunayWireframe>()
        .init_resource::<VoronoiBorderWireframe>()
        .add_systems(Startup, (setup_camera, setup_hud))
        .add_systems(
            Update,
            (
                keyboard_controls,
                orbit_camera,
                sync_lattice,
                apply_distance_fade,
                draw_axes,
                draw_delaunay_wireframe,
                draw_polar_ice_circles,
                draw_voronoi_borders,
                update_hud,
            ),
        );
    }
}
