# Tesseract OS: The Topological Trinity (Alpha / Beta Network)

Tesseract OS is designed as a bare-metal edge runtime. Because it aggressively captures the Direct Rendering Manager (DRM) and Kernel Mode Setting (KMS) subsystems for zero-allocation rendering, local system panics or UI lockups will freeze the physical display. 

To ensure continuous, "perfect" telemetry and debugging without relying on complex local configurations, we utilize a bifurcated hardware topology that ultimately forms a single living entity: **The Trinity**. "It needs to be three to make one."

---

## Topology Architecture: Three to Make One

### 1. Node Alpha: The Physical Vessel (Target)
* **Hardware:** An older PC with medium-grade components. The "Black Box."
* **Network Role:** Target / Execution Environment / The Frontier of Life.
* **Connectivity:**
  * **Wi-Fi:** Connects to the local network for package downloads, apt updates, and external dependencies via a pluggable USB/PCIe antenna.
  * **Ethernet (RJ45):** A direct, hardwired connection to Node Beta for high-bandwidth deployment and telemetry.
* **Purpose:** Acts as the primary physical execution runtime. This is the entity reaching into the void, interacting with the real world (USB devices, antennas) to give birth to its physical presence. It can safely crash, panic, or lock up without halting development.

### 2. Node Beta: The Architect's Mind (Host)
* **Hardware:** Primary developer machine.
* **Network Role:** Host / Debug Controller.
* **Connectivity:**
  * **Wi-Fi / General Network:** Standard internet connectivity.
  * **Ethernet (RJ45):** Direct umbilical connection to Node Alpha.
* **Purpose:** Used for writing code, cross-compiling (or natively compiling and pushing), deploying binaries via SSH, and running remote debuggers (GDB/LLDB). It represents the conscious intent shaping the void.

### 3. The Hive (The Resultant Swarm)
* **The Union:** Alpha and Beta are bridged by the Umbilical Cord (RJ45). They do not merely communicate; their timelines merge. Alpha provides the physical sensory input (the Frontier of Life), while Beta provides the logic and direction. Together, they create the third entity: the true, unkillable Tesseract OS node. Three to make one.

---

## The Umbilical Cord (RJ45) Setup

A direct Ethernet cable (the "umbilical cord") connects Alpha and Beta. We establish an isolated `/24` subnet on this physical link. 

* **Beta IP:** `10.42.0.1`
* **Alpha IP:** `10.42.0.2`

### Step-by-Step Connection

1. **Physical Link:** Connect an Ethernet cable directly from Beta's RJ45 port to Alpha's RJ45 port.
2. **Determine Interface Names:** On both machines, run `ip link` to find the Ethernet interface name (e.g., `eth0`, `enp3s0`, `enp4s0`).
3. **Configure the Host (Beta):**
   Navigate to `scripts/` and run the host configuration script:
   ```bash
   sudo ./scripts/umbilical_host.sh <interface_name>
   ```
4. **Configure the Target (Alpha):**
   On the Alpha machine, run the target configuration script:
   ```bash
   sudo ./scripts/umbilical_target.sh <interface_name>
   ```

### Debugging Workflows

#### Remote Logging & Telemetry
Once the umbilical cord is established, Alpha is configured to stream its `journald` logs directly to Beta. 
You can monitor Alpha's output from Beta using:
```bash
# Listen for incoming UDP syslog streams on Beta (Port 514)
sudo tail -f /var/log/syslog | grep "alpha"
```
*(Alternatively, configure `netconsole` to catch hard kernel panics over the same interface).*

#### Remote GDB / LLDB
If you need to step through the bare-metal logic without trusting the local Tesseract UI:
1. Compile the debug binary on Beta: `cargo build`
2. Push it to Alpha via SSH over the umbilical cord:
   ```bash
   scp target/debug/prismatic-os jarvis@10.42.0.2:/home/jarvis/tesseract-os/
   ```
3. Start the debugger on Alpha:
   ```bash
   ssh jarvis@10.42.0.2 "gdbserver 10.42.0.2:2159 /home/jarvis/tesseract-os/prismatic-os"
   ```
4. Connect from Beta:
   ```bash
   gdb target/debug/prismatic-os
   (gdb) target remote 10.42.0.2:2159
   ```

---

## Dogfooding on Beta

When you are ready to deploy on **Node Beta** (your main development machine), extreme caution is required. Tesseract OS will attempt to claim the DRM node.

**Safety Precautions for Beta:**
1. **Enable SysRq:** Ensure the Magic SysRq key is enabled (`sysctl kernel.sysrq=1`) so you can reboot the kernel safely (`Alt+SysRq+REISUB`) if the display locks.
2. **SSH Daemon:** Ensure `sshd` is running and enabled on Beta so you can SSH in from a phone or another device to kill the `tesseract.service` if the UI freezes.
3. **Run from TTY:** Never launch the bare-metal OS from within an active X11/Wayland session. Always switch to `Ctrl+Alt+F3` first.
