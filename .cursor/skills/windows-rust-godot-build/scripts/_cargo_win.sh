#!/usr/bin/env bash
# Shared helper: resolve repo root and Windows cargo.exe for WSL builds.
set -euo pipefail

find_cargo_win() {
	if [[ -n "${CARGO_WIN:-}" && -x "$CARGO_WIN" ]]; then
		echo "$CARGO_WIN"
		return 0
	fi

	local user_name
	user_name="$(cmd.exe /c "echo %USERNAME%" 2>/dev/null | tr -d '\r\n' || true)"
	if [[ -n "$user_name" ]]; then
		local candidate="/mnt/c/Users/${user_name}/.cargo/bin/cargo.exe"
		if [[ -x "$candidate" ]]; then
			echo "$candidate"
			return 0
		fi
	fi

	local fallback="/mnt/c/Users/FITT/.cargo/bin/cargo.exe"
	if [[ -x "$fallback" ]]; then
		echo "$fallback"
		return 0
	fi

	echo "windows-rust-godot-build: could not find cargo.exe; set CARGO_WIN" >&2
	exit 1
}

repo_root() {
	git rev-parse --show-toplevel
}

win_manifest_path() {
	local root
	root="$(repo_root)"
	wslpath -w "${root}/Cargo.toml"
}

run_cargo_win() {
	local cargo_win manifest
	cargo_win="$(find_cargo_win)"
	manifest="$(win_manifest_path)"
	"$cargo_win" "$@" --manifest-path "$manifest"
}
