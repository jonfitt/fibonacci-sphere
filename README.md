# fibonacci_sphere

Rust library for generating evenly distributed sample points on a sphere, with surface topology
(Delaunay wireframe, Voronoi cells, pathfinding), Perlin terrain, and Godot 4 integration.

Coordinates are **Y-up, right-handed**, matching Godot 4's default 3D frame.

## Workspace

| Crate / path | Role |
|--------------|------|
| [`fibonacci_sphere`](./) (root) | Core library — points, topology, terrain, geography |
| [`examples/sphere_lattice_visualizer`](./examples/sphere_lattice_visualizer/) | Bevy desktop app for comparing distribution methods |
| [`extensions/fibonacci_sphere_gd`](./extensions/fibonacci_sphere_gd/) | Godot 4 GDExtension (`cdylib`) |
| [`godot/`](./godot/) | Godot 4 demo project |
| [`docs/`](./docs/) | Architecture, Godot API, project guide |

```text
fibonacci/
├── src/                          # Core library
├── tests/                        # Integration tests
├── examples/sphere_lattice_visualizer/
├── extensions/fibonacci_sphere_gd/
├── godot/
└── docs/
    ├── architecture.md           # Workspace layout and dependencies
    ├── godot.md                  # GDExtension API
    └── description.md            # Project guide (Python devs)
```

See [`docs/architecture.md`](./docs/architecture.md) for how the core library, visualizer, and Godot extension relate.

## Core library

### Entry points

```rust
use fibonacci_sphere::{DistributionMethod, PerlinNoiseConfig, SphereLattice};

let lattice = SphereLattice::generate(DistributionMethod::CanonicalMidpoint, 100, 1.0)?;
let points = lattice.points();
let flat = lattice.positions_flat(); // [x0,y0,z0, x1,y1,z1, ...]

let mut rng = rand::rngs::StdRng::seed_from_u64(42);
let terrain = lattice.generate_terrain(PerlinNoiseConfig::default(), &mut rng);
let areas = lattice.terrain_area_polygons(&terrain);
let mesh = lattice.combined_terrain_mesh(&terrain, Default::default());
```

[`SphereLattice`](./src/lattice.rs) is the main handle: generated [`SpherePoint`](./src/point.rs)s,
method, radius, wireframe, routing graph, terrain, and geography queries.

### Modules

| Module | Responsibility |
|--------|----------------|
| [`methods`](./src/methods/) | Six distribution algorithms, ε lookup tables, [`MethodInfo`](./src/methods/info.rs) |
| [`point`](./src/point.rs) | `SpherePoint`, golden-ratio constant, spherical ↔ Cartesian |
| [`topology`](./src/topology/) | Spherical Delaunay, Voronoi cells, [`SurfaceGraph`](./src/topology/pathfinding.rs), pathfinding |
| [`terrain`](./src/terrain/) | Perlin and other assigners, area polygons, border kinds (coastline, etc.) |
| [`render`](./src/render/) | Combined terrain meshes, coastline segments, line ribbon meshes |
| [`geography`](./src/geography.rs) | Pole/equator angular distances, vertices within angular bands |
| [`neighbors`](./src/neighbors.rs) | Closest-neighbor queries and distance binning |
| [`error`](./src/error.rs) | [`SphereError`](./src/error.rs) |

### Distribution methods

[`DistributionMethod`](./src/methods/mod.rs):

| Variant | Typical use |
|---------|-------------|
| `CanonicalMidpoint` | **Default** — fast golden-angle spiral |
| `Canonical` | Baseline Fibonacci lattice (north pole at index 0) |
| `OffsetPacking` | Optimized minimum neighbor distance (Roberts 2018) |
| `OffsetPackingWithPoles` | Offset packing with explicit ±Y pole samples |
| `OffsetAverageNeighbor` | More uniform local neighbor spacing |
| `LatitudeLongitude` | Lat–long grid baseline |

Each method exposes [`MethodInfo`](./src/methods/info.rs) via `method.info()` or `method.format_description()`.

### Surface topology and pathfinding

Wireframe edges come from **spherical Delaunay triangulation** (stereographic projection + planar
Delaunay). The same graph powers routing and Voronoi terrain areas.

```rust
let edges = lattice.wireframe_edges();
let graph = lattice.surface_graph();
let path = graph.shortest_path(from_index, to_index)?;

// Terrain-filtered routing (requires generated terrain)
let path = graph.shortest_path_with_allowed_terrain(
    from_index, to_index, terrain.as_slice(), &[TerrainType::Land, TerrainType::Mountain],
)?;
```

