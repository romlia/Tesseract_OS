#!/usr/bin/env bash
set -e

echo "[*] Compiling Prismatic OS for bare-metal execution..."
# Compile using regular user to avoid creating root-owned files in target/
cargo build --release

echo "[*] Requesting sudo privileges to launch Tesseract OS..."
# We must preserve environment variables if needed, or explicitly set them.
# The binary needs access to DRM and input devices which sudo provides.
sudo ./target/release/prismatic-os
