# Akida NPU Driver Deployment Guide

## Overview

The Akida driver supports **two deployment modes**:

1. **Kernel Driver** (high performance): Requires one-time install, then persists across boots
2. **Userspace Driver** (no kernel module): Works immediately, no installation needed

## Quick Start

### For Development (Userspace - No Install)

```rust
use akida_driver::{select_backend, BackendSelection};

// Just works, no sudo needed
let backend = select_backend(
    BackendSelection::Userspace,
    "0000:01:00.0"  // PCIe address from lspci
)?;
```

**Requirements:**
- PCIe resources readable: `/sys/bus/pci/devices/*/resource*`
- Udev rules (optional, for non-root): See below

### For Production (Kernel Driver - One-Time Install)

```bash
# Run once with sudo (installs systemd service)
cd /path/to/toadstool
sudo ./scripts/install-akida-driver.sh

# Driver now loads automatically on every boot
# No sudo needed ever again
```

**What this does:**
- Installs `akida-setup` binary to `/opt/toadstool/bin/`
- Installs systemd service: `akida-driver.service`
- Installs udev rules: `/etc/udev/rules.d/99-akida.rules`
- Loads kernel module if available: `akida-pcie.ko`
- Persists across reboots

---

## Detailed Deployment

### 1. Userspace Driver (Zero Install)

**Advantages:**
- ✅ No root/sudo required
- ✅ Works on any system immediately
- ✅ Safe for untrusted code (sandboxable)
- ✅ Cross-kernel compatible
- ✅ Easy development/debugging

**Disadvantages:**
- ❌ Lower throughput (~500 MB/s vs 5-10 GB/s)
- ❌ Higher latency (~1 ms vs <100 µs)
- ❌ Polling-based (no interrupts)

**Setup (Optional - for non-root access):**

Create `/etc/udev/rules.d/99-akida-userspace.rules`:
```udev
# Allow non-root users to access Akida PCIe resources
SUBSYSTEM=="pci", ATTR{vendor}=="0x1e7c", ATTR{device}=="0xbca1", \
  RUN+="/bin/chmod 666 $sys$devpath/resource*", \
  RUN+="/bin/chmod 666 $sys$devpath/enable"
```

Reload udev:
```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

**Usage:**
```rust
// No installation needed!
let backend = select_backend(
    BackendSelection::Userspace,
    "0000:01:00.0"
)?;

backend.load_model(&model)?;
let output = backend.infer(&input)?;
```

---

### 2. Kernel Driver (One-Time Install)

**Advantages:**
- ✅ Maximum performance (DMA + interrupts)
- ✅ Low latency (<100 µs)
- ✅ Mature, well-tested
- ✅ Automatic on boot

**Disadvantages:**
- ❌ Requires one-time `sudo` install
- ❌ Needs kernel module (`akida-pcie.ko`)
- ❌ Kernel-version specific

**Installation:**

```bash
# Option 1: Automated installer (recommended)
sudo ./scripts/install-akida-driver.sh

# Option 2: Manual steps
sudo cargo build --release -p akida-setup
sudo ./target/release/akida-setup
```

**Systemd Service:**

The installer creates `/etc/systemd/system/akida-driver.service`:

```ini
[Unit]
Description=Akida NPU Driver Loader
After=multi-user.target

[Service]
Type=oneshot
ExecStart=/opt/toadstool/bin/akida-setup
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
```

**Management:**
```bash
# Check status
systemctl status akida-driver
journalctl -u akida-driver

# Manual start (if not booted yet)
sudo systemctl start akida-driver

# Disable (if needed)
sudo systemctl disable akida-driver
```

---

## Multi-System Deployment

### For Container/Cloud Deployment

Use **userspace driver** (no installation):

```dockerfile
FROM rust:latest

# Copy ToadStool binary
COPY target/release/toadstool /usr/local/bin/

# No driver installation needed!
# Userspace driver works immediately

