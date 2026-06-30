# Godot Integration

The `fibonacci_sphere` library ships as a Godot 4 GDExtension. Coordinates are **Y-up, right-handed**
and match Godot 4's default 3D frame without conversion.

The extension depends on the core library. Spherical Delaunay wireframe, Voronoi terrain polygons,
and pathfinding are all part of the default build.

## Build the extension

From the repository root:

```bash
cargo build -p fibonacci_sphere_gd
# or optimized:
cargo build -p fibonacci_sphere_gd --release
```

The shared library is written to `target/debug/` (or `target/release/`):

| Platform | Debug artifact |
|----------|----------------|
| Windows  | `target/debug/fibonacci_sphere_gd.dll` |
| Linux    | `target/debug/libfibonacci_sphere_gd.so` |
| macOS    | `target/debug/libfibonacci_sphere_gd.dylib` |

Use [`scripts/linux/`](../scripts/linux/) on Linux/WSL or [`scripts/windows/`](../scripts/windows/) on
Windows so `cargo` matches the platform where you run Godot (`.so` on Linux, `.dll` on Windows).

## Open the demo project

1. Install Godot 4.1 or later.
2. Open `godot/project.godot` in the Godot editor.
3. Ensure the extension built successfully (Godot loads `godot/fibonacci_sphere.gdextension`).
4. Run the main scene (`demo/main.tscn`).

### Demo controls

Core lattice controls (same as Bevy visualizer):

| Key | Action |
|-----|--------|
| M | Cycle distribution method (0..5) |
| + / - | Increase / decrease point count by 10 |
| [ / ] | Decrease / increase radius by 0.1 |
| H | Toggle spherical Delaunay wireframe |
| , / . | Perlin mountain threshold ±0.05 |
| 9 / 0 | Deep water threshold ±0.05 |
| ; / ' | Perlin spacing factor ±0.1 |
| 1 / 2 | North polar ice distance ±0.05 rad |
| 3 / 4 | South polar ice distance ±0.05 rad |
| 5 / 6 | Polar ice mountain resistance ±0.1 |
| 7 / 8 | Polar ice water resistance ±0.1 |
| Z / X | Polar ice latitude cost ±0.25 |
| R | Regenerate terrain with a new seed |
| Esc | Clear route selection |
| LMB click (3D view) | Select route start/end on lattice vertices |
| Drag (LMB) | Orbit camera |
| Scroll | Zoom camera |

The HUD lists method descriptions, parameters, and terrain-type checkboxes for filtered routing.
Terrain polygons and coastline use ribbon quads lifted slightly above the sphere surface. Lattice
points render via a **`MultiMeshInstance3D`** (one draw call, not thousands of scene nodes). RGB axis
lines mark +X (red), +Y (green), and +Z (blue).

On first open, Godot creates `godot/.godot/extension_list.cfg` automatically.

## GDExtension types

| Rust / Godot class | Role |
|--------------------|------|
| `FibonacciSphere` | Lattice generation, wireframe, terrain, routing, geography |
| `FibonacciTerrainType` | Constants: `LAND`, `WATER`, `DEEP_WATER`, `MOUNTAIN`, `ICE`, `ICE_MOUNTAIN` |
| `FibonacciAreaBorderKind` | Constants: `SAME_TYPE`, `COASTLINE`, `SHALLOW_DEEP_WATER`, `LAND_MOUNTAIN` |
| `FibonacciTerrainArea` | One Voronoi cell polygon with border metadata |

### GDScript example

