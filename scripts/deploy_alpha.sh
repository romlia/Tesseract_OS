#!/usr/bin/env bash
set -e

# ==============================================================================
# Tesseract OS: Alpha Node Deployment Script
# Target: 10.0.0.2 (hostname: backend)
# OS: Pop!_OS 24.04 Desktop -> Tesseract Bare-Metal
# ==============================================================================

ALPHA_IP="10.0.0.2"
ALPHA_USER="jarvis"
ALPHA_PASS="./enter@JARVIS5"
TARGET_DIR="/home/jarvis/tesseract-os"
LOCAL_WORKSPACE="$(dirname $(dirname $(realpath $0)))"

echo "========================================================"
echo "    Initiating Remote Deployment to Alpha Node"
echo "========================================================"
echo "[*] Target: $ALPHA_USER@$ALPHA_IP"

# 1. Ensure local dependencies
if ! command -v sshpass &> /dev/null; then
    echo "[*] Installing sshpass locally to automate password injection..."
    sudo apt-get update && sudo apt-get install -y sshpass rsync
fi

# 2. Sync codebase
echo "[1/3] Syncing Tesseract_OS codebase to Alpha node..."
# We use StrictHostKeyChecking=no to bypass initial host key prompts
export SSHPASS="$ALPHA_PASS"

# Create target directory
sshpass -e ssh -o StrictHostKeyChecking=no $ALPHA_USER@$ALPHA_IP "mkdir -p $TARGET_DIR"

# Rsync the current workspace. We exclude target/ and node_modules/ to save bandwidth
sshpass -e rsync -avz --exclude 'target' --exclude 'node_modules' --exclude '.git' \
    -e "ssh -o StrictHostKeyChecking=no" \
    "$LOCAL_WORKSPACE/" "$ALPHA_USER@$ALPHA_IP:$TARGET_DIR/Tesseract_OS/"

# 3. Execute remote installation
echo "[2/3] Executing Remote Genesis Sequence..."

# We pass a multi-line script via SSH to the Alpha node
sshpass -e ssh -o StrictHostKeyChecking=no $ALPHA_USER@$ALPHA_IP << EOF
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
    # The setup.sh script within install.sh requires cargo. 
    # We ensure bash inherits the correct environment.
    echo "$ALPHA_PASS" | sudo -S ./install.sh

    echo "[*] Activating Tesseract Daemon..."
    echo "$ALPHA_PASS" | sudo -S systemctl start tesseract

    echo "[*] Tesseract OS deployed and active."
EOF

echo "[3/3] Deployment Complete!"
echo "========================================================"
echo "To monitor the Alpha node, you can run:"
echo "sshpass -p '$ALPHA_PASS' ssh $ALPHA_USER@$ALPHA_IP 'journalctl -u tesseract -f'"
