#!/usr/bin/env bash
set -e

echo "========================================================"
echo "    Tesseract OS: Umbilical Target Setup (Alpha Node)"
echo "========================================================"

if [ "$EUID" -ne 0 ]; then
  echo "[!] Please run as root: sudo ./umbilical_target.sh <interface>"
  exit 1
fi

IFACE=$1

if [ -z "$IFACE" ]; then
    echo "[!] Error: No network interface provided."
    echo "Usage: sudo ./umbilical_target.sh <interface_name>"
    echo "Available interfaces:"
    ip -br link | awk '{print $1}' | grep -v "^lo$"
    exit 1
fi

echo "[*] Configuring interface $IFACE with static IP 10.42.0.2/24..."
ip addr flush dev "$IFACE"
ip addr add 10.42.0.2/24 dev "$IFACE"
ip link set "$IFACE" up

echo "[*] Setting default route to 10.42.0.1 (Beta Host)..."
# Optional: Only uncomment if Alpha lacks Wi-Fi and needs to route all internet through Beta
# ip route add default via 10.42.0.1 dev "$IFACE"

echo "[*] Configuring Remote Syslog forwarding to 10.42.0.1..."
if ! grep -q "10.42.0.1" /etc/rsyslog.conf 2>/dev/null; then
    echo "*.* @10.42.0.1:514" >> /etc/rsyslog.conf
    systemctl restart rsyslog || true
fi

echo "========================================================"
echo "    Target Configuration Complete!"
echo "========================================================"
echo "The Alpha Node is now accessible at 10.42.0.2 via SSH."
echo "Remote Syslog is streaming to 10.42.0.1."
echo "You can deploy binaries via: scp <file> jarvis@10.42.0.2:~/"
echo "You can start the debug server via: gdbserver 10.42.0.2:2159 <file>"
