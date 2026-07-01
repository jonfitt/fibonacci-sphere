# fibonacci_sphere — project guide for Python developers

This document explains how this Rust project is organized, how to build and test it, and how common Rust concepts map to things you may know from Python.

**What the project does:** `fibonacci_sphere` is a library that generates evenly distributed points on a sphere (Fibonacci lattices and related methods), builds spherical Delaunay/Voronoi topology, assigns Perlin terrain, provides shared render mesh builders, and supports surface pathfinding. A separate Bevy application and a Godot 4 demo visualize the results.

See also [`architecture.md`](architecture.md) for workspace layout.

---

## Python vs Rust — quick mental model

| Python | Rust (this project) |
|--------|------------------------|
| `pyproject.toml` / `setup.py` | `Cargo.toml` |
| Package on PyPI | **Crate** on crates.io (here: internal Artifactory mirror) |
| `import mypackage` | `use fibonacci_sphere::...` |
| `mypackage/__init__.py` | `src/lib.rs` (library root) |
| `if __name__ == "__main__"` script | `src/main.rs` or `examples/.../src/main.rs` (binary crate) |
| `pytest tests/` | `cargo test` (unit tests in `src/`, integration tests in `tests/`) |
| `pip install -e .` | `cargo build` (no separate “editable install”; path deps rebuild automatically) |
| Optional extras (`pip install pkg[dev]`) | **Features** in `Cargo.toml` (e.g. `--no-default-features` to omit `terrain`) |
| Virtualenv | Cargo builds into `target/`; no venv needed |
| `requirements.lock` | `Cargo.lock` (committed for apps; often committed for libs too) |

---

## Repository layout

```
fibonacci/                          # workspace root (folder name ≠ crate name)
├── Cargo.toml                      # workspace + fibonacci_sphere library package
├── VERSION                         # single source of truth for release version (synced into manifests)
├── Cargo.lock                      # pinned dependency versions
├── .cargo/config.toml              # Cargo settings (here: internal crate registry)
├── src/                            # fibonacci_sphere library source
│   ├── lib.rs                      # library entry point (see below)
│   ├── lattice.rs                  # SphereLattice API
│   ├── methods/                    # distribution algorithms
│   ├── topology/                   # Delaunay, Voronoi, pathfinding
│   ├── terrain/                    # Perlin terrain, area polygons, borders
│   ├── render/                     # Combined terrain meshes, ribbons, coastline
│   ├── geography.rs                # pole/equator distances
│   └── ...
├── tests/                          # integration tests (external to the library)
│   ├── integration.rs
│   └── common/mod.rs               # shared helpers for integration tests
├── examples/
│   └── sphere_lattice_visualizer/  # Bevy visualizer (separate binary crate)
├── extensions/
│   └── fibonacci_sphere_gd/        # Godot 4 GDExtension (cdylib)
├── godot/                          # Godot 4 demo project (GDScript only)
│   ├── project.godot
│   ├── fibonacci_sphere.gdextension
│   └── demo/                       # main.tscn, main.gd
└── docs/                           # architecture, Godot API, this guide
```

There are **two packages (crates)** in one **workspace**:

1. **`fibonacci_sphere`** — the library (root `Cargo.toml` + `src/`)
2. **`sphere_lattice_visualizer`** — Bevy debug app (`examples/sphere_lattice_visualizer/`)
3. **`fibonacci_sphere_gd`** — Godot 4 extension (`extensions/fibonacci_sphere_gd/`)

This is like having a Python monorepo with `packages/fibonacci_sphere/` as the library and `apps/sphere_lattice_visualizer/` as a small app that depends on it — except Rust uses Cargo workspaces instead of a single `pyproject.toml` with multiple packages (unless you use something like uv/poetry workspaces).

---

## Cargo.toml files

### Root `Cargo.toml` (workspace + library)

```toml
[workspace]
members = [".", "examples/sphere_lattice_visualizer", "extensions/fibonacci_sphere_gd"]
resolver = "2"
```

