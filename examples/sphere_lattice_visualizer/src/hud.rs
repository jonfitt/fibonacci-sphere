//! HUD font loading and method description formatting.

use bevy::prelude::*;
use dejavu::sans;
use fibonacci_sphere::{MethodInfo, OptimizationGoal};

use crate::settings::VizSettings;

/// Loads DejaVu Sans for the HUD. Bevy's default font is a minimal Latin subset and
/// cannot render symbols such as φ, π, ε, or δ used in method descriptions.
pub fn load_hud_font(fonts: &mut Assets<Font>) -> Handle<Font> {
    let font = Font::try_from_bytes(sans::regular().to_vec())
        .expect("embedded DejaVu Sans is a valid TTF");
    fonts.add(font)
}

/// Formats [`MethodInfo`] for the on-screen HUD.
pub fn format_method_info(info: &MethodInfo, goal: OptimizationGoal) -> String {
    info.format_description(goal)
}

/// Marker for the HUD text entity.
#[derive(Component)]
pub struct HudText;

/// Spawns the UI overlay camera and HUD panel.
pub fn setup_hud(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    let hud_font = load_hud_font(&mut fonts);

    commands.spawn((
        Camera2d,
        IsDefaultUiCamera,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));

    commands.spawn((
        Text::new("Loading..."),
        TextFont {
            font: hud_font,
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.92, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            max_width: Val::Px(520.0),
            ..default()
        },
        HudText,
    ));
}

/// Refreshes HUD text when settings change.
pub fn update_hud(settings: Res<VizSettings>, mut query: Query<&mut Text, With<HudText>>) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    let method = settings.method();
    let info = method.info();
    let perlin_controls = format!(
        "\nPerlin mountain split: {:.0}% land / {:.0}% mountain (above sea level)\n\
         Perlin deep-water split: {:.0}% shallow / {:.0}% deep (below sea level)\n\
         Perlin spacing factor: {:.2}\n\
         Polar ice distance (N/S): {:.2} / {:.2} rad\n\
         Polar ice flood (mountain / water / latitude cost): {:.2} / {:.2} / {:.2}\n\
         ,/.: mountain split  9/0: deep-water split  ;/': spacing factor\n\
         1/2: north ice  3/4: south ice  5/6: mountain resist  7/8: water resist  Z/X: latitude cost",
        settings.perlin_mountain_threshold * 100.0,
        (1.0 - settings.perlin_mountain_threshold) * 100.0,
        (1.0 - settings.perlin_deep_water_threshold) * 100.0,
        settings.perlin_deep_water_threshold * 100.0,
        settings.perlin_spacing_factor,
        settings.north_polar_ice_distance,
        settings.south_polar_ice_distance,
        settings.polar_ice_mountain_resistance,
        settings.polar_ice_water_resistance,
        settings.polar_ice_latitude_cost,
    );
    **text = format!(
        "{}\n\n---\nPoints: {}  Radius: {:.1}  Wireframe: {}  Voronoi borders: {}  Voronoi fill: {}  Seed: {}{}\n\
         Colors: land (green)  shallow water (blue)  deep water (dark blue)  mountain (red)  ice (pale)  ice mountain (blue-white)\n\
         Voronoi fill (C): shaded cells, black nodes, white wireframe\n\
         Axes: Y-up (RGB = XYZ)\n\n\
         M: method  +/-: count  [/]: radius  H: wireframe  B: Voronoi borders  C: Voronoi fill  R: new seed",
        format_method_info(info, method.optimizes()),
        settings.point_count,
        settings.radius,
        if settings.show_wireframe { "on" } else { "off" },
        if settings.show_voronoi_borders { "on" } else { "off" },
        if settings.show_voronoi_cell_shading { "on" } else { "off" },
        settings.terrain_seed,
        perlin_controls,
    );
}