CMD ["toadstool", "run", "--backend=userspace"]
```

### For Bare Metal Fleet

Use **kernel driver** with configuration management:

**Ansible:**
```yaml
- name: Install Akida driver
  become: yes
  command: /opt/toadstool/scripts/install-akida-driver.sh
  args:
    creates: /etc/systemd/system/akida-driver.service

- name: Enable Akida driver
  systemd:
    name: akida-driver
    enabled: yes
    state: started
```

**Puppet:**
```puppet
exec { 'install-akida-driver':
  command => '/opt/toadstool/scripts/install-akida-driver.sh',
  creates => '/etc/systemd/system/akida-driver.service',
  user    => 'root',
}

service { 'akida-driver':
  ensure => running,
  enable => true,
  require => Exec['install-akida-driver'],
}
```

---

## Runtime Backend Selection

ToadStool automatically selects the best available backend:

```rust
// Auto-select (tries kernel first, falls back to userspace)
let backend = select_backend(
    BackendSelection::Auto,
    device_id
)?;

match backend.backend_type() {
    BackendType::Kernel => {
        println!("Using kernel driver (high performance)");
    }
    BackendType::Userspace => {
        println!("Using userspace driver (no kernel module)");
    }
}
```

**Selection Logic:**
1. `Auto`: Try kernel first, fall back to userspace
2. `Kernel`: Require kernel driver (fail if unavailable)
3. `Userspace`: Always use userspace (even if kernel available)

---

## Verification

### Check Driver Status

```bash
# Check for Akida PCIe devices
lspci -d 1e7c:

# Check kernel driver loaded
lsmod | grep akida
ls -l /dev/akida*

# Check userspace access
ls -l /sys/bus/pci/devices/*/resource*
```

### Test with ToadStool

```bash
# Test kernel backend
cargo run --release -p akida-driver --example detect_akida

# Test userspace backend
cargo run --release -p akida-driver --example detect_akida -- --backend=userspace

# Run showcase
cd showcase/neuromorphic/01-akida-detection
./demo.sh
```

---

## Troubleshooting

### "Permission denied" on `/sys/bus/pci/devices/*/resource*`

**Solution:** Install udev rules (see Userspace Driver section above)

### "No such device /dev/akida0"

**Cause:** Kernel module not loaded

**Solutions:**
1. Install kernel driver: `sudo ./scripts/install-akida-driver.sh`
2. Use userspace driver: `BackendSelection::Userspace`

### "Kernel module not found"

**Cause:** `akida-pcie.ko` not installed

**Solutions:**
1. Contact BrainChip for kernel module source
2. Use userspace driver (recommended for now)

### Driver not loading on boot

```bash
# Check systemd service
systemctl status akida-driver
journalctl -u akida-driver

# Manually reload
sudo systemctl daemon-reload
sudo systemctl restart akida-driver
```

---

## Security Considerations

### Kernel Driver (Trusted)
- Requires root to install (one-time)
- Runs as system service
- Full hardware access
- Use for: Owner workloads, production

### Userspace Driver (Untrusted)
- No root required
- Sandboxable (cgroups + seccomp)
- Limited hardware access (PIO only)
- Use for: Multi-tenant, development, lending

---

## Uninstallation

```bash
# Stop and disable service
sudo systemctl stop akida-driver
sudo systemctl disable akida-driver

# Remove files
sudo rm /etc/systemd/system/akida-driver.service
sudo rm /etc/udev/rules.d/99-akida.rules
sudo rm -rf /opt/toadstool

# Unload module
sudo modprobe -r akida-pcie

# Reload
sudo systemctl daemon-reload
sudo udevadm control --reload-rules
```

---

## Summary

| Scenario | Driver Mode | Install Required | Command |
|----------|-------------|------------------|---------|
| **Development** | Userspace | No | Just run code |
| **Production (Owner)** | Kernel | Yes (once) | `sudo ./scripts/install-akida-driver.sh` |
| **Multi-Tenant** | Userspace | No | Sandbox + userspace |
| **Container** | Userspace | No | Just deploy binary |
| **Cloud VM** | Userspace | No | Works immediately |

**Key Point:** After kernel driver installation, **no sudo is ever needed again** - the driver loads automatically on boot via systemd.
