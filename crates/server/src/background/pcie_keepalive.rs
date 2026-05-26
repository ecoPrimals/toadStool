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
//!    GPU endpoints downstream. Also walks GPU ancestry to find PLX bridges
//!    that class-based scanning might miss (e.g., when config space returns
//!    `0xFFFF` during early boot). Pins `d3cold_allowed=0` and
//!    `power/control=on` on every bridge in the hierarchy from GPU to root
//!    complex. First heartbeat fires immediately (no initial delay).
//!
//! 2. **Steady state**: Uses `tokio::time::interval` to generate periodic
//!    CfgRd TLPs. Skips synthetic heartbeats when recent real PCIe traffic
//!    was observed (activity-aware backpressure via [`activity_tracker`]).
//!
//! 3. **Swap guard**: During driver swaps, the keepalive switches to burst
//!    mode (10ms interval) to saturate the fabric with CfgRd traffic during
//!    the critical unbind/rebind window. Callers use [`SwapGuard`].

use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::time::MissedTickBehavior;
use tracing::{debug, info, warn};

use toadstool_ember::plx_keepalive::{ActivityTracker, PLX_VENDOR_ID, is_pci_bdf};

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(3);

const BURST_INTERVAL: Duration = Duration::from_millis(10);

const DISCOVERY_RETRIES: usize = 3;
const DISCOVERY_RETRY_DELAY: Duration = Duration::from_secs(1);

/// Global swap-guard refcount. When >0, keepalive runs at burst frequency.
static SWAP_GUARD_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Shared activity tracker — exported so other modules (VFIO ops, config
/// reads) can call `activity_tracker().record()` to suppress redundant
/// synthetic keepalive heartbeats.
static ACTIVITY: std::sync::OnceLock<ActivityTracker> = std::sync::OnceLock::new();

/// Get the global PCIe activity tracker.
///
/// Any code performing real PCIe traffic should call
/// `activity_tracker().record()` so the keepalive loop can skip
/// redundant synthetic heartbeats.
pub fn activity_tracker() -> &'static ActivityTracker {
    ACTIVITY.get_or_init(ActivityTracker::new)
}

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

/// PCI base class + subclass for PCI-to-PCI bridge.
const PCI_CLASS_BRIDGE_PCI: u16 = 0x0604;
/// PCI base class + subclass for VGA-compatible controller.
const PCI_CLASS_VGA: u16 = 0x0300;
/// PCI base class + subclass for 3D controller (non-VGA GPU).
const PCI_CLASS_3D: u16 = 0x0302;

/// Extract `base_class:subclass` (16-bit) from the raw 32-bit PCI
/// class register at config offset 0x08.
fn pci_base_subclass(class_reg: u32) -> u16 {
    ((class_reg >> 16) & 0xFFFF) as u16
}

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
        if pci_base_subclass(class) == PCI_CLASS_BRIDGE_PCI {
            info!(bdf, vendor = %format!("0x{vendor:04x}"), "discovered PLX bridge via class scan");
            bridges.push(bdf);
        }
    }

    bridges.sort();
    bridges
}

/// Walk each GPU's sysfs ancestry to find PLX bridges that class-based
/// scanning might miss (e.g., if the bridge's config space was temporarily
/// returning 0xFFFF during early boot).
fn discover_plx_bridges_via_gpu_ancestry() -> Vec<String> {
    let mut bridges = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") else {
        return bridges;
    };

    for entry in entries.flatten() {
        let bdf = entry.file_name().to_string_lossy().to_string();

        let Some(class) = read_config_u32(&bdf, 0x08) else {
            continue;
        };
        let base_sub = pci_base_subclass(class);
        if base_sub != PCI_CLASS_VGA && base_sub != PCI_CLASS_3D {
            // 0xFFFF class can mean dead device; check driver_override
            // for vfio-pci bound GPUs whose config space is unreadable
            if class != 0xFFFF_FFFF {
                continue;
            }
            let override_path = format!("/sys/bus/pci/devices/{bdf}/driver_override");
            let Ok(drv) = std::fs::read_to_string(&override_path) else {
                continue;
            };
            if drv.trim() != "vfio-pci" {
                continue;
            }
            info!(bdf, "found vfio-pci device with dead config space — checking ancestry for PLX");
        }

        let link = format!("/sys/bus/pci/devices/{bdf}");
        let Ok(canonical) = std::fs::canonicalize(&link) else {
            continue;
        };

        let mut current = canonical.as_path().parent();
        while let Some(parent) = current {
            let Some(name) = parent.file_name().and_then(|n| n.to_str()) else {
                break;
            };
            if !is_pci_bdf(name) {
                break;
            }
            let parent_bdf = name;
            if bridges.contains(&parent_bdf.to_string()) {
                current = parent.parent();
                continue;
            }
            match read_config_u16(parent_bdf, 0x00) {
                Some(PLX_VENDOR_ID) => {
                    info!(
                        gpu_bdf = bdf.as_str(),
                        bridge_bdf = parent_bdf,
                        "discovered PLX bridge via GPU ancestry walk"
                    );
                    bridges.push(parent_bdf.to_string());
                }
                Some(0xFFFF) | None => {
                    // Parent bridge config space is dead — the whole PLX
                    // fabric is likely in D3cold. Add this bridge as a
                    // keepalive target and pin power to try to wake it.
                    warn!(
                        gpu_bdf = bdf.as_str(),
                        bridge_bdf = parent_bdf,
                        "parent bridge config space dead (0xFFFF) — \
                         adding as keepalive target and pinning power"
                    );
                    toadstool_ember::sysfs::pin_power(parent_bdf);
                    bridges.push(parent_bdf.to_string());
                }
                Some(_) => {}
            }
            current = parent.parent();
        }
    }

    bridges.sort();
    bridges.dedup();
    bridges
}

