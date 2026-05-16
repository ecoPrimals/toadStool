// SPDX-License-Identifier: AGPL-3.0-or-later
//! PCIe bridge keepalive — prevents D3cold on PCIe switch fabrics.
//!
//! Generates periodic config-space reads on upstream bridges and downstream
//! GPU endpoints to prevent BIOS/ACPI idle power-gating from transitioning
//! PCIe switches into D3cold. This is critical for any GPU behind a PCIe
//! switch (PLX PEX 8747 on Tesla K80, AMD Matisse switches, Broadcom
//! PEX switches in multi-GPU workstations, etc.).
//!
//! ## How it works
//!
//! 1. **Startup**: Discovers all PCI-to-PCI bridges (class `0x0604`) with
//!    GPU endpoints downstream. Pins `d3cold_allowed=0` and `power/control=on`
//!    on every bridge in the hierarchy from GPU to root complex.
//!
//! 2. **Steady state**: Every `INTERVAL` seconds, reads PCI config-space
//!    offset 0x04 (COMMAND register) on each monitored BDF. This generates
//!    a CfgRd TLP that keeps the LTSSM in L0.
//!
//! 3. **Swap guard**: During driver swaps, the keepalive switches to burst
//!    mode (10ms interval) to saturate the fabric with CfgRd traffic during
//!    the critical unbind/rebind window. Callers use [`SwapGuard`].
//!
//! ## Discovery
//!
//! Scans `/sys/bus/pci/devices` for bridge-class devices (class `0x060400`)
//! that have GPU endpoints (class `0x0300` or `0x0302`) downstream.

use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tracing::{debug, info, warn};

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(3);

const BURST_INTERVAL: Duration = Duration::from_millis(10);

/// Global swap-guard refcount. When >0, keepalive runs at burst frequency.
static SWAP_GUARD_COUNT: AtomicUsize = AtomicUsize::new(0);

/// RAII guard that switches pcie_keepalive to burst mode for the duration
/// of a driver swap. Drop the guard to return to normal cadence.
pub struct SwapGuard(());

impl SwapGuard {
    /// Enter burst mode. Returns a guard whose drop restores normal cadence.
    #[must_use]
    pub fn enter() -> Self {
        let prev = SWAP_GUARD_COUNT.fetch_add(1, Ordering::SeqCst);
        info!(active_guards = prev + 1, "PCIe keepalive: burst mode ON");
        Self(())
    }
}

impl Drop for SwapGuard {
    fn drop(&mut self) {
        let prev = SWAP_GUARD_COUNT.fetch_sub(1, Ordering::SeqCst);
        info!(active_guards = prev - 1, "PCIe keepalive: burst mode OFF");
    }
}

fn current_interval() -> Duration {
    if SWAP_GUARD_COUNT.load(Ordering::Relaxed) > 0 {
        BURST_INTERVAL
    } else {
        KEEPALIVE_INTERVAL
    }
}

const PLX_VENDOR_ID: u16 = 0x10b5;

fn read_config_u16(bdf: &str, offset: u64) -> Option<u16> {
    let path = format!("/sys/bus/pci/devices/{bdf}/config");
    let mut f = std::fs::File::open(&path).ok()?;
    f.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = [0u8; 2];
    f.read_exact(&mut buf).ok()?;
    Some(u16::from_le_bytes(buf))
}

fn read_config_u32(bdf: &str, offset: u64) -> Option<u32> {
    let path = format!("/sys/bus/pci/devices/{bdf}/config");
    let mut f = std::fs::File::open(&path).ok()?;
    f.seek(SeekFrom::Start(offset)).ok()?;
    let mut buf = [0u8; 4];
    f.read_exact(&mut buf).ok()?;
    Some(u32::from_le_bytes(buf))
}

