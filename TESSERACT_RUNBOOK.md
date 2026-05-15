# Tesseract OS: Master Deployment Runbook

This runbook documents the exact chronological sequence to trigger the full Genesis deployment of Tesseract OS from start to finish. Follow these steps to ensure a clean, bare-metal installation.

## Pre-requisites
- **Beta Node (Host)**: Your primary development machine (e.g., Pop!_OS laptop).
- **Alpha Node (Target)**: The destination machine. Must be accessible over Wi-Fi (`10.0.0.2`) initially.
- **Physical Ethernet Cable**: Connecting Beta and Alpha nodes directly.

---

## Phase 1: Local Development & Preparation (Beta Node)

1. **Setup Development Environment**
   Run the setup script to install dependencies and the Rust toolchain.
   ```bash
   sudo ./setup.sh
   ```
   *Expected Output: `[ OK ] Tesseract OS compiled successfully.`*

2. **Verify Codebase Compatibility**
   Run the smoke tests to ensure all Rust feature flags compile correctly.
   ```bash
   ./smoke_test.sh
   ```

3. **(Optional) Local Bare-Metal Testing**
   If you wish to test the KMS/DRM rendering locally, switch to a pure hardware TTY (`Ctrl+Alt+F3`) and run:
   ```bash
   ./run_baremetal.sh
   ```

---

## Phase 2: Kernel Synthesis (Beta Node)

1. **Build Custom Linux Kernel & Initramfs**
   This step fetches the Linux source, compiles the `bzImage` with your custom DRM/Vulkan configuration, and packages the root filesystem via Docker.
   ```bash
   ./scripts/build_custom_kernel.sh
   ```
   *Note: This will take significant time. Verify `tesseract-kernel-bzImage` and `tesseract-initramfs.cpio.gz` are generated in the root directory.*

---

## Phase 3: The Umbilical Network Setup

1. **Configure Beta Node as Umbilical Host**
   Identify your physical ethernet interface (e.g., `enp3s0`) and run the host script.
   ```bash
   sudo ./scripts/umbilical_host.sh enp3s0
   ```
   *This establishes `10.42.0.1` and enables IP masquerading so the Alpha node can use your internet connection if necessary.*

---

## Phase 4: Remote Deployment & Genesis (Alpha Node)

1. **Deploy Tesseract OS via the Umbilical Cord**
   This orchestrator script will contact the Alpha node over Wi-Fi (`10.0.0.2`), configure its ethernet port as `10.42.0.2`, sever the Wi-Fi connection, and push the entire codebase over the physical wire. Finally, it executes the Genesis installation sequence.
   ```bash
   ./scripts/deploy_alpha.sh
   ```
   *Expected Output:*
   - `[ OK ] Umbilical Wire is HOT. Severing Wi-Fi dependency.`
   - `[ OK ] SSH access verified.`
   - `[ OK ] Systemd daemon configured and enabled.`
   - `[ OK ] Tesseract OS Daemon is active.`

2. **Monitor the Target**
   Once deployed, the OS runs headlessly or takes over the DRM display on the Alpha node. You can securely monitor the `journalctl` logs remotely over the wire:
   ```bash
   sshpass -p './enter@JARVIS5' ssh jarvis@10.42.0.2 'journalctl -u tesseract -f'
   ```

---

## Troubleshooting

- **"Failed to add user to hardware groups"**: Ensure you are running `setup.sh` via `sudo ./setup.sh`, not after `sudo su -`.
- **"Failed to reach Alpha node over the wire"**: Double check the physical ethernet cable connection and ensure the host interface name passed to `umbilical_host.sh` is correct.
- **"Sysprep verification failed"**: Check if the disk is full or if `/etc/tesseract/` permissions were manually altered.