fn discover_downstream_gpus(bridges: &[String]) -> Vec<String> {
    let mut gpus = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") else {
        return gpus;
    };

    for entry in entries.flatten() {
        let bdf = entry.file_name().to_string_lossy().to_string();
        let class_opt = read_config_u32(&bdf, 0x08);
        let base_sub = class_opt.map_or(0xFFFF, pci_base_subclass);

        let is_gpu = base_sub == PCI_CLASS_VGA || base_sub == PCI_CLASS_3D;
        let is_dead = class_opt == Some(0xFFFF_FFFF) || class_opt.is_none();
        if !is_gpu && !is_dead {
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
        if pci_base_subclass(class) == PCI_CLASS_BRIDGE_PCI {
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

/// Discover all VFIO-bound GPUs that need keepalive, regardless of bridge topology.
/// Covers Titan V and other direct-attached GPUs that might not sit behind a
/// discoverable PLX switch.
fn discover_vfio_gpus() -> Vec<String> {
    let mut gpus = Vec::new();
    let driver_dir = "/sys/bus/pci/drivers/vfio-pci";
    let Ok(entries) = std::fs::read_dir(driver_dir) else {
        return gpus;
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if is_pci_bdf(&name) {
            gpus.push(name);
        }
    }

    gpus.sort();
    gpus
}

pub(crate) async fn run() {
    // Phase 1: class-based PLX discovery
    let mut plx_bridges = discover_plx_bridges();

    // Phase 2: if class scan found nothing, try GPU ancestry walk
    // (handles dead config space at early boot)
    if plx_bridges.is_empty() {
        info!("class-based PLX scan found nothing — trying GPU ancestry walk");
        plx_bridges = discover_plx_bridges_via_gpu_ancestry();
    }

    // Phase 3: if still nothing, retry with delay (bridge might be booting)
    if plx_bridges.is_empty() {
        for attempt in 1..=DISCOVERY_RETRIES {
            info!(attempt, "PLX bridge not found — retrying after delay");
            tokio::time::sleep(DISCOVERY_RETRY_DELAY).await;

            plx_bridges = discover_plx_bridges();
            if plx_bridges.is_empty() {
                plx_bridges = discover_plx_bridges_via_gpu_ancestry();
            }
            if !plx_bridges.is_empty() {
                info!(attempt, count = plx_bridges.len(), "PLX bridge found on retry");
                break;
            }
        }
    }

    let all_bridges = discover_gpu_bridges();
    let bridges = if plx_bridges.is_empty() { &all_bridges } else { &plx_bridges };

    let mut downstream = discover_downstream_gpus(bridges);

    // Ensure all VFIO-bound GPUs are included, even if not behind a
    // discovered bridge (direct CPU-to-GPU topology like Titan V).
    let vfio_gpus = discover_vfio_gpus();
    for bdf in &vfio_gpus {
        if !downstream.contains(bdf) {
            info!(bdf, "VFIO GPU not behind any bridge — adding to keepalive targets");
            downstream.push(bdf.clone());
        }
    }

    if bridges.is_empty() && downstream.is_empty() {
        info!("no PCIe bridges or GPU endpoints found — keepalive disabled");
        return;
    }

    // Pin immediately — before the first interval tick
    let pinned = pin_hierarchy_for_gpus(&downstream);

    // Exp 226: Pre-load SBR suppression for all VFIO VGA GPUs at startup.
    // When the daemon later releases a VfioAnchor (warm_handoff, catalyst_boot),
    // PCI_DEV_FLAGS_NO_BUS_RESET prevents pci_reset_bus() from firing SBR
    // and destroying warm state. The module is compiled once and cached in
    // /var/lib/toadstool/kmod-cache/ for instant reload across reboots.
    // FLR is already disabled by ExecStartPre in the systemd unit.
    let vga_gpus: Vec<String> = vfio_gpus.iter().filter(|bdf| {
        read_config_u32(bdf, 0x08)
            .map_or(false, |c| pci_base_subclass(c) == PCI_CLASS_VGA
                            || pci_base_subclass(c) == PCI_CLASS_3D)
    }).cloned().collect();
    if !vga_gpus.is_empty() {
        let all_bdfs = vga_gpus.join(",");
        match toadstool_cylinder::vfio::guarded_sysfs::suppress_bus_reset(&all_bdfs) {
            Ok(()) => info!(bdfs = %all_bdfs, count = vga_gpus.len(),
                           "startup SBR suppression: PCI_DEV_FLAGS_NO_BUS_RESET set (Exp 226)"),
            Err(e) => warn!(bdfs = %all_bdfs, error = %e,
                           "startup SBR suppression failed — warm handoff may trigger bus reset (Exp 226)"),
        }
    }

    info!(
        bridge_count = bridges.len(),
        gpu_count = downstream.len(),
        vfio_gpu_count = vfio_gpus.len(),
        hierarchies_pinned = pinned,
        bridges = ?bridges,
        downstream = ?downstream,
        "PCIe bridge keepalive started (hierarchies pinned)"
    );

    let all_targets: Vec<String> = bridges
        .iter()
        .chain(downstream.iter())
        .cloned()
        .collect();

    let mut consecutive_failures = 0u32;

    // interval.tick() fires immediately on first call — no initial delay
    let mut ticker = tokio::time::interval(current_interval());
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;

        // Reconfigure interval if swap guard changed
        let new_interval = current_interval();
        if ticker.period() != new_interval {
            ticker = tokio::time::interval(new_interval);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            ticker.tick().await; // consume the immediate first tick
        }

        // Activity-aware backpressure: skip synthetic heartbeat if real
        // PCIe traffic happened recently (within one interval)
        if activity_tracker().ms_since_last() < KEEPALIVE_INTERVAL.as_millis() as u64
            && SWAP_GUARD_COUNT.load(Ordering::Relaxed) == 0
        {
            continue;
        }

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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pci_base_subclass_bridge() {
        // PLX PEX 8747: class register 0x060400ca
        assert_eq!(pci_base_subclass(0x0604_00ca), PCI_CLASS_BRIDGE_PCI);
    }

    #[test]
    fn pci_base_subclass_3d_controller() {
        // Tesla K80 GK210: class register 0x030200a1
        assert_eq!(pci_base_subclass(0x0302_00a1), PCI_CLASS_3D);
    }

    #[test]
    fn pci_base_subclass_vga() {
        // RTX 5060: class register 0x030000a1
        assert_eq!(pci_base_subclass(0x0300_00a1), PCI_CLASS_VGA);
    }

    #[test]
    fn pci_base_subclass_dead_device() {
        assert_eq!(pci_base_subclass(0xFFFF_FFFF), 0xFFFF);
    }

    #[test]
    fn read_config_u16_nonexistent() {
        assert!(read_config_u16("9999:99:99.9", 0x00).is_none());
    }

    #[test]
    fn read_config_u32_nonexistent() {
        assert!(read_config_u32("9999:99:99.9", 0x08).is_none());
    }

    #[test]
    fn discover_plx_bridges_runs_without_panic() {
        let bridges = discover_plx_bridges();
        for bdf in &bridges {
            assert!(is_pci_bdf(bdf), "invalid BDF in PLX bridges: {bdf}");
        }
    }

    #[test]
    fn discover_gpu_bridges_runs_without_panic() {
        let bridges = discover_gpu_bridges();
        for bdf in &bridges {
            assert!(is_pci_bdf(bdf), "invalid BDF in GPU bridges: {bdf}");
        }
    }

    #[test]
    fn discover_ancestry_runs_without_panic() {
        let bridges = discover_plx_bridges_via_gpu_ancestry();
        for bdf in &bridges {
            assert!(is_pci_bdf(bdf), "invalid BDF from ancestry walk: {bdf}");
        }
    }

    #[test]
    fn activity_tracker_integration() {
        let tracker = activity_tracker();
        // Initially no activity
        assert!(tracker.ms_since_last() > 1_000_000 || tracker.ms_since_last() == u64::MAX);

        tracker.record();
        assert!(tracker.ms_since_last() < 1000);
    }

    #[test]
    fn swap_guard_refcount() {
        assert_eq!(SWAP_GUARD_COUNT.load(Ordering::Relaxed), 0);
        let guard = SwapGuard::enter();
        assert_eq!(SWAP_GUARD_COUNT.load(Ordering::Relaxed), 1);
        assert_eq!(current_interval(), BURST_INTERVAL);
        drop(guard);
        assert_eq!(SWAP_GUARD_COUNT.load(Ordering::Relaxed), 0);
        assert_eq!(current_interval(), KEEPALIVE_INTERVAL);
    }

    #[test]
    fn current_interval_normal() {
        assert_eq!(current_interval(), KEEPALIVE_INTERVAL);
    }
}
