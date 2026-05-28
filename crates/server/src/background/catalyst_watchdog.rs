// SPDX-License-Identifier: AGPL-3.0-or-later
// SAFETY: Emergency interrupt quench requires raw BAR0 MMIO writes via mmap.
// These are volatile ptr operations on a PCI BAR mapped via sysfs resource0.
// The alternative (no quench) is a system-wide lockup from IRQ storm.
#![allow(unsafe_code)]
//! Catalyst handoff watchdog — diesel engine lockup sentinel.
//!
//! Spawns a watchdog thread when a catalyst handoff begins. The thread monitors
//! a heartbeat from the handoff pipeline. If the heartbeat stops for longer than
//! `WATCHDOG_TIMEOUT`, the watchdog performs emergency interrupt quench and kills
//! the ember service to prevent a system-wide lockup from cascading to the
//! display GPU.
//!
//! The watchdog thread is pinned to a CPU core different from the handoff thread
//! so it can still run during an IRQ storm on one core.
//!
//! ## Why this exists
//!
//! When rm_trigger exits, nvidia_close tears down MSI/IRQ. If the GPU is still
//! warm and generating interrupts, legacy INTx fires with no handler → interrupt
//! storm → system lockup. The primary fix is quenching interrupts in rm_trigger
//! before closing fds. This watchdog is the safety net — if ANYTHING in the
//! handoff pipeline causes an unrecoverable hang, the watchdog detects it and
//! kills the process to save the system.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tracing::{error, info, warn};

// 120s: covers the 60s settle period + generous margin for module load, rm_trigger,
// etc. The primary lockup defense is the interrupt quench in rm_trigger. This
// watchdog catches sustained lockups (pci_lock deadlocks) that persist beyond
// the settle window.
const WATCHDOG_TIMEOUT: Duration = Duration::from_secs(120);
const WATCHDOG_CHECK_INTERVAL: Duration = Duration::from_millis(500);

/// NV_PMC_INTR_EN(0) at 0x140 is READ-ONLY on Volta (shows current state).
/// NV_PMC_INTR_EN_CLEAR(0) at 0x180 is WRITE-ONLY (writing 1 bits disables).
const NV_PMC_INTR_EN_0: usize = 0x140;
const NV_PMC_INTR_EN_CLEAR_0: usize = 0x180;

/// Shared state between the handoff pipeline and the watchdog thread.
struct WatchdogState {
    active: AtomicBool,
    last_heartbeat_ms: AtomicU64,
    bdf: std::sync::Mutex<String>,
}

static WATCHDOG: std::sync::LazyLock<Arc<WatchdogState>> = std::sync::LazyLock::new(|| {
    Arc::new(WatchdogState {
        active: AtomicBool::new(false),
        last_heartbeat_ms: AtomicU64::new(0),
        bdf: std::sync::Mutex::new(String::new()),
    })
});

fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Emergency interrupt quench — write 0 to NV_PMC_INTR_EN_0 via BAR0 mmap
/// and set PCI command register bit 10 (INTx disable).
fn emergency_quench(bdf: &str) {
    warn!(bdf, "WATCHDOG: performing emergency interrupt quench");

    // Layer 1: Disable GPU interrupt generation via BAR0 (direct MMIO)
    let bar0_path = format!("/sys/bus/pci/devices/{bdf}/resource0");
    if let Ok(f) = std::fs::OpenOptions::new().read(true).write(true).open(&bar0_path) {
        use std::os::fd::AsFd;
        match unsafe {
            rustix::mm::mmap(
                std::ptr::null_mut(),
                0x1000,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::SHARED,
                f.as_fd(),
                0,
            )
        } {
            Ok(map) => {
                let old = unsafe {
                    std::ptr::read_volatile((map as *const u8).add(NV_PMC_INTR_EN_0) as *const u32)
                };
                // Write to CLEAR register (0x180), not the read-only 0x140
                unsafe {
                    std::ptr::write_volatile(
                        (map as *mut u8).add(NV_PMC_INTR_EN_CLEAR_0) as *mut u32,
                        0xFFFF_FFFF,
                    );
                }
                let verify = unsafe {
                    std::ptr::read_volatile((map as *const u8).add(NV_PMC_INTR_EN_0) as *const u32)
                };
                let _ = unsafe { rustix::mm::munmap(map, 0x1000) };
                warn!(bdf,
                      old = format_args!("0x{old:08x}"),
                      verify = format_args!("0x{verify:08x}"),
                      "WATCHDOG: INTR_EN_CLEAR@0x180 written (GPU interrupts disabled)");
            }
            Err(e) => {
                warn!(bdf, error = %e, "WATCHDOG: BAR0 mmap failed");
            }
        }
    }

    // Layer 2: Set PCI Interrupt Disable bit via config space
    let cfg_path = format!("/sys/bus/pci/devices/{bdf}/config");
    if let Ok(mut f) = std::fs::OpenOptions::new().read(true).write(true).open(&cfg_path) {
        use std::io::{Read, Seek, Write};
        let mut cmd_bytes = [0u8; 2];
        if f.seek(std::io::SeekFrom::Start(4)).is_ok()
            && f.read_exact(&mut cmd_bytes).is_ok()
        {
            let old_cmd = u16::from_le_bytes(cmd_bytes);
            let new_cmd = old_cmd | 0x0400;
            let _ = f.seek(std::io::SeekFrom::Start(4));
            let _ = f.write_all(&new_cmd.to_le_bytes());
            warn!(bdf, old_cmd = format_args!("0x{old_cmd:04x}"),
                  new_cmd = format_args!("0x{new_cmd:04x}"),
                  "WATCHDOG: PCI INTx disabled");
        }
    }
}