```gdscript
var generator := FibonacciSphere.new()

# Generate lattice + Perlin terrain in one call (recommended for demos/games).
var positions: PackedVector3Array = generator.generate_with_terrain(
    0, 100, 1.0,   # method, point count, radius
    1, 0.55, 0.5, 0.2,  # seed, mountain threshold, deep water threshold, spacing
    0.25, 0.25,    # north/south polar ice distance (radians; 0 disables)
    0.25, 1.0, 2.5, 5.0, 2.0  # mountain/land/water/deep resistances, latitude cost
)

# Batch terrain mesh (vertices, colors, normals, indices) — built once in Rust.
var terrain_data: Array = generator.get_terrain_mesh_data()
var coastline: PackedVector3Array = generator.get_coastline_segments()

# Points via MultiMesh (fast regeneration at large N).
var multimesh := MultiMesh.new()
multimesh.transform_format = MultiMesh.TRANSFORM_3D
multimesh.use_colors = true
multimesh.mesh = preload("res://point_mesh.tres")
generator.populate_point_multimesh(multimesh, 0.004, Color(1.0, 0.85, 0.2))

var start := generator.find_nearest_vertex_index(Vector3(0.2, 0.9, 0.3))
var goal := generator.find_nearest_vertex_index(Vector3(-0.7, 0.1, 0.6))
var allowed := PackedInt32Array([FibonacciTerrainType.LAND, FibonacciTerrainType.MOUNTAIN])
var path: PackedVector3Array = generator.shortest_surface_path_positions_with_allowed_terrain(
    start, goal, allowed
)
```

For step-by-step generation (lattice first, terrain later):

```gdscript
generator.generate(0, 100, 1.0)
generator.generate_terrain(1, 0.55, 0.5, 0.2, 0.25, 0.25, 0.25, 1.0, 2.5, 5.0, 2.0)
```

### One-shot (no cached state)

```gdscript
var positions := FibonacciSphere.generate_positions(2, 64, 1.5)
```

## API reference

### Lattice generation

| Method | Returns | Description |
|--------|---------|-------------|
| `generate(method, n, radius)` | `PackedVector3Array` | Generate lattice, cache graph, return positions |
| `generate_with_terrain(..., north_ice, south_ice, mountain_resist, land_resist, water_resist, deep_resist, latitude_cost)` | `PackedVector3Array` | Generate lattice and Perlin terrain in one call |
| `get_positions()` | `PackedVector3Array` | Positions from last successful `generate` |
| `get_wireframe_segments()` | `PackedVector3Array` | Delaunay line pairs (`[a, b, c, d, ...]`) |
| `get_point_count()` | `int` | Point count (0 if not generated) |
| `get_radius()` | `float` | Cached radius |
| `get_method_index()` | `int` | Cached method index (`-1` if not generated) |
| `clear()` | `void` | Drop cached lattice, graph, and terrain |
| `generate_positions(method, n, radius)` | `PackedVector3Array` | Static helper; does not retain state |
| `get_method_count()` | `int` | Number of methods (6) |
| `get_method_description(method)` | `String` | Multi-line HUD text for a method index |

### Terrain

| Method | Returns | Description |
|--------|---------|-------------|
| `generate_terrain(seed, mountain_threshold, deep_water_threshold, spacing_factor, north_polar_ice_distance, south_polar_ice_distance, polar_ice_mountain_resistance, polar_ice_land_resistance, polar_ice_water_resistance, polar_ice_deep_water_resistance, polar_ice_latitude_cost)` | `bool` | Perlin terrain; pass `seed < 0` for random seed |
| `has_terrain()` | `bool` | Whether terrain was generated |
| `get_terrain_type_at(vertex_index)` | `int` | Terrain type index, or `-1` |
| `get_terrain_area_polygons()` | `Array[FibonacciTerrainArea]` | Voronoi polygons (cached after terrain generation) |
| `get_terrain_mesh_data()` | `Array` | Combined terrain mesh: `[vertices, colors, normals, indices]` |
| `get_coastline_segments()` | `PackedVector3Array` | Deduplicated coastline segment pairs |
| `get_voronoi_fan_apex_position(site_index)` | `Vector3` | Fan apex for one cell (pole or site) |
| `build_voronoi_cell_fan_mesh(fan_apex, boundary, radius)` | `Array` | `[vertices, indices]` fan mesh for one cell |
| `classify_border_between_terrain_types(left, right)` | `int` | `FibonacciAreaBorderKind` index |
| `is_coastline_between_terrain_types(left, right)` | `bool` | Sea-level crossing between types |