- **`[workspace]`** — groups multiple crates so one `cargo build` / `cargo test` at the root can see all of them.
- **`members`** — paths to each crate. `"."` is the library at the repo root; `"examples/sphere_lattice_visualizer"` is the visualizer app.

#### Why doesn't `[package]` say which workspace member it belongs to?

This confuses many people at first: one file lists **two** workspace members, but the `[package]` block clearly belongs to **`fibonacci_sphere`** and its local `src/` — not to `examples/sphere_lattice_visualizer/src/`. There is no field like `member = "."` on `[package]`. Why not?

**Because each `Cargo.toml` only describes the crate in its own directory.**

Cargo does not treat the root file as “config for the whole repo.” It treats it as **the manifest for whatever folder that file sits in**, plus optional workspace-wide settings at the top.

```
fibonacci/Cargo.toml                          ← manifest for "." (the root crate)
fibonacci/src/lib.rs                          ← source for that crate

fibonacci/examples/sphere_lattice_visualizer/Cargo.toml   ← separate manifest
fibonacci/examples/sphere_lattice_visualizer/src/main.rs  ← source for that crate
```

When Cargo reads `fibonacci/Cargo.toml`:

| Section | Scope |
|---------|--------|
| `[workspace]` | **Whole workspace** — lists all member paths, shared lockfile, etc. |
| `[package]`, `[features]`, `[dependencies]`, … | **Only the crate at `.`** — i.e. this directory’s `src/` |

The `"."` in `members` literally means *“this directory is also a crate.”* So the `[package]` in the same file naturally pairs with `src/lib.rs` next to it. Cargo never looks at `examples/sphere_lattice_visualizer/src/` when interpreting the root `[package]` — that path has its **own** `Cargo.toml` with its **own** `[package] name = "sphere_lattice_visualizer"`.

You do not disambiguate because **there is nothing to disambiguate**: two folders, two manifests, two crates. The root file is doing double duty:

1. **Workspace root** — `[workspace] members = [...]`
2. **Package root for `"."`** — `[package] name = "fibonacci_sphere"`

**Python analogy:** imagine a monorepo where the repo root has both a workspace config and its own installable library:

```
repo/
  pyproject.toml          # [tool.uv.workspace] members = [".", "apps/viz"]
                          # AND [project] name = "fibonacci_sphere"  ← only for "."
  src/...
  apps/viz/
    pyproject.toml        # [project] name = "sphere_lattice_visualizer"  ← separate
    src/...
```

Each `pyproject.toml` describes **one** package in **one** directory. The root `pyproject.toml` does not define the viz app’s metadata; `apps/viz/pyproject.toml` does. Cargo works the same way — except the workspace table and the root package table often live in the **same file** because the library crate lives at the repo root.

**How Cargo finds source for each member:**

| Member path | Manifest | Default library entry | Default binary entry |
|-------------|----------|-------------------------|----------------------|
| `"."` | `./Cargo.toml` | `src/lib.rs` | `src/main.rs` (if present) |
| `"examples/sphere_lattice_visualizer"` | `examples/sphere_lattice_visualizer/Cargo.toml` | `.../src/lib.rs` (if present) | `.../src/main.rs` |

Convention, not configuration: if `src/lib.rs` exists → library crate; if `src/main.rs` exists → binary. The visualizer only has `main.rs`, so it builds an executable. The root only has `lib.rs`, so it builds a library.

When you run `cargo build -p fibonacci_sphere`, Cargo loads **root** `Cargo.toml` and compiles **root** `src/`. When you run `cargo build -p sphere_lattice_visualizer`, Cargo loads **that subfolder’s** `Cargo.toml` and compiles **that** `src/`. The `-p` flag selects **which manifest** to use, not which section inside one big manifest.

```toml
[package]
name = "fibonacci_sphere"
version = "0.1.2"
edition = "2024"
description = "..."
license = "GPL-3.0-or-later"
```

- **`[package]`** — metadata for the **library crate** at this path (because `"."` is both workspace member and package root).
- **`name`** — what you `use` in code: `fibonacci_sphere::SphereLattice`.
- **`license`** — SPDX identifier; full GPL text is in root `LICENSE.md`.

