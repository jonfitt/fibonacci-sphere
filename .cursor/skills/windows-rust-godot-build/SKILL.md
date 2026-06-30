---
name: windows-rust-godot-build
description: >-
  Build and test the fibonacci_sphere core library and fibonacci_sphere_gd Godot
  extension as Windows artifacts from WSL Cursor agents or PowerShell/CMD. Use when
  cargo fails in WSL with "linker cc not found", when building or testing the main
  library, running cargo test on fibonacci_sphere, building the GDExtension DLL, or
  when the user mentions Windows cargo.exe, core crate, or Godot extension builds.
---

# Windows Rust / Godot build (WSL + Windows)

This repo is developed on **Windows** with **WSL** shells in Cursor. The Godot
GDExtension targets **Windows** (`fibonacci_sphere_gd.dll`). Crates.io access goes
through the internal mirror in `.cargo/config.toml` — use normal `cargo`; do not
switch to public registries.

## Choose a shell

| Environment | Use when |
|-------------|----------|
| **WSL** (Cursor agent default) | Linux shell but Windows Rust toolchain is installed |
| **PowerShell / CMD** | Native Windows terminal, or WSL lacks `cc` |

### WSL: prefer Windows `cargo.exe`

WSL `cargo` often fails with:

```text
error: linker `cc` not found
```

**Do not** treat that as a code error. Build with **Windows** `cargo.exe` and a
**Windows** manifest path (`C:\...`). `/mnt/c/...` paths do **not** work as
`--manifest-path` for `cargo.exe`.

Optional fix for native WSL builds (not required if using `cargo.exe`):

```bash
sudo apt-get install -y build-essential
```

## Commands (WSL agent)

From the repo root, run helper scripts (recommended):

```bash
.cursor/skills/windows-rust-godot-build/scripts/build-lib.sh
.cursor/skills/windows-rust-godot-build/scripts/build-lib.sh --release
.cursor/skills/windows-rust-godot-build/scripts/test.sh
.cursor/skills/windows-rust-godot-build/scripts/build-gdextension.sh
.cursor/skills/windows-rust-godot-build/scripts/build-gdextension.sh --release
```

Or invoke Windows cargo directly:

```bash
CARGO="/mnt/c/Users/FITT/.cargo/bin/cargo.exe"
MANIFEST="C:\\Users\\FITT\\RustroverProjects\\fibonacci\\Cargo.toml"

"$CARGO" build --manifest-path "$MANIFEST"
"$CARGO" build --release --manifest-path "$MANIFEST"
"$CARGO" test --manifest-path "$MANIFEST"
"$CARGO" build -p fibonacci_sphere_gd --manifest-path "$MANIFEST"
"$CARGO" build -p fibonacci_sphere_gd --release --manifest-path "$MANIFEST"
```

Override toolchain location if needed:

```bash
export CARGO_WIN="/mnt/c/Users/FITT/.cargo/bin/cargo.exe"
```

Resolve manifest path dynamically:

```bash
MANIFEST="$(wslpath -w "$(git rev-parse --show-toplevel)/Cargo.toml")"
```

## Commands (PowerShell / CMD)

```powershell
cd C:\Users\FITT\RustroverProjects\fibonacci
cargo build
cargo build --release
cargo test
cargo build -p fibonacci_sphere_gd
cargo build -p fibonacci_sphere_gd --release
```

## Core library outputs

Paths are relative to the repo root:

| Profile | Artifact |
|---------|----------|
| Debug | `target/debug/fibonacci_sphere.rlib` |
| Release | `target/release/fibonacci_sphere.rlib` |

## Godot extension outputs

Paths are relative to the repo root (shared `target/` on the Windows filesystem):

| Profile | DLL |
|---------|-----|
| Debug | `target/debug/fibonacci_sphere_gd.dll` |
| Release | `target/release/fibonacci_sphere_gd.dll` |

Godot loads the DLL via `godot/fibonacci_sphere.gdextension` (debug/release entries
point at `../target/...`).

After a successful GDExtension build, open the Godot project under `godot/` and run
the demo scene.

## Agent checklist

1. **Build/test from WSL?** → Use `cargo.exe` + Windows manifest path, or run `scripts/*.sh`.
2. **`cc` not found?** → Expected in WSL; switch to Windows cargo (above).
3. **Core library only?** → `build-lib.sh` or `cargo build` (root package, no `-p`).
4. **Library + tests?** → `test.sh` or `cargo test` (114 unit + 4 integration + doc tests).
5. **Godot extension only?** → `build-gdextension.sh` or `cargo build -p fibonacci_sphere_gd`.
6. **Report results** → Note which runner was used (WSL+`cargo.exe` vs PowerShell).

## Workspace crates

| Crate | Purpose |
|-------|---------|
| `fibonacci_sphere` (root) | Core library |
| `fibonacci_sphere_gd` | Godot 4 GDExtension (`cdylib`) |
| `sphere_lattice_visualizer` | Bevy example (optional) |

## Troubleshooting

| Symptom | Action |
|---------|--------|
| `linker cc not found` in WSL | Use Windows `cargo.exe` or install `build-essential` |
| `manifest path ... does not exist` with `/mnt/c/...` | Use `C:\Users\...` or `wslpath -w` |
| Godot cannot load extension | Rebuild `fibonacci_sphere_gd`; confirm DLL exists under `target/debug/` |
| Fresh WSL Rust install only | Install Windows Rust too, or install `build-essential` for pure Linux target |
