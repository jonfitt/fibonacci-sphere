# Fibonacci Sphere GDExtension

Godot 4 addon for spherical lattice generation, Delaunay/Voronoi mesh data, Perlin terrain, and
pathfinding. Coordinates are Y-up, right-handed, matching Godot 4's default 3D frame.

## Installation

1. Download `fibonacci_sphere-<version>.zip` from
   [GitHub Releases](https://github.com/jonfitt/fibonacci-sphere/releases).
2. Extract the `fibonacci_sphere` folder into your Godot project's `addons/` directory so the
   layout is `addons/fibonacci_sphere/fibonacci_sphere.gdextension`.
3. Open your project in Godot. The extension loads automatically when Godot finds the
   `.gdextension` file.

## Requirements

- Godot 4.3 or later (4.3+ for in-editor API documentation from Rust `///` comments).
- Supported platforms: Linux (x86_64), Windows (x86_64), macOS (Apple Silicon).

## Demo

Download `fibonacci_sphere-demo-<version>.zip` from Releases, extract anywhere, and open
`project.godot` in Godot 4.

Full API reference: [docs/godot.md](https://github.com/jonfitt/fibonacci-sphere/blob/main/docs/godot.md)
