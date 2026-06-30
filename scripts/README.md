# Project scripts

This repo supports two local development setups. Use the scripts that match where your **files** and **Rust toolchain** live.

| Setup | Scripts | Pre-commit install |
|-------|---------|-------------------|
| Files on Linux or WSL, `cargo` on Linux/WSL | `scripts/linux/*.sh` | `./scripts/linux/setup-git-hooks.sh` |
| Files on Windows, `cargo` on Windows | `scripts/windows/*.ps1` or `*.cmd` | `scripts\windows\setup-git-hooks.cmd` |

Do not mix paths (for example, WSL files with Windows `cargo.exe`).

## Line endings

This repository standardizes on **CRLF** for all text files (Windows, Linux, and WSL). Git
enforces that via [`.gitattributes`](../.gitattributes). Set locally once per clone:

```bash
git config core.autocrlf false
git add --renormalize .
```

POSIX shell entry points (`.githooks/pre-commit`, `scripts/linux/*.sh`) stay **LF** so hooks
and scripts execute on Linux/WSL. Editors should follow [`.editorconfig`](../.editorconfig).

## Linux / WSL (native cargo)

```bash
./scripts/linux/ci-check.sh              # fmt, clippy, build, test
./scripts/linux/setup-git-hooks.sh       # once per clone
./scripts/linux/setup-branch-protection.sh
```

Requires `cargo`, `rustfmt`, and `clippy` on your `PATH`. On WSL, install a Linux toolchain (`build-essential`, `rustup`).

## Windows (native cargo)

```cmd
scripts\windows\ci-check.cmd
scripts\windows\setup-git-hooks.cmd
scripts\windows\setup-branch-protection.cmd
```

PowerShell equivalents:

```powershell
.\scripts\windows\ci-check.ps1
.\scripts\windows\setup-git-hooks.ps1
.\scripts\windows\setup-branch-protection.ps1
```

## Git hooks

`.githooks/pre-commit` is committed and dispatches to the right CI script:

- **Linux / WSL / macOS** → `scripts/linux/ci-check.sh`
- **Windows (Git for Windows)** → `scripts/windows/ci-check.ps1`

Run the setup script once per clone to set `core.hooksPath`:

```bash
./scripts/linux/setup-git-hooks.sh
```

```cmd
scripts\windows\setup-git-hooks.cmd
```
