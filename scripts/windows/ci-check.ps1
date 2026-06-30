# Local checks mirrored by .github/workflows/rust.yml (format, lint, build, test).
# Use when the repo and Rust toolchain both live on Windows.
$ErrorActionPreference = "Stop"

$Root = git rev-parse --show-toplevel
if (-not $Root) {
    throw "Not inside a git repository."
}
Set-Location $Root

Write-Host "==> cargo fmt --check"
cargo fmt --all -- --check

Write-Host "==> cargo clippy"
cargo clippy --all-targets -- -D warnings

Write-Host "==> cargo build"
cargo build --verbose

Write-Host "==> cargo test"
cargo test --verbose

Write-Host "All checks passed."
