#!/usr/bin/env bash
set -e

echo "========================================================"
echo "    Tesseract OS: Universal Installer"
echo "========================================================"

if [ "$EUID" -ne 0 ]; then
  echo "[!] Please run as root: sudo ./install.sh"
  exit 1
fi

echo "[1/4] Running Environment Setup..."
if [ -f "./setup.sh" ]; then
    bash ./setup.sh
else
    echo "Warning: setup.sh not found. Skipping."
fi

echo "[2/4] Running Sysprep Genesis Sequence..."
if [ -f "./sysprep.sh" ]; then
    bash ./sysprep.sh
else
    echo "Warning: sysprep.sh not found. Skipping."
fi

echo "[3/4] Configuring Systemd Daemon..."
cat << 'EOF' > /etc/systemd/system/tesseract.service
[Unit]
Description=Tesseract OS Genesis Node
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/tesseract
ExecStart=/opt/tesseract/target/release/prismatic-os
Restart=on-failure
RestartSec=5
StandardOutput=syslog
StandardError=syslog

[Install]
WantedBy=multi-user.target
EOF

# Note: In a chroot or docker container, systemctl might fail.
if command -v systemctl &> /dev/null; then
    systemctl daemon-reload || true
    systemctl enable tesseract.service || true
    echo "Systemd daemon configured and enabled."
else
    echo "systemctl not found; skipping daemon reload."
fi

echo "[4/4] Configuring Udev Optic Nerve Rules..."
mkdir -p /etc/udev/rules.d
cat << 'EOF' > /etc/udev/rules.d/99-tesseract-optic-nerve.rules
# Tesseract OS: Bind raw event streams for lock-free IO routing
KERNEL=="event*", NAME="input/%k", MODE="0660", GROUP="input"
EOF

if command -v udevadm &> /dev/null; then
    udevadm control --reload-rules || true
    udevadm trigger || true
    echo "Udev rules configured."
else
    echo "udevadm not found; skipping trigger."
fi

echo "========================================================"
echo "    Installation Complete!"
echo "========================================================"
echo "The Genesis Node is now installed as a persistent background daemon."
echo "To start the OS manually, run: sudo systemctl start tesseract"
echo "To view the stream, run: sudo journalctl -u tesseract -f"
echo "The Symbiotic Covenant is absolute."