- Edge weights are **geodesic arc length** on the sphere.
- Prefer `surface_graph()` in hot loops; `SphereLattice::shortest_surface_path` rebuilds the graph each call.

### Terrain

[`TerrainType`](./src/terrain/types.rs): `Land`, `Water`, `DeepWater`, `Mountain`, `Ice`, `IceMountain`.

[`PerlinNoiseConfig`](./src/terrain/assign/perlin.rs) drives elevation bands and optional polar ice
caps grown by flood fill from each pole (`north_polar_ice_distance`, `south_polar_ice_distance` in
radians, plus resistance and `polar_ice_latitude_cost`). Within a flooded cap, temperate terrain
becomes `Ice` / `IceMountain`. Voronoi cells become [`TerrainAreaPolygon`](./src/terrain/polygons.rs)
with [`AreaBorderKind`](./src/terrain/borders.rs) per edge (`Coastline` = sea-level crossing). For
rendering, [`render`](./src/render/) provides [`build_combined_terrain_mesh`](./src/render/terrain_mesh.rs),
[`coastline_segment_positions`](./src/render/terrain_mesh.rs), and [`build_line_ribbon_mesh`](./src/render/ribbon.rs).

### Features

| Feature | Effect |
|---------|--------|
| `default` | Enables `terrain` — points, topology, Perlin terrain, geography, pathfinding |
| `terrain` | Perlin terrain, Voronoi areas, filtered routing (`noise`, `rand` deps) |

Build points-only without terrain deps:

```bash
cargo build -p fibonacci_sphere --no-default-features
```

## Bevy visualizer

Interactive comparison of distribution methods with Delaunay wireframe, Voronoi terrain fill
(combined meshes by terrain type), coastline borders, and Perlin controls.

```bash
cargo run -p sphere_lattice_visualizer --release
```

| Key | Action |
|-----|--------|
| M | Cycle distribution method |
| + / - | Point count ±10 |
| [ / ] | Radius ±0.1 |
| H | Toggle Delaunay wireframe |
| B | Toggle Voronoi area borders |
| C | Toggle Voronoi terrain fill |
| , / . | Perlin mountain threshold ±0.05 |
| 9 / 0 | Deep water threshold ±0.05 |
| ; / ' | Perlin spacing factor ±0.1 |
| 1 / 2 | North polar ice distance ±0.05 rad |
| 3 / 4 | South polar ice distance ±0.05 rad |
| R | New terrain seed |
| Drag / scroll | Orbit / zoom |

Source: [`examples/sphere_lattice_visualizer/src/`](./examples/sphere_lattice_visualizer/src/).

## Godot integration

Build the extension from the repo root:

```bash
cargo build -p fibonacci_sphere_gd --release
```

Open [`godot/project.godot`](./godot/project.godot) in Godot 4.1+, then run [`godot/demo/main.tscn`](./godot/demo/main.tscn).

The demo adds terrain polygons, coastline ribbons, click-to-route with terrain-type checkboxes, and
Perlin hotkeys. Regeneration uses batch Rust APIs (`generate_with_terrain`, `get_terrain_mesh_data`,
`MultiMesh` points). Method cycling uses **M** (same as Bevy). See [`docs/godot.md`](./docs/godot.md)
for the full API.

## Tests

```bash
cargo test -p fibonacci_sphere
cargo test --workspace
```

Integration tests: [`tests/integration.rs`](./tests/integration.rs).

## Line endings

All text files use **CRLF** on every platform. See [`.gitattributes`](./.gitattributes) and
[`.editorconfig`](./.editorconfig). After cloning:

```bash
git config core.autocrlf false
git add --renormalize .
```

## CI and git hooks

GitHub Actions (`.github/workflows/rust.yml`) runs on every push to `main` and every pull request
targeting `main`: format check, Clippy, build, and test. The required check name is **`Rust / build`**.

Local scripts are split by platform — see [`scripts/README.md`](./scripts/README.md). Use the set
that matches where your **files** and **`cargo`** live:

| Environment | CI check | Git hooks | Branch protection |
|-------------|----------|-----------|-------------------|
| Linux / WSL with Linux `cargo` | `./scripts/linux/ci-check.sh` | `./scripts/linux/setup-git-hooks.sh` | `./scripts/linux/setup-branch-protection.sh` |
| Windows with Windows `cargo` | `scripts\windows\ci-check.cmd` | `scripts\windows\setup-git-hooks.cmd` | `scripts\windows\setup-branch-protection.cmd` |

Branch protection (once, after `gh auth login`):

```bash
./scripts/linux/setup-branch-protection.sh
```

```cmd
scripts\windows\setup-branch-protection.cmd
```

## License

See [LICENSE.md](./LICENSE.md).