fn discover_plx_bridges() -> Vec<String> {
    let mut bridges = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") else {
        return bridges;
    };

    for entry in entries.flatten() {
        let bdf = entry.file_name().to_string_lossy().to_string();
        let Some(vendor) = read_config_u16(&bdf, 0x00) else {
            continue;
        };
        if vendor != PLX_VENDOR_ID {
            continue;
        }
        let Some(class) = read_config_u32(&bdf, 0x08) else {
            continue;
        };
        let class_code = (class >> 8) & 0xFF_FFFF;
        if class_code == 0x0604 {
            bridges.push(bdf);
        }
    }

    bridges.sort();
    bridges
}

fn discover_downstream_gpus(bridges: &[String]) -> Vec<String> {
    let mut gpus = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") else {
        return gpus;
    };

    for entry in entries.flatten() {
        let bdf = entry.file_name().to_string_lossy().to_string();
        let Some(class) = read_config_u32(&bdf, 0x08) else {
            continue;
        };
        let class_code = (class >> 8) & 0xFF_FFFF;
        // VGA compatible (0x0300) or 3D controller (0x0302)
        if class_code != 0x0300 && class_code != 0x0302 {
            continue;
        }

        let link = format!("/sys/bus/pci/devices/{bdf}");
        let Ok(canonical) = std::fs::canonicalize(&link) else {
            continue;
        };
        let path_str = canonical.to_string_lossy();
        for bridge_bdf in bridges {
            if path_str.contains(bridge_bdf) {
                gpus.push(bdf.clone());
                break;
            }
        }
    }

    gpus
}

fn discover_gpu_bridges() -> Vec<String> {
    let mut bridges = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") else {
        return bridges;
    };

    for entry in entries.flatten() {
        let bdf = entry.file_name().to_string_lossy().to_string();
        let Some(class) = read_config_u32(&bdf, 0x08) else {
            continue;
        };
        let class_code = (class >> 8) & 0xFF_FFFF;
        if class_code == 0x0604 {
            bridges.push(bdf);
        }
    }

    bridges.sort();
    bridges
}

fn pin_hierarchy_for_gpus(gpu_bdfs: &[String]) -> usize {
    let mut total_pinned = 0usize;
    for bdf in gpu_bdfs {
        let pinned = toadstool_ember::sysfs::pin_bridge_hierarchy(bdf);
        toadstool_ember::sysfs::pin_power(bdf);
        if pinned > 0 {
            info!(bdf, bridges_pinned = pinned, "pinned GPU bridge hierarchy at startup");
        }
        total_pinned += pinned;
    }
    total_pinned
}

pub(crate) async fn run() {
    let all_bridges = discover_gpu_bridges();
    let plx_bridges = discover_plx_bridges();

    let bridges = if plx_bridges.is_empty() { &all_bridges } else { &plx_bridges };

    if bridges.is_empty() {
        info!("No PCIe bridges with GPU endpoints found — keepalive disabled");
        return;
    }

    let downstream = discover_downstream_gpus(bridges);

    let pinned = pin_hierarchy_for_gpus(&downstream);

    info!(
        bridge_count = bridges.len(),
        gpu_count = downstream.len(),
        hierarchies_pinned = pinned,
        bridges = ?bridges,
        "PCIe bridge keepalive started (hierarchies pinned)"
    );

    let all_targets: Vec<String> = bridges
        .iter()
        .chain(downstream.iter())
        .cloned()
        .collect();

    let mut consecutive_failures = 0u32;

    loop {
        let mut alive = 0usize;
        let mut dead = 0usize;

        for bdf in &all_targets {
            match read_config_u16(bdf, 0x04) {
                Some(cmd) if cmd != 0xFFFF => {
                    alive += 1;
                }
                _ => {
                    dead += 1;
                    debug!(bdf, "keepalive: config read failed or returned 0xFFFF");
                }
            }
        }

        if dead > 0 {
            consecutive_failures += 1;
            if consecutive_failures % 20 == 1 {
                warn!(
                    alive,
                    dead,
                    consecutive_failures,
                    "PCIe keepalive: some devices unreachable (D3cold?)"
                );
            }
        } else {
            if consecutive_failures > 0 {
                info!(alive, "PCIe keepalive: all devices recovered");
            }
            consecutive_failures = 0;
        }

        tokio::time::sleep(current_interval()).await;
    }
}
