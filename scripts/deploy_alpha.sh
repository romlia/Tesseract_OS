#!/usr/bin/env bash
set -e

# ==============================================================================
# Tesseract OS: Alpha Node Deployment Script (The Umbilical Bootstrap)
# Target: Wi-Fi -> Umbilical Handoff
# ==============================================================================

WIFI_IP="10.0.0.2"
UMBILICAL_IP="10.42.0.2"
ALPHA_USER="jarvis"
ALPHA_PASS="./enter@JARVIS5"
TARGET_DIR="/home/jarvis/tesseract-os"
LOCAL_WORKSPACE="$(dirname $(dirname $(realpath $0)))"

echo "========================================================"
echo "    Initiating Remote Deployment to Alpha Node"
echo "========================================================"
echo "[*] Phase 1 Target (Wi-Fi): $ALPHA_USER@$WIFI_IP"

# 1. Ensure local dependencies
if ! command -v sshpass &> /dev/null; then
    echo "[*] Installing sshpass locally to automate password injection..."
    sudo apt-get update && sudo apt-get install -y sshpass rsync
fi

export SSHPASS="$ALPHA_PASS"

# 2. Bootstrap the Umbilical Cord
echo "[1/4] Bootstrapping the Umbilical Wire..."
sshpass -e ssh -o StrictHostKeyChecking=no $ALPHA_USER@$WIFI_IP "mkdir -p /tmp/tesseract_scripts"
sshpass -e rsync -avz -e "ssh -o StrictHostKeyChecking=no" "$LOCAL_WORKSPACE/scripts/umbilical_target.sh" "$ALPHA_USER@$WIFI_IP:/tmp/tesseract_scripts/"

# Discover the ethernet interface dynamically (ignoring lo and wifi)
TARGET_IFACE=$(sshpass -e ssh -o StrictHostKeyChecking=no $ALPHA_USER@$WIFI_IP "ip -br link | grep -v 'lo\|wlan\|wifi' | awk '{print \$1}' | head -n 1")

if [ -z "$TARGET_IFACE" ]; then
    echo "[!] Could not detect a physical ethernet port on Alpha node."
    exit 1
fi

echo "[*] Discovered physical interface on Alpha: $TARGET_IFACE"
echo "[*] Executing umbilical script remotely over Wi-Fi..."

sshpass -e ssh -o StrictHostKeyChecking=no $ALPHA_USER@$WIFI_IP "echo '$ALPHA_PASS' | sudo -S bash /tmp/tesseract_scripts/umbilical_target.sh $TARGET_IFACE"

echo "[*] Waiting for Umbilical Wire ($UMBILICAL_IP) to stabilize..."
sleep 3
if ping -c 1 -W 2 $UMBILICAL_IP > /dev/null 2>&1; then
    echo "[*] Umbilical Wire is HOT. Severing Wi-Fi dependency."
else
    echo "[!] Failed to reach Alpha node over the wire. Check physical connection."
    exit 1
fi

# 3. Sync codebase over the WIRE
echo "[2/4] Syncing Tesseract_OS codebase via Umbilical Cord..."
sshpass -e ssh -o StrictHostKeyChecking=no $ALPHA_USER@$UMBILICAL_IP "mkdir -p $TARGET_DIR"

sshpass -e rsync -avz --exclude 'target' --exclude 'node_modules' --exclude '.git' \
    -e "ssh -o StrictHostKeyChecking=no" \
    "$LOCAL_WORKSPACE/" "$ALPHA_USER@$UMBILICAL_IP:$TARGET_DIR/Tesseract_OS/"

# 4. Execute remote installation over the WIRE
echo "[3/4] Executing Remote Genesis Sequence..."

sshpass -e ssh -o StrictHostKeyChecking=no $ALPHA_USER@$UMBILICAL_IP << EOF
    set -e
    echo "========================================================"
    echo "    Alpha Node Remote Execution Commenced"
    echo "========================================================"

    echo "[*] Disabling Pop!_OS Desktop Environment (gdm3)..."
    echo "$ALPHA_PASS" | sudo -S systemctl stop gdm3 || true
    echo "$ALPHA_PASS" | sudo -S systemctl set-default multi-user.target || true

    echo "[*] Navigating to Tesseract OS source..."
    cd $TARGET_DIR/Tesseract_OS

    echo "[*] Running master installation script..."
    echo "$ALPHA_PASS" | sudo -S ./install.sh

    echo "[*] Activating Tesseract Daemon..."
    echo "$ALPHA_PASS" | sudo -S systemctl start tesseract

    echo "[*] Tesseract OS deployed and active."
EOF

echo "[4/4] Deployment Complete!"
echo "========================================================"
echo "To monitor the Alpha node securely over the wire, run:"
echo "sshpass -p '$ALPHA_PASS' ssh $ALPHA_USER@$UMBILICAL_IP 'journalctl -u tesseract -f'"
