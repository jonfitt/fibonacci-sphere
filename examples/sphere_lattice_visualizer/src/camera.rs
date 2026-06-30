//! Orbit camera for inspecting the sphere.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

/// Marks the 3D camera and stores orbit parameters.
#[derive(Component)]
pub struct OrbitCamera {
    /// Horizontal rotation around Y.
    pub yaw: f32,
    /// Vertical tilt.
    pub pitch: f32,
    /// Distance from the origin.
    pub distance: f32,
}

/// Spawns the 3D orbit camera and scene lighting.
pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.08, 0.09, 0.12)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        OrbitCamera {
            yaw: 0.0,
            pitch: 0.2,
            distance: 3.0,
        },
    ));
}

/// Drag to orbit, scroll to zoom.
pub fn orbit_camera(
    mut motion: EventReader<MouseMotion>,
    mut scroll: EventReader<MouseWheel>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut Transform, &mut OrbitCamera)>,
) {
    let Ok((mut transform, mut orbit)) = query.single_mut() else {
        return;
    };

    if mouse.pressed(MouseButton::Left) {
        for ev in motion.read() {
            orbit.yaw -= ev.delta.x * 0.005;
            orbit.pitch = (orbit.pitch - ev.delta.y * 0.005).clamp(-1.4, 1.4);
        }
    }

    for ev in scroll.read() {
        orbit.distance = (orbit.distance - ev.y * 0.2).clamp(1.0, 10.0);
    }

    let rot = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    *transform = Transform::from_translation(rot * Vec3::new(0.0, 0.0, orbit.distance))
        .looking_at(Vec3::ZERO, Vec3::Y);
}
