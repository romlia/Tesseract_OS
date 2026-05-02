#!/usr/bin/env bash
set -e

cd "$(dirname "$(readlink -f "$0")")"

if [ -n "$WAYLAND_DISPLAY" ] || [ -n "$DISPLAY" ]; then
    echo "[!] WARNING: You are running inside a Desktop Environment ($XDG_SESSION_TYPE)."
    echo "[!] Tesseract OS achieves zero-allocation rendering via direct KMS/DRM."
    echo "[!] For the true bare-metal experience, switch to a pure TTY (Ctrl+Alt+F3)."
    echo "[!] Sleeping for 3 seconds before attempting execution anyway..."
    sleep 3
fi

echo "[*] Compiling Prismatic OS for bare-metal execution..."
# Compile using regular user to avoid creating root-owned files in target/
cargo build --release

echo "[*] Requesting sudo privileges to launch Tesseract OS..."
# We must preserve environment variables if needed, or explicitly set them.
# The binary needs access to DRM and input devices which sudo provides.
sudo ./target/release/prismatic-os
