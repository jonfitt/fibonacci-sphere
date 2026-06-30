# Architecture and dependency boundaries

This document describes how the workspace is layered and how consumers should depend on each crate.

## Workspace overview

```text
fibonacci_sphere (core library)
    ├── topology/     Delaunay mesh, Voronoi cells, surface pathfinding, voronoi_mesh
    ├── terrain/      Perlin assignment, Voronoi areas, border classification
    ├── render/       Combined terrain meshes, coastline segments, line ribbons
    └── geography/    Pole/equator distances and polar/equatorial queries

sphere_lattice_visualizer (Bevy example binary)
    └── depends on fibonacci_sphere (render batch builders)

fibonacci_sphere_gd (Godot GDExtension cdylib)
    └── depends on fibonacci_sphere (render batch builders + cached derived meshes)
```

Coordinates are **Y-up, right-handed** everywhere, matching Godot 4's default 3D frame.

## Core library (`fibonacci_sphere`)

The core crate is the product: point generation, surface connectivity, terrain, and routing. Game engines and bindings should depend on this crate directly.

### Modules

| Module | Responsibility |
|--------|----------------|
| [`methods`](../src/methods/) | Six distribution algorithms and [`MethodInfo`](../src/methods/info.rs) metadata |
| [`point`](../src/point.rs) | [`SpherePoint`](../src/point.rs), golden-ratio constant, spherical ↔ Cartesian |
| [`lattice`](../src/lattice.rs) | [`SphereLattice`](../src/lattice.rs) — generation, wireframe, graph, terrain, geography |
| [`topology`](../src/topology/) | Spherical Delaunay, Voronoi, [`SurfaceGraph`](../src/topology/pathfinding.rs), [`voronoi_mesh`](../src/topology/voronoi_mesh.rs) |
| [`terrain`](../src/terrain/) | Perlin and other assigners, [`TerrainMap`](../src/terrain/mod.rs), area polygons, borders |
| [`render`](../src/render/) | Combined terrain meshes, coastline segments, line ribbon meshes (shared by Godot/Bevy) |
| [`geography`](../src/geography.rs) | Angular distances to poles/equator; vertex sets within angular bands |
| [`neighbors`](../src/neighbors.rs) | Closest-neighbor analysis helpers |
| [`error`](../src/error.rs) | [`SphereError`](../src/error.rs) |

[`SphereLattice`](../src/lattice.rs) is the primary facade. Typical Rust usage:

```rust
use fibonacci_sphere::{DistributionMethod, PerlinNoiseConfig, SphereLattice};

let lattice = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 200, 1.0)?;
let graph = lattice.surface_graph(); // build once
let path = graph.shortest_path(0, 42)?;

let mut rng = rand::rngs::StdRng::seed_from_u64(1);
let terrain = lattice.generate_terrain(PerlinNoiseConfig::default(), &mut rng);
let polygons = lattice.terrain_area_polygons(&terrain);
let terrain_mesh = lattice.combined_terrain_mesh(&terrain, Default::default());
let coastline = lattice.coastline_segment_positions(&terrain);
```

### Wireframe and routing

Surface connectivity uses **spherical Delaunay triangulation** (always on):

- [`wireframe_edges()`](../src/lattice.rs) — Delaunay edges for rendering
- [`surface_graph()`](../src/lattice.rs) — geodesic-weighted graph for pathfinding
- Voronoi cells, terrain polygons, and filtered routing build on the same mesh

### Runtime dependencies

| Crate | Used for | Feature |
|-------|----------|---------|
| `noise` | Perlin terrain elevation | `terrain` |
| `rand` | Terrain seeding and assigners | `terrain` |
| `thiserror` | Error types | always |

Default features include `terrain`. For a slim points-only build:

```bash
cargo build -p fibonacci_sphere --no-default-features
```

That omits the `terrain` module, Perlin APIs, and terrain-filtered routing.

**Polar ice caps:** `PerlinNoiseConfig::north_polar_ice_distance` and `south_polar_ice_distance` (radians) bound the maximum angular reach. Ice is grown by least-cost Dijkstra flood fill from each pole across the Delaunay `SurfaceGraph`. Resistance parameters (`polar_ice_mountain_resistance`, `polar_ice_land_resistance`, `polar_ice_water_resistance`, `polar_ice_deep_water_resistance`) set per-terrain step costs; `polar_ice_latitude_cost` adds uniform cost per geodesic edge. Lower mountain resistance yields spidery caps along high ground; higher latitude cost yields rounder caps. Within the flooded region, temperate terrain maps to `Ice` / `IceMountain`.

## Bevy visualizer (`sphere_lattice_visualizer`)

Location: [`examples/sphere_lattice_visualizer/`](../examples/sphere_lattice_visualizer/)

Separate binary crate for interactive method comparison. Owns Bevy rendering; consumes `SphereLattice`, topology, terrain, and [`render`](../src/render/) batch mesh builders (combined terrain meshes by type, gizmo wireframe).

```bash
cargo run -p sphere_lattice_visualizer --release
```

At large point counts, Voronoi terrain fill uses **`build_combined_terrain_mesh`** (four meshes by terrain type) instead of one entity per cell.

## Godot extension (`fibonacci_sphere_gd`)

Location: [`extensions/fibonacci_sphere_gd/`](../extensions/fibonacci_sphere_gd/)

`cdylib` wrapping [`SphereLattice`](../src/lattice.rs) for GDScript. Exposes batch render helpers and caches derived terrain meshes between regenerations. See [`docs/godot.md`](godot.md).

```bash
cargo build -p fibonacci_sphere_gd --release
```

## Godot demo (`godot/`)

GDScript + scenes only. The demo calls **`generate_with_terrain`**, **`get_terrain_mesh_data`**, **`get_coastline_segments`**, **`populate_point_multimesh`**, and **`build_ribbon_line_mesh`** so heavy mesh work stays in Rust. See [`docs/godot.md`](godot.md).

## Data flow (terrain + routing)

```text
SphereLattice::generate
        │
        ├─► surface_graph() ──► Dijkstra (optionally terrain-filtered)
        │
        └─► generate_terrain(PerlinNoiseConfig)
                 │
                 ├─► terrain_area_polygons(&terrain)
                 │        └── Voronoi boundaries + border kinds (coastline, etc.)
                 │
                 └─► render (batch consumers)
                          ├── combined_terrain_mesh / get_terrain_mesh_data
                          ├── coastline_segment_positions / get_coastline_segments
                          └── build_line_ribbon_mesh / build_ribbon_line_mesh
```

## Tests

```bash
cargo test -p fibonacci_sphere
cargo test --workspace
```

Integration tests: [`tests/integration.rs`](../tests/integration.rs).
