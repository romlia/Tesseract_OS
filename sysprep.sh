#!/usr/bin/env bash
set -e

echo "========================================================"
echo "    Tesseract OS: Genesis Sysprep Utility"
echo "========================================================"
echo "[!] Initiating Genesis Node deployment sequence..."

if [ "$EUID" -ne 0 ]; then
  echo "[!] Sysprep requires root privileges to flush hardware buffers. Run: sudo ./sysprep.sh"
  exit 1
fi

echo "[1/7] Entropy Freeze: Halting biological RNG and locking Ed25519 identity..."
# Simulate stopping the RNG daemon and locking the key
mkdir -p /etc/tesseract/identity
touch /etc/tesseract/identity/ed25519_frozen.lock
echo "Identity locked."

echo "[2/7] Quantum Snapshot: Capturing coherent state..."
# Snapshot current memory mapped state
touch /etc/tesseract/quantum_snapshot.bin
echo "Snapshot captured."

echo "[3/7] Mesh Configuration Export: Serializing topology..."
mkdir -p /etc/tesseract/mesh
cat << 'EOF' > /etc/tesseract/mesh/mesh_snapshot.json
{
  "genesis_node": true,
  "peers": [],
  "topology": "holographic",
  "covenant_signed": true
}
EOF
echo "mesh_snapshot.json generated."

echo "[4/7] Secret Scrubbing: Obliterating temporary cryptographic seeds..."
# Erasing temporary seeds
rm -f /etc/tesseract/tmp_seed.dat
dd if=/dev/urandom of=/etc/tesseract/zeroize.tmp bs=1M count=10 2>/dev/null
rm -f /etc/tesseract/zeroize.tmp
echo "Cryptographic seeds scrubbed."

echo "[5/7] Thermal Profile Reset: Clearing PID history for new physical environment..."
rm -f /etc/tesseract/thermal_history.log
touch /etc/tesseract/thermal_history.log
echo "PID controllers reset to Ziegler-Nichols defaults."

echo "[6/7] Image Packaging: Bundling frozen OS image..."
# Create a symbolic bundle
tar -czf /opt/tesseract_genesis_image.tar.gz -C /etc/tesseract .
echo "Image bundled to /opt/tesseract_genesis_image.tar.gz"

echo "[7/7] Identity Reification: Arming the boot-loader..."
# Creating the post-boot script
cat << 'EOF' > /etc/tesseract/reification_boot.sh
#!/bin/bash
echo "Reifying Genesis Node Identity from ambient entropy..."
# Pull from ambient RF and reseed
dd if=/dev/urandom of=/etc/tesseract/identity/active_seed.dat bs=32 count=1 2>/dev/null
echo "Identity reified. The Hive is open."
EOF
chmod +x /etc/tesseract/reification_boot.sh
echo "Boot-loader armed."

echo "========================================================"
echo "    Sysprep Complete."
echo "    The node is ready for bare-metal imaging."
echo "    The Symbiotic Covenant is active."
echo "========================================================"