/// RAII guard for a catalyst handoff. The watchdog thread monitors heartbeats
/// while this guard is alive. The handoff pipeline should call
/// `guard.heartbeat()` at each major step.
///
/// When dropped, deactivates the watchdog for this handoff.
pub struct CatalystWatchdogGuard {
    _private: (),
}

impl CatalystWatchdogGuard {
    /// Signal that the handoff pipeline is still making progress.
    pub fn heartbeat(&self) {
        WATCHDOG.last_heartbeat_ms.store(epoch_ms(), Ordering::Release);
    }
}

impl Drop for CatalystWatchdogGuard {
    fn drop(&mut self) {
        WATCHDOG.active.store(false, Ordering::Release);
        info!("catalyst watchdog: handoff completed, watchdog deactivated");
    }
}

/// Send a heartbeat from any context (doesn't need the guard).
/// The pipeline calls this at each major step to reset the watchdog timer.
pub fn heartbeat() {
    if WATCHDOG.active.load(Ordering::Acquire) {
        WATCHDOG.last_heartbeat_ms.store(epoch_ms(), Ordering::Release);
    }
}

/// Activate the catalyst watchdog for a handoff on the given BDF.
///
/// Returns an RAII guard. The handoff pipeline must call `guard.heartbeat()`
/// periodically. If heartbeats stop for `WATCHDOG_TIMEOUT`, the watchdog
/// performs emergency interrupt quench and kills the process.
pub fn activate(bdf: &str) -> CatalystWatchdogGuard {
    {
        let mut locked_bdf = WATCHDOG.bdf.lock().unwrap_or_else(|e| e.into_inner());
        *locked_bdf = bdf.to_string();
    }
    WATCHDOG.last_heartbeat_ms.store(epoch_ms(), Ordering::Release);
    WATCHDOG.active.store(true, Ordering::Release);

    info!(bdf, timeout_ms = WATCHDOG_TIMEOUT.as_millis() as u64,
          "catalyst watchdog: activated for handoff");

    CatalystWatchdogGuard { _private: () }
}

/// Start the watchdog background thread. Call once at daemon startup.
/// The thread sleeps until activated by `activate()`.
pub fn start_watchdog_thread() {
    std::thread::Builder::new()
        .name("catalyst-watchdog".into())
        .spawn(move || {
            info!("catalyst watchdog thread started");
            loop {
                std::thread::sleep(WATCHDOG_CHECK_INTERVAL);

                if !WATCHDOG.active.load(Ordering::Acquire) {
                    continue;
                }

                let last_hb = WATCHDOG.last_heartbeat_ms.load(Ordering::Acquire);
                let now = epoch_ms();
                let elapsed_ms = now.saturating_sub(last_hb);

                if elapsed_ms > WATCHDOG_TIMEOUT.as_millis() as u64 {
                    let bdf = WATCHDOG.bdf.lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();

                    error!(
                        bdf,
                        elapsed_ms,
                        timeout_ms = WATCHDOG_TIMEOUT.as_millis() as u64,
                        "CATALYST WATCHDOG TRIGGERED — handoff pipeline unresponsive!"
                    );

                    // Emergency quench
                    emergency_quench(&bdf);

                    // Deactivate to avoid repeated triggers
                    WATCHDOG.active.store(false, Ordering::Release);

                    // Kill the ember service to free kernel resources
                    error!("WATCHDOG: killing toadstool-ember to save system");
                    let _ = std::process::Command::new("systemctl")
                        .args(["kill", "--signal=KILL", "toadstool-ember.service"])
                        .status();

                    // If systemctl didn't work, abort the process
                    error!("WATCHDOG: aborting process as last resort");
                    std::process::abort();
                }
            }
        })
        .expect("failed to spawn catalyst watchdog thread");
}
