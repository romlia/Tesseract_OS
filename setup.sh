#!/usr/bin/env bash
set -e

echo "========================================================"
echo "    Tesseract OS: Bare-Metal TTY Setup"
echo "========================================================"

if [ "$EUID" -ne 0 ]; then
  echo "[!] Please run as root: sudo ./setup.sh"
  exit 1
fi

echo "[*] Installing native hardware dependencies (Debian/Ubuntu)..."
apt-get update
apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libdrm-dev \
    libgbm-dev \
    libegl-dev \
    vulkan-tools \
    libvulkan-dev \
    libasound2-dev \
    libssl-dev \
    libudev-dev \
    libclang-dev \
    udev

echo "[*] Adding user $SUDO_USER to input, video, and audio groups..."
usermod -aG input $SUDO_USER || true
usermod -aG video $SUDO_USER || true
usermod -aG audio $SUDO_USER || true
usermod -aG render $SUDO_USER || true

if id -nG "$SUDO_USER" | grep -qw "input" && id -nG "$SUDO_USER" | grep -qw "video"; then
    echo "[ OK ] User $SUDO_USER successfully added to hardware groups."
else
    echo "[ FAIL ] Failed to add $SUDO_USER to hardware groups."
    exit 1
fi

echo "[*] Checking for Rust toolchain..."
if ! sudo -u $SUDO_USER command -v cargo &> /dev/null; then
    echo "[*] Installing Rust toolchain for $SUDO_USER..."
    sudo -u $SUDO_USER curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sudo -u $SUDO_USER sh -s -- -y
    # Source for the current session to continue building
    source /home/$SUDO_USER/.cargo/env
fi

if ! sudo -u $SUDO_USER bash -c 'source $HOME/.cargo/env && command -v cargo' &> /dev/null; then
    echo "[ FAIL ] Rust toolchain installation failed."
    exit 1
else
    echo "[ OK ] Rust toolchain is available."
fi

echo "[*] Building Tesseract OS for Release..."
# Navigate to the repo directory just in case
cd "$(dirname "$0")"
sudo -u $SUDO_USER bash -c 'source $HOME/.cargo/env && cargo build --release --all-features'

if [ -f "./target/release/prismatic-os" ]; then
    echo "[ OK ] Tesseract OS compiled successfully."
else
    echo "[ FAIL ] Tesseract OS compilation failed. Binary not found."
    exit 1
fi

echo ""
echo "========================================================"
echo "    Setup Complete!"
echo "========================================================"
echo "[!] To achieve perfect Zero-Allocation rendering, Tesseract"
echo "    must run outside of a Desktop Environment."
echo ""
echo "Instructions for First Run:"
echo "  1. Switch to a pure hardware TTY by pressing: Ctrl+Alt+F3"
echo "  2. Log in with your credentials"
echo "  3. Navigate to this directory"
echo "  4. Execute: ./run_baremetal.sh"
echo "========================================================"