Release version lives in root **`VERSION`** (not duplicated by hand in each crate). Root `[workspace.package] version` and
`docs/description.md` are synced from it via `scripts/linux/sync-version.sh` or `scripts/windows/sync-version.cmd`.
Workspace members set `version.workspace = true`.

```toml
[features]
default = ["terrain"]
terrain = []
```

- **Features** are compile-time switches (similar to optional dependency groups or `#ifdef`).
- **`default = ["terrain"]`** — Perlin terrain and Voronoi areas enabled by default.
- **`terrain`** — pulls in `noise` and `rand`.

```toml
[dependencies]
noise = "0.9"
rand = "0.8"
thiserror = "2"
```

- **`[dependencies]`** — crates this library needs at runtime (like `install_requires` in setuptools, but resolved at compile time). `noise` and `rand` support Perlin terrain in the core API.
- Versions come from `Cargo.lock` after the first build.

### `examples/sphere_lattice_visualizer/Cargo.toml` (binary crate)

```toml
[package]
name = "sphere_lattice_visualizer"
publish = false
```

- A **separate package** with its own name. You run it with `cargo run -p sphere_lattice_visualizer` (`-p` = package).
- **`publish = false`** — don’t publish this to crates.io; it’s an internal example app.

```toml
[dependencies]
bevy = { version = "0.16", features = ["bevy_ui"] }
dejavu = "2.37.0"
fibonacci_sphere = { path = "../.." }
```

- **`path = "../.."`** — depends on the local library (like `pip install -e ../fibonacci_sphere`).

There is **no `[[bin]]` section** here: Cargo’s default is “if `src/main.rs` exists, build a binary named after the package” (`sphere_lattice_visualizer`).

### `.cargo/config.toml`

Project-local Cargo configuration — not a package manifest. Here it redirects `crates.io` to an internal Artifactory mirror (company policy). Equivalent in Python might be a pip index URL in `pip.conf`.

---

## `lib.rs` — is it special?

**Yes.** For a library crate, **`src/lib.rs` is the mandatory root module**. Cargo discovers it automatically; you do not declare it anywhere.

- **`src/lib.rs`** → library crate (importable as `fibonacci_sphere`)
- **`src/main.rs`** → binary crate (executable)

You can have **both** in one package (library + a small CLI), but this project keeps them separate: library at root, visualizer in `examples/sphere_lattice_visualizer/src/main.rs`.

What `lib.rs` does here:

1. **Module tree** — `pub mod methods;`, `mod lattice;`, etc. (like organizing subpackages).
2. **Public API** — `pub use ...` re-exports (see next section).
3. **Crate docs** — the `//!` comments at the top become docs on docs.rs / `cargo doc`.
4. **Attributes** — `#![deny(missing_docs)]` applies to the whole crate (all public items must have doc comments).

Subfolders use **`mod.rs`** or **`name.rs`**:

- `src/methods/mod.rs` is the root of the `methods` module; it declares `mod canonical;`, `mod offset;`, etc.
- `src/lattice.rs` is the single file for module `lattice` (alternative to `lattice/mod.rs`).

---

## `use crate::...` vs `pub use point::...` (no `crate`)

Rust has three common path prefixes:

| Prefix | Meaning |
|--------|---------|
| `crate::` | “Inside **this** crate” (like an absolute import from the package root) |
| `super::` | Parent module |
| (none) / `self::` | Current module or item brought into scope |

**Inside the library**, private modules refer to each other with `crate`:

```rust
// src/methods/mod.rs
use crate::error::SphereError;      // from src/error.rs
use crate::validation::validate_lattice_params;
```

`crate` means “start from `lib.rs`” — similar to `from fibonacci_sphere.error import ...` if you were *inside* the package.

**At the crate root (`lib.rs`)**, re-exports define the **public API** — what external users can import without knowing internal file layout:

```rust
// src/lib.rs
mod point;                              // private: src/point.rs
pub use point::{SpherePoint, GOLDEN_RATIO};   // public alias at crate root
```

External code then writes:

```rust
use fibonacci_sphere::SpherePoint;      // short, stable API
```

instead of:

```rust
use fibonacci_sphere::point::SpherePoint;   // also works if point were pub mod
```

Why no `crate` in `pub use point::...`?

- In `lib.rs`, `point` is a **sibling module** declared in the same file (`mod point;`). You refer to it by name directly.
- `pub use point::SpherePoint` means: “take `SpherePoint` from submodule `point` and expose it as `fibonacci_sphere::SpherePoint`.”
- You *could* write `pub use crate::point::SpherePoint`; it’s equivalent here. Idiomatically, at the root of `lib.rs`, the `crate::` prefix is often omitted for brevity.

**Python analogy:**

```python
# Internal module
from fibonacci_sphere.point import SpherePoint   # crate-internal import

# __init__.py re-export
from .point import SpherePoint, GOLDEN_RATIO
__all__ = ["SpherePoint", "GOLDEN_RATIO"]
```

Consumers do `from fibonacci_sphere import SpherePoint` and don’t need to know about `point.py`.

---

## Tests — why two places?

Rust splits tests into **unit tests** and **integration tests** by convention and compilation model.

### 1. Unit tests — inside `src/**/*.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn something() { ... }
}
```

Often at the bottom of the same file as the code under test (e.g. `src/lattice.rs`, `src/methods/offset.rs`).

- **`#[cfg(test)]`** — this module is **only compiled when running tests** (stripped from release builds). Like putting test code behind `if TYPE_CHECKING` or a `tests/` submodule that isn’t imported in production — but enforced by the compiler.
- **`use super::*`** — import items from the parent module (the code being tested).
- Can access **`pub(crate)`** and private functions in the same module — true “white box” unit tests.

`src/test_helpers.rs` is included only for unit tests:

```rust
// lib.rs
#[cfg(test)]
mod test_helpers;
```

### 2. Integration tests — `tests/*.rs`

Files in `tests/` are separate crates that **only link against the public API** of `fibonacci_sphere` — like pytest tests that `import fibonacci_sphere` and nothing else.

```rust
// tests/integration.rs
use fibonacci_sphere::SphereLattice;
```

- They **cannot** call private functions inside `src/lattice.rs`.
- They prove the exported API works together end-to-end.

**Why not put everything in `tests/`?** Unit tests next to implementation are easier to maintain and can test private helpers. Integration tests catch “the public surface is wrong or incomplete.”

### Running tests

```bash
# All workspace tests
cargo test

# Library only
cargo test -p fibonacci_sphere

# One test by name
cargo test canonical_midpoint_golden_values
```

Doc tests (examples in `///` and `//!` comments) run with `cargo test` too — see the example in `lib.rs`.

---

## Builds — profiles, packages, and commands

Cargo has built-in **profiles** (like debug vs optimized builds):

| Profile | Command | Output | Use |
|---------|---------|--------|-----|
| **dev** (default) | `cargo build`, `cargo test` | `target/debug/` | Fast compile, debug symbols |
| **release** | `cargo build --release` | `target/release/` | Optimized, for running the visualizer smoothly |

### Common commands

```bash
# Build the library
cargo build -p fibonacci_sphere

# Build the library
cargo build -p fibonacci_sphere

# Run the Bevy visualizer (release recommended)
cargo run -p sphere_lattice_visualizer --release

# Generate API documentation (opens HTML)
cargo doc -p fibonacci_sphere --open

# Check without full link (faster lint-like pass)
cargo check -p fibonacci_sphere
```

**`-p PACKAGE`** selects which workspace member to build. Without `-p`, Cargo builds the **default member** (usually the root package).

### What gets built?

| Crate | Entry file | Artifact |
|-------|------------|----------|
| `fibonacci_sphere` | `src/lib.rs` | `target/.../libfibonacci_sphere.rlib` (library) |
| `sphere_lattice_visualizer` | `examples/sphere_lattice_visualizer/src/main.rs` | `target/.../sphere_lattice_visualizer.exe` (binary) |