`FibonacciTerrainArea` methods: `get_site_index()`, `get_terrain_type()`, `get_boundary()`,
`get_boundary_neighbors()`, `get_edge_border_kinds()`, `is_coastline_edge(edge_index)`.

For full-sphere terrain rendering, prefer **`get_terrain_mesh_data()`** over per-cell `build_voronoi_cell_fan_mesh()` loops.

### Rendering helpers

| Method | Returns | Description |
|--------|---------|-------------|
| `build_ribbon_line_mesh(segments, width, lift)` | `Array` | Ribbon mesh: `[vertices, indices]` from segment pairs |
| `populate_point_multimesh(multimesh, lift, default_color)` | `bool` | Fill a `MultiMesh` with lifted lattice points |
| `update_point_multimesh_highlights(multimesh, from, to, default, selected)` | `bool` | Set route highlight colors on a populated `MultiMesh` |

Wireframe, coastline, and route overlays in the demo call **`FibonacciSphere.build_ribbon_line_mesh()`**
(static) so ribbon expansion runs in Rust, not GDScript.

Terrain type indices match `FibonacciTerrainType`: Land=0, Water=1, DeepWater=2, Mountain=3, Ice=4, IceMountain=5.

**Polar ice caps:** `north_polar_ice_distance` and `south_polar_ice_distance` set the maximum angular
reach (radians). Ice is grown by least-cost flood fill from each pole across the Delaunay mesh.
Resistance parameters (`polar_ice_mountain_resistance`, `polar_ice_land_resistance`,
`polar_ice_water_resistance`, `polar_ice_deep_water_resistance`) control how easily ice crosses each
temperate terrain type; lower mountain resistance yields spidery caps along high ground.
`polar_ice_latitude_cost` adds uniform cost per geodesic edge and pushes caps toward rounder boundaries
when increased.

### Geography

| Method | Returns | Description |
|--------|---------|-------------|
| `angular_distance_to_north_pole(vertex_index)` | `float` | Radians to north pole (`-1.0` on error) |
| `angular_distance_to_south_pole(vertex_index)` | `float` | Radians to south pole |
| `angular_distance_to_equator(vertex_index)` | `float` | Radians to equator |
| `vertices_within_north_polar_distance(max_angle)` | `PackedInt32Array` | Vertices within angular band of north pole |
| `vertices_within_south_polar_distance(max_angle)` | `PackedInt32Array` | Vertices within angular band of south pole |
| `vertices_within_equatorial_distance(max_angle)` | `PackedInt32Array` | Vertices within angular band of equator |

### Pathfinding

| Method | Returns | Description |
|--------|---------|-------------|
| `find_nearest_vertex_index(position)` | `int` | Closest lattice vertex (`-1` on error) |
| `shortest_surface_path_indices(from, to)` | `PackedInt32Array` | Vertex indices along shortest path |
| `shortest_surface_path_positions(from, to)` | `PackedVector3Array` | World positions along shortest path |
| `shortest_surface_path_length(from, to)` | `float` | Total geodesic length (`-1.0` on error) |
| `shortest_surface_path_*_with_allowed_terrain(from, to, allowed)` | same as above | Restrict routing to terrain types; empty `allowed` = all types |

Invalid arguments log a Godot error and return empty arrays or sentinel values (`-1` / `-1.0`).

## Surface pathfinding

Paths follow the **spherical Delaunay wireframe** — the same edge set as `get_wireframe_segments()`.
Edge weights are **geodesic arc length** on the sphere.

Terrain-filtered variants require `generate_terrain()` first. Pass an empty `PackedInt32Array` for
`allowed_terrain_types` to allow every type.

### Typical workflow

1. Call **`generate_with_terrain(...)`** (or `generate` + `generate_terrain`) once.
2. Build visuals from cached batch APIs: `get_terrain_mesh_data()`, `get_coastline_segments()`, `populate_point_multimesh(...)`.
3. Map click positions to vertices with `find_nearest_vertex_index`.
4. Query `shortest_surface_path_positions_with_allowed_terrain(from, to, allowed)`.
5. Draw overlays with `build_ribbon_line_mesh(segments, width, lift)`.

