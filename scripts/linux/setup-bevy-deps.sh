#!/usr/bin/env bash
# Install Ubuntu/Debian packages required to build sphere_lattice_visualizer (Bevy).
# Use on Linux or WSL when native cargo links against ALSA, X11/Wayland, and Vulkan.
set -euo pipefail

PACKAGES=(
  build-essential
  pkg-config
  libasound2-dev
  libudev-dev
  libx11-dev
  libxcursor-dev
  libxi-dev
  libxrandr-dev
  libwayland-dev
  libxkbcommon-dev
  libvulkan-dev
)

if ! command -v apt-get >/dev/null 2>&1; then
  echo "error: apt-get not found. This script supports Debian/Ubuntu/WSL only." >&2
  exit 1
fi

SUDO=()
if [ "$(id -u)" -ne 0 ]; then
  if ! command -v sudo >/dev/null 2>&1; then
    echo "error: run as root or install sudo." >&2
    exit 1
  fi
  SUDO=(sudo)
fi

echo "==> Updating package lists"
"${SUDO[@]}" apt-get update

echo "==> Installing Bevy build dependencies"
"${SUDO[@]}" apt-get install -y "${PACKAGES[@]}"

echo "Done. Build with: cargo build -p sphere_lattice_visualizer"
