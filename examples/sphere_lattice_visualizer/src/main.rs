//! Interactive visualizer for Fibonacci sphere lattice distributions.
//!
//! Run with: `cargo run -p sphere_lattice_visualizer`
//!
//! Controls:
//! - M: cycle distribution method
//! - +/-: increase/decrease point count
//! - [/]: decrease/increase radius
//! - H: toggle Delaunay wireframe
//! - B: toggle Voronoi cell borders
//! - C: toggle Voronoi cell shading (terrain fill, black nodes, white wireframe)
//! - ,/.: Perlin mountain threshold  ;/': Perlin spacing factor
//! - R: regenerate terrain (new seed)
//! - Left drag: orbit camera
//! - Scroll: zoom

mod camera;
mod controls;
mod gizmos;
mod hud;
mod lattice;
mod plugin;
mod settings;

use bevy::prelude::*;
use plugin::VizPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(VizPlugin)
        .run();
}
