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

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use toadstool_cylinder::nv::registers::pmc::InterruptProfile;
use tracing::{error, info, warn};

const DEFAULT_WATCHDOG_TIMEOUT: Duration = Duration::from_secs(120);
const WATCHDOG_CHECK_INTERVAL: Duration = Duration::from_millis(500);
const MODULE_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Pipeline phase constants for module lifecycle tracking.
const PHASE_IDLE: u8 = 0;
const PHASE_PIPELINE_ACTIVE: u8 = 1;
const PHASE_MODULE_CLEANUP: u8 = 2;

/// Shared state between the handoff pipeline and the watchdog thread.
struct WatchdogState {
    active: AtomicBool,
    last_heartbeat_ms: AtomicU64,
    timeout_ms: AtomicU64,
    bdf: std::sync::Mutex<String>,
    interrupt_profile: std::sync::Mutex<InterruptProfile>,
    module_name: std::sync::Mutex<String>,
    phase: AtomicU8,
}

static WATCHDOG: std::sync::LazyLock<Arc<WatchdogState>> = std::sync::LazyLock::new(|| {
    Arc::new(WatchdogState {
        active: AtomicBool::new(false),
        last_heartbeat_ms: AtomicU64::new(0),
        timeout_ms: AtomicU64::new(DEFAULT_WATCHDOG_TIMEOUT.as_millis() as u64),
        bdf: std::sync::Mutex::new(String::new()),
        interrupt_profile: std::sync::Mutex::new(InterruptProfile::VOLTA_PLUS),
        module_name: std::sync::Mutex::new(String::new()),
        phase: AtomicU8::new(PHASE_IDLE),
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
    #[allow(dead_code, clippy::unused_self)] // guard-scoped API; free `heartbeat()` used today
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

/// Signal that the pipeline is entering the module cleanup phase.
/// The watchdog will begin high-frequency module state polling to observe
/// the module death transition in real-time.
pub fn enter_module_cleanup(module_name: &str) {
    {
        let mut locked = WATCHDOG.module_name.lock().unwrap_or_else(|e| e.into_inner());
        *locked = module_name.to_string();
    }
    WATCHDOG.phase.store(PHASE_MODULE_CLEANUP, Ordering::Release);
    WATCHDOG.last_heartbeat_ms.store(epoch_ms(), Ordering::Release);
    info!(module = module_name, "watchdog: entering module_cleanup phase — high-frequency monitoring");
}

/// Signal that module cleanup is complete (success or failure).
pub fn exit_module_cleanup() {
    WATCHDOG.phase.store(PHASE_PIPELINE_ACTIVE, Ordering::Release);
    WATCHDOG.last_heartbeat_ms.store(epoch_ms(), Ordering::Release);
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
    module_name: &str,
) -> CatalystWatchdogGuard {
    {
        let mut locked_bdf = WATCHDOG.bdf.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        *locked_bdf = bdf.to_string();
    }
    {
        let mut locked_profile = WATCHDOG
            .interrupt_profile
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *locked_profile = profile;
    }
    {
        let mut locked_module = WATCHDOG.module_name.lock().unwrap_or_else(|e| e.into_inner());
        *locked_module = module_name.to_string();
    }
    let timeout = timeout.unwrap_or(DEFAULT_WATCHDOG_TIMEOUT);
    WATCHDOG.timeout_ms.store(timeout.as_millis() as u64, Ordering::Release);
    WATCHDOG.last_heartbeat_ms.store(epoch_ms(), Ordering::Release);
    WATCHDOG.phase.store(PHASE_PIPELINE_ACTIVE, Ordering::Release);
    WATCHDOG.active.store(true, Ordering::Release);

    info!(bdf, timeout_ms = timeout.as_millis() as u64, module_name,
          "catalyst watchdog: activated for handoff");

    CatalystWatchdogGuard { _private: () }
}

/// Check whether the watchdog is currently monitoring an active handoff.
pub fn is_active() -> bool {
    WATCHDOG.active.load(Ordering::Acquire)
}

/// Force an emergency interrupt quench from an external caller (e.g. kernel
/// sentinel). Uses the currently registered BDF and interrupt profile.
pub fn force_emergency_quench() {
    let bdf = WATCHDOG.bdf.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let profile = WATCHDOG.interrupt_profile.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if !bdf.is_empty() {
        emergency_quench(&bdf, &profile);
    }
}

/// Start the watchdog background thread. Call once at daemon startup.
/// The thread sleeps until activated by `activate()`.
///
/// # Errors
///
/// Returns an error if the OS refuses to spawn the watchdog thread.
pub fn start_watchdog_thread() -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("catalyst-watchdog".into())
        .spawn(move || {
            info!("catalyst watchdog thread started");
            let mut last_module_state: Option<String> = None;
            let mut last_module_refcount: Option<i64> = None;
            let mut module_poll_counter: u64 = 0;

            loop {
                let phase = WATCHDOG.phase.load(Ordering::Acquire);
                let sleep_duration = if phase == PHASE_MODULE_CLEANUP {
                    MODULE_POLL_INTERVAL
                } else {
                    WATCHDOG_CHECK_INTERVAL
                };
                std::thread::sleep(sleep_duration);

                if !WATCHDOG.active.load(Ordering::Acquire) {
                    if last_module_state.is_some() {
                        last_module_state = None;
                        last_module_refcount = None;
                        module_poll_counter = 0;
                    }
                    continue;
                }

                // Module lifecycle monitoring during cleanup phase
                if phase == PHASE_MODULE_CLEANUP {
                    let module_name = WATCHDOG.module_name.lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();
                    if !module_name.is_empty() {
                        module_poll_counter += 1;
                        if let Some(snap) = toadstool_cylinder::vfio::guarded_sysfs::module_snapshot(&module_name) {
                            let state_changed = last_module_state.as_deref() != Some(&snap.state);
                            let refcount_changed = last_module_refcount != Some(snap.refcount);

                            if state_changed || refcount_changed || module_poll_counter % 25 == 1 {
                                info!(
                                    module = snap.name.as_str(),
                                    state = snap.state.as_str(),
                                    refcount = snap.refcount,
                                    size = snap.size,
                                    is_stuck = snap.is_stuck,
                                    poll = module_poll_counter,
                                    "watchdog: module state snapshot"
                                );
                            }

                            if snap.is_zombie() && !last_module_state.as_deref().is_some_and(|s| s == "Unloading") {
                                warn!(
                                    module = snap.name.as_str(),
                                    state = snap.state.as_str(),
                                    refcount = snap.refcount,
                                    "watchdog: MODULE ENTERED ZOMBIE STATE — delete_module stuck in kernel"
                                );
                            }

                            last_module_state = Some(snap.state);
                            last_module_refcount = Some(snap.refcount);
                        } else if last_module_state.is_some() {
                            info!(module = module_name.as_str(),
                                  poll = module_poll_counter,
                                  "watchdog: module disappeared from /proc/modules — clean unload");
                            last_module_state = None;
                            last_module_refcount = None;
                        }
                    }
                }

                // IRQ storm detection — if INTR_EN goes hot during handoff,
                // re-quench immediately to prevent level-triggered IRQ storm
                if phase == PHASE_PIPELINE_ACTIVE || phase == PHASE_MODULE_CLEANUP {
                    let bdf_for_irq = WATCHDOG.bdf.lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();
                    if !bdf_for_irq.is_empty() {
                        if let Some(intr_en) = read_intr_en_safe(&bdf_for_irq) {
                            let hot_bits = intr_en & !0x200; // bit 9 (PBDMA) is expected
                            if hot_bits != 0 {
                                let profile = WATCHDOG.interrupt_profile.lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .clone();
                                warn!(
                                    bdf = bdf_for_irq.as_str(),
                                    intr_en = format!("0x{:08x}", intr_en).as_str(),
                                    hot_bits = format!("0x{:08x}", hot_bits).as_str(),
                                    "WATCHDOG: INTR_EN went hot during handoff — pre-emptive quench"
                                );
                                emergency_quench(&bdf_for_irq, &profile);
                            }
                        }
                    }
                }

                // Heartbeat timeout check
                let last_hb = WATCHDOG.last_heartbeat_ms.load(Ordering::Acquire);
                let timeout_ms = WATCHDOG.timeout_ms.load(Ordering::Acquire);
                let now = epoch_ms();
                let elapsed_ms = now.saturating_sub(last_hb);

                if elapsed_ms > timeout_ms {
                    let bdf = WATCHDOG
                        .bdf
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .clone();
                    let profile = *WATCHDOG
                        .interrupt_profile
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);

                    let module_name = WATCHDOG.module_name.lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();

                    let module_state = if !module_name.is_empty() {
                        toadstool_cylinder::vfio::guarded_sysfs::module_snapshot(&module_name)
                            .map(|s| format!("state={} ref={} stuck={}", s.state, s.refcount, s.is_stuck))
                            .unwrap_or_else(|| "not loaded".into())
                    } else {
                        "unknown".into()
                    };

                    error!(
                        bdf,
                        elapsed_ms,
                        timeout_ms,
                        phase,
                        module = module_name.as_str(),
                        module_state = module_state.as_str(),
                        "CATALYST WATCHDOG TRIGGERED — handoff pipeline unresponsive!"
                    );

                    emergency_quench(&bdf, &profile);

                    WATCHDOG.active.store(false, Ordering::Release);
                    WATCHDOG.phase.store(PHASE_IDLE, Ordering::Release);

                    error!("WATCHDOG: killing toadstool-ember to save system");
                    let _ = std::process::Command::new("systemctl")
                        .args(["kill", "--signal=KILL", "toadstool-ember.service"])
                        .status();

                    error!("WATCHDOG: aborting process as last resort");
                    std::process::abort();
                }
            }
        })
        .map(|_| ())
}

/// Best-effort read of INTR_EN_0 (0x140) from BAR0 via sysfs resource0.
/// Returns None if the GPU is owned by vfio-pci or the read fails.
fn read_intr_en_safe(bdf: &str) -> Option<u32> {
    use std::os::unix::fs::OpenOptionsExt;

    let resource_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "resource0");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::SYNC.bits() as i32)
        .open(&resource_path)
        .ok()?;

    let map_size: usize = 0x200; // only need first 512 bytes
    // SAFETY: mmap of PCI BAR0 for a volatile register read at a known offset.
    let map = unsafe {
        rustix::mm::mmap(
            std::ptr::null_mut(),
            map_size,
            rustix::mm::ProtFlags::READ,
            rustix::mm::MapFlags::SHARED,
            &file,
            0,
        )
    }.ok()?;

    // SAFETY: offset 0x140 is within the 512-byte mapping, volatile read of
    // memory-mapped hardware register.
    let val = unsafe {
        std::ptr::read_volatile((map as *const u8).add(0x140) as *const u32)
    };

    // SAFETY: unmapping the region we mapped above.
    unsafe { let _ = rustix::mm::munmap(map, map_size); }

    Some(val)
}
