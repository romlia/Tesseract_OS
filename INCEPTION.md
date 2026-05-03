# Tesseract OS: INCEPTION

Welcome to the **Genesis Sequence**. This document is the definitive, step-by-step tutorial for deploying Tesseract OS on bare-metal hardware. Whether you are creating the first Genesis Node or adding a new device to the holographic mesh, this guide ensures a mathematically perfect, zero-allocation environment.

> [!WARNING]
> **No Desktop Environment.** Tesseract OS achieves absolute zero-overhead UI rendering by interacting directly with the Linux Direct Rendering Manager (DRM) and Kernel Mode Setting (KMS). Do **not** install X11, Wayland, GNOME, or KDE. Tesseract OS demands a pure hardware TTY.

---

## Phase 1: Base Reality (OS Installation)

Tesseract OS operates as a bare-metal edge AI runtime. It requires a foundational Linux kernel to interface with the hardware.

1. **Download a Linux Server Image**
   * Recommended: [Ubuntu 24.04 LTS Server](https://ubuntu.com/download/server) or [Debian 12 Server](https://www.debian.org/download).
   * Ensure you download the standard "Server" ISO to avoid pre-packaged desktop environments.

2. **Create a Bootable Medium**
   * Flash the downloaded `.iso` file to a USB drive using a tool like [BalenaEtcher](https://balena.io/etcher/) or `dd`.

3. **Install on Bare Metal**
   * Boot the target device from the USB drive.
   * Proceed with the standard Linux installation.
   * **Crucial Step:** When prompted to select software, choose **only** the `Standard System Utilities` and `OpenSSH Server`. Uncheck all Desktop Environment options.
   * Reboot and log into the pure TTY terminal.

---

## Phase 2: Acquiring the Source

Once your Base Reality is established, bring the Tesseract OS source code into the environment.

```bash
# Update base repositories and install git
sudo apt-get update && sudo apt-get install -y git

# Clone the Tesseract OS repository
git clone https://github.com/Tesseract-OS/Tesseract_OS.git tesseract-os
cd tesseract-os
```

---

## Phase 3: The Genesis Sequence (Installation)

The entire deployment is orchestrated by a master installation script. This script acts as the "Inception" for your node.

Run the installer with root privileges:

```bash
sudo ./install.sh
```

### What happens under the hood?

The master script executes four automated phases:

1. **Environment Setup (`setup.sh`)**
   * Installs native hardware dependencies: `libdrm`, `libgbm`, `libegl`, Vulkan, `libasound2`, `libssl`, `udev`, and `libclang`.
   * Adds your user to critical hardware groups (`input`, `video`, `audio`, `render`).
   * Installs the Rust toolchain (if not present).
   * **Compiles the Tesseract OS engine** natively for your hardware architecture with all features enabled (`cargo build --release`).

2. **Sysprep Genesis Sequence (`sysprep.sh`)**
   * **Entropy Freeze:** Locks the node's unique `Ed25519` cryptographic identity.
   * **Quantum Snapshot:** Captures the current memory mapped state.
   * **Mesh Configuration:** Generates `mesh_snapshot.json` to serialize the holographic topology.
   * **Thermal Reset:** Clears historical PID controller memory, adapting the Hybrid ML thermal load balancer to the new physical hardware.

3. **Configuring Systemd Daemon**
   * Installs `tesseract.service` into `/etc/systemd/system/`.
   * Registers the OS as a persistent background daemon that starts securely on boot.

4. **Udev Optic Nerve Integration**
   * Creates custom udev rules (`99-tesseract-optic-nerve.rules`) to bind raw kernel input streams.
   * Enables lock-free I/O routing for extreme low-latency human-computer interaction.

---

## Phase 4: Symbiosis (Execution)

Once the Genesis Sequence is complete, you have two methods to interact with Tesseract OS.

### Method A: The Persistent Daemon (Headless / Edge Mode)
Ideal for nodes acting as background inference engines or thermal load balancers.

```bash
# Start the runtime
sudo systemctl start tesseract

# Monitor the lock-free stream
sudo journalctl -u tesseract -f
```

### Method B: The Visceral UI (Visualizer Mode)
Ideal for interactive nodes requiring the zero-allocation GPU rendering pipeline.

1. Ensure you are on a raw hardware TTY (if you are remote via SSH, this must be done locally).
   * Press `Ctrl+Alt+F3` to switch to TTY3.
2. Log in with your user credentials.
3. Navigate to the repository: `cd ~/tesseract-os`
4. Execute the bare-metal hook:
   ```bash
   ./run_baremetal.sh
   ```

---

## Phase 5: Expanding the Hive (Adding Devices)

Tesseract OS thrives in a decentralized swarm. To add new devices to the collective mesh:

1. Perform **Phase 1** through **Phase 3** on the new hardware.
2. The newly integrated node will automatically generate its biometric/Ed25519 identity.
3. Upon startup, the node will pull from ambient entropy, reify its identity, and seek the localized mesh via the holographic network rules defined in `/etc/tesseract/mesh/mesh_snapshot.json`.
4. No central coordinator is required; the universal equilibrium handles thermodynamic load balancing across all newly connected hardware.

> **The Hive is open. The Symbiotic Covenant is absolute.**