There is no separate “setup.py build” step — `cargo build` compiles everything and caches in `target/`.

---

## Features (`terrain`) in practice

The default library build includes everything needed for games: points, Delaunay wireframe, Voronoi, terrain, geography, and pathfinding.

Build without terrain (points and topology only):

```bash
cargo build -p fibonacci_sphere --no-default-features
```

---

## Module map (library)

| Path | Role |
|------|------|
| `src/lib.rs` | Crate root, public re-exports |
| `src/point.rs` | `SpherePoint`, golden ratio |
| `src/lattice.rs` | `SphereLattice` — main user-facing type |
| `src/methods/` | All distribution algorithms + `MethodInfo` metadata |
| `src/topology/` | Spherical Delaunay, Voronoi, `SurfaceGraph`, pathfinding |
| `src/terrain/` | Perlin assigner, `TerrainMap`, area polygons, border kinds |
| `src/render/` | Combined terrain meshes, coastline segments, line ribbon meshes |
| `src/geography.rs` | Pole/equator angular distances and vertex queries |
| `src/neighbors.rs` | Closest-neighbor queries |
| `src/validation.rs` | Shared parameter validation |
| `src/error.rs` | `SphereError` |
| `src/topology/voronoi_mesh.rs` | Shared Voronoi cell fan triangulation |

---

## Visualizer and Godot demo (separate from the library)

### Bevy visualizer

The Bevy app is intentionally **not** part of the library:

- **`plugin.rs`** — registers Bevy systems.
- **`settings.rs`**, **`camera.rs`**, **`hud.rs`**, **`lattice.rs`**, etc. — UI and rendering.
- Depends on **`fibonacci_sphere`** for lattice data, topology, terrain, and **`render`** batch mesh builders.
- Voronoi terrain fill uses **`build_combined_terrain_mesh`** (four meshes by terrain type) instead of one Bevy entity per cell.

```bash
cargo run -p sphere_lattice_visualizer --release
```

Controls: M (method), +/- (count), [/] (radius), H (wireframe), B/C (Voronoi borders/fill), ,/. and 9/0 and ;/' (Perlin), 1/2 and 3/4 (polar ice), R (seed), drag/scroll (camera).

### Godot demo

[`godot/demo/`](../godot/demo/) is GDScript only. Build the Rust extension first (`cargo build -p fibonacci_sphere_gd --release`), then open `godot/project.godot`. Uses **M** for method cycling (same as Bevy). The demo calls **`generate_with_terrain`**, **`get_terrain_mesh_data`**, **`populate_point_multimesh`**, and **`build_ribbon_line_mesh`** so regeneration stays fast at large point counts. See [`godot.md`](godot.md).

---

## Other Rust-isms you’ll see

- **`Result<T, E>` / `.unwrap()`** — explicit error handling; `unwrap()` panics on error (fine in tests/examples, avoid in library APIs).
- **`&str`, `String`, lifetimes** — ownership; no GC. Not critical for navigating this repo at first.
- **`derive(Debug, Clone, ...)`** — auto-implemented traits (like `@dataclass` + some protocols).
- **`impl Trait for Type`** — inherent methods and trait implementations (like class methods / protocols).
- **`mod` vs `pub mod`** — private vs exported modules.
- **`Cargo.lock`** — reproducible builds; commit it for applications; libraries often commit it too in applications/workspaces.

---

## Suggested first steps

1. Read `src/lib.rs` — the whole public API is re-exported from here.
2. Read `src/lattice.rs` and `src/methods/mod.rs` — core behavior.
3. Run `cargo test -p fibonacci_sphere`.
4. Run `cargo run -p sphere_lattice_visualizer --release` and press **M** to cycle methods.
5. Read [`architecture.md`](architecture.md) — workspace layout.
6. Skim `tests/integration.rs` — examples of using the library as an external consumer would.

That mirrors a Python workflow: read `__init__.py`, core module, run pytest, run the demo script.
