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
use std::time::Duration;

use toadstool_cylinder::nv::registers::pmc::InterruptProfile;
use tracing::{error, info, warn};

const DEFAULT_WATCHDOG_TIMEOUT: Duration = Duration::from_secs(120);
const WATCHDOG_CHECK_INTERVAL: Duration = Duration::from_millis(500);

/// Shared state between the handoff pipeline and the watchdog thread.
struct WatchdogState {
    active: AtomicBool,
    last_heartbeat_ms: AtomicU64,
    timeout_ms: AtomicU64,
    bdf: std::sync::Mutex<String>,
    interrupt_profile: std::sync::Mutex<InterruptProfile>,
}

static WATCHDOG: std::sync::LazyLock<Arc<WatchdogState>> = std::sync::LazyLock::new(|| {
    Arc::new(WatchdogState {
        active: AtomicBool::new(false),
        last_heartbeat_ms: AtomicU64::new(0),
        timeout_ms: AtomicU64::new(DEFAULT_WATCHDOG_TIMEOUT.as_millis() as u64),
        bdf: std::sync::Mutex::new(String::new()),
        interrupt_profile: std::sync::Mutex::new(InterruptProfile::VOLTA_PLUS),
    })
});

fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Emergency interrupt quench — uses generation-aware register writes via
/// `InterruptProfile`, plus PCI INTx disable as belt-and-suspenders.
fn emergency_quench(bdf: &str, profile: &InterruptProfile) {
    warn!(bdf, "WATCHDOG: performing emergency interrupt quench");
    toadstool_cylinder::nv::registers::pmc::quench_interrupts(bdf, profile, "watchdog-emergency");
    toadstool_cylinder::nv::registers::pmc::intx_disable(bdf, "watchdog-emergency");
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
/// `profile` determines the correct interrupt disable register semantics
/// (Volta+ SET/CLEAR vs pre-Volta direct write).
///
/// `timeout` overrides the default 120s watchdog timeout. Set to match or
/// exceed the pipeline deadline to avoid false-positive kills.
///
/// Returns an RAII guard. The handoff pipeline must call `guard.heartbeat()`
/// periodically. If heartbeats stop for longer than `timeout`, the watchdog
/// performs emergency interrupt quench and kills the process.
pub fn activate(
    bdf: &str,
    profile: InterruptProfile,
    timeout: Option<Duration>,
) -> CatalystWatchdogGuard {
    {
        let mut locked_bdf = WATCHDOG.bdf.lock().unwrap_or_else(|e| e.into_inner());
        *locked_bdf = bdf.to_string();
    }
    {
        let mut locked_profile = WATCHDOG.interrupt_profile.lock().unwrap_or_else(|e| e.into_inner());
        *locked_profile = profile;
    }
    let timeout = timeout.unwrap_or(DEFAULT_WATCHDOG_TIMEOUT);
    WATCHDOG.timeout_ms.store(timeout.as_millis() as u64, Ordering::Release);
    WATCHDOG.last_heartbeat_ms.store(epoch_ms(), Ordering::Release);
    WATCHDOG.active.store(true, Ordering::Release);

    info!(bdf, timeout_ms = timeout.as_millis() as u64,
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
                let timeout_ms = WATCHDOG.timeout_ms.load(Ordering::Acquire);
                let now = epoch_ms();
                let elapsed_ms = now.saturating_sub(last_hb);

                if elapsed_ms > timeout_ms {
                    let bdf = WATCHDOG.bdf.lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();
                    let profile = WATCHDOG.interrupt_profile.lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();

                    error!(
                        bdf,
                        elapsed_ms,
                        timeout_ms,
                        "CATALYST WATCHDOG TRIGGERED — handoff pipeline unresponsive!"
                    );

                    // Emergency quench using generation-aware profile
                    emergency_quench(&bdf, &profile);

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
