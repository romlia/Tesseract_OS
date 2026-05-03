#!/usr/bin/env bash
set -e

echo "========================================================"
echo "    Tesseract OS: Umbilical Host Setup (Beta Node)"
echo "========================================================"

if [ "$EUID" -ne 0 ]; then
  echo "[!] Please run as root: sudo ./umbilical_host.sh <interface>"
  exit 1
fi

IFACE=$1

if [ -z "$IFACE" ]; then
    echo "[!] Error: No network interface provided."
    echo "Usage: sudo ./umbilical_host.sh <interface_name>"
    echo "Available interfaces:"
    ip -br link | awk '{print $1}' | grep -v "^lo$"
    exit 1
fi

echo "[*] Configuring interface $IFACE with static IP 10.42.0.1/24..."
ip addr flush dev "$IFACE"
ip addr add 10.42.0.1/24 dev "$IFACE"
ip link set "$IFACE" up

echo "[*] Enabling IP Forwarding (NAT/Masquerade)..."
sysctl -w net.ipv4.ip_forward=1 > /dev/null

# Set up iptables for NAT so the Alpha node can use the Beta node's internet connection if needed
# We assume the default route interface is the one providing internet
DEFAULT_IFACE=$(ip route | grep default | awk '{print $5}')
if [ -n "$DEFAULT_IFACE" ]; then
    echo "[*] Bridging $IFACE to internet interface $DEFAULT_IFACE..."
    iptables -t nat -A POSTROUTING -o "$DEFAULT_IFACE" -j MASQUERADE
    iptables -A FORWARD -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
    iptables -A FORWARD -i "$IFACE" -o "$DEFAULT_IFACE" -j ACCEPT
else
    echo "[!] Warning: No default route found. NAT bridging skipped."
fi

echo "========================================================"
echo "    Host Configuration Complete!"
echo "========================================================"
echo "The Dev Host (Beta) is now listening on 10.42.0.1"
echo "Waiting for Alpha node to connect..."