For custom per-cell logic, iterate `get_terrain_area_polygons()` or call `build_voronoi_cell_fan_mesh()` per cell.

```gdscript
var gen := FibonacciSphere.new()
gen.generate_with_terrain(0, 200, 1.0, 1, 0.55, 0.5, 0.2, 0.25, 0.25, 0.25, 1.0, 2.5, 5.0, 2.0)

var terrain_mesh: Array = gen.get_terrain_mesh_data()
var wireframe: PackedVector3Array = gen.get_wireframe_segments()
var wireframe_mesh: Array = FibonacciSphere.build_ribbon_line_mesh(
    wireframe, 0.0025, 0.004
)

var start := gen.find_nearest_vertex_index(Vector3(0.2, 0.9, 0.3))
var goal := gen.find_nearest_vertex_index(Vector3(-0.7, 0.1, 0.6))
var path: PackedVector3Array = gen.shortest_surface_path_positions(start, goal)
var length: float = gen.shortest_surface_path_length(start, goal)
```

### Cached render data

`FibonacciSphere` builds the surface graph inside `generate()` and reuses it for every `shortest_surface_path_*` call.

After terrain generation, the extension also caches **terrain polygons**, a **combined terrain mesh**,
and **coastline segments** until the lattice or terrain changes (`generate`, `generate_terrain`, or
`clear()`). This avoids rebuilding Voronoi data on every visual update.

In Rust, prefer `lattice.surface_graph()` and [`render`](../src/render/mod.rs) batch builders over
per-cell FFI loops — see [`docs/architecture.md`](architecture.md).

### Errors

| Situation | Rust | Godot |
|-----------|------|-------|
| Vertex index out of range | `SphereError::InvalidVertexIndex` | error logged, empty result / `-1` |
| No connecting route on mesh | `SphereError::NoSurfacePath` | error logged, empty result / `-1.0` |
| Terrain routing without terrain | `SphereError::TerrainNotGenerated` | error logged, empty result |
| No lattice generated yet | — | error logged, empty result / `-1` |

## Coastline and borders

`AreaBorderKind::Coastline` marks edges where one side is above sea level (land or mountain) and the
other is below (water or deep water).

- **`get_coastline_segments()`** — deduplicated segment pairs ready for `build_ribbon_line_mesh()`.
- Per-polygon: `is_coastline_edge` on `FibonacciTerrainArea`, or filter `edge_border_kinds`.

## Distribution method indices

Each method has detailed purpose, trade-offs, and references in [`MethodInfo`](../src/methods/info.rs).

| Index | Rust variant | Optimizes for | When to use |
|-------|--------------|---------------|-------------|
| 0 | `CanonicalMidpoint` | Packing distance | **Default** |
| 1 | `Canonical` | Baseline | Classic Fibonacci; north pole at index 0 |
| 2 | `OffsetPacking` | Min neighbor distance | Tightest worst-case gaps (Roberts 2018) |
| 3 | `OffsetPackingWithPoles` | Min neighbor + poles | Offset packing with ±Y samples |
| 4 | `OffsetAverageNeighbor` | Average neighbor spacing | Uniform local spacing |
| 5 | `LatitudeLongitude` | Equal-area rings | Lat–long baseline |

## Project layout

```text
extensions/fibonacci_sphere_gd/   # Rust GDExtension crate (cdylib)
godot/
  project.godot
  fibonacci_sphere.gdextension    # library paths → ../target/...
  demo/                           # GDScript demo (main.gd, main.tscn)
```

## Debug visualization (development)

For interactive method comparison outside Godot, use the Bevy lattice visualizer:

```bash
cargo run -p sphere_lattice_visualizer --release
```

Press **M** to cycle methods. The Bevy app and Godot demo share the same hotkeys.

## Flat buffer (Rust-only)

[`SphereLattice::positions_flat()`](../src/lattice.rs) returns `[x0, y0, z0, x1, y1, z1, ...]` as
`Vec<f32>`. The GDExtension converts that layout to `PackedVector3Array` for Godot.
