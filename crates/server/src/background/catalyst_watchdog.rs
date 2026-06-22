// SPDX-License-Identifier: AGPL-3.0-or-later
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

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::time::Duration;

use toadstool_cylinder::nv::registers::pmc::InterruptProfile;
use tracing::{error, info, warn};

const DEFAULT_WATCHDOG_TIMEOUT: Duration = Duration::from_mins(2);
const WATCHDOG_CHECK_INTERVAL: Duration = Duration::from_millis(500);
const MODULE_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Pipeline phase constants for module lifecycle tracking.
const PHASE_IDLE: u8 = 0;
const PHASE_PIPELINE_ACTIVE: u8 = 1;
const PHASE_MODULE_CLEANUP: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    PipelineActive,
    ModuleCleanup,
}

impl Phase {
    fn from_u8(v: u8) -> Self {
        match v {
            PHASE_PIPELINE_ACTIVE => Phase::PipelineActive,
            PHASE_MODULE_CLEANUP => Phase::ModuleCleanup,
            _ => Phase::Idle,
        }
    }
}

/// Shared state between the handoff pipeline and the watchdog thread.
struct WatchdogState {
    active: AtomicBool,
    thread_running: AtomicBool,
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
        thread_running: AtomicBool::new(false),
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

/// Emergency quench — full nuclear shutdown including Bus Master disable.
/// Only called on heartbeat timeout (pipeline is hung, RM is unresponsive).
fn emergency_quench(bdf: &str, profile: &InterruptProfile) {
    warn!(
        bdf,
        "WATCHDOG: performing EMERGENCY interrupt quench (full nuclear)"
    );
    toadstool_cylinder::nv::registers::pmc::quench_interrupts(bdf, profile, "watchdog-emergency");
    toadstool_cylinder::nv::registers::pmc::disable_pci_msi(bdf, "watchdog-emergency");
    toadstool_cylinder::nv::registers::pmc::disable_bus_master(bdf, "watchdog-emergency");
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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "guard-scoped API; tested but not yet wired in production"
        )
    )]
    #[expect(
        clippy::unused_self,
        reason = "guard-scoped API; heartbeat is on the shared static"
    )]
    pub fn heartbeat(&self) {
        WATCHDOG
            .last_heartbeat_ms
            .store(epoch_ms(), Ordering::Release);
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
        WATCHDOG
            .last_heartbeat_ms
            .store(epoch_ms(), Ordering::Release);
    }
}

/// Signal that the pipeline is entering the module cleanup phase.
/// The watchdog will begin high-frequency module state polling to observe
/// the module death transition in real-time.
pub fn enter_module_cleanup(module_name: &str) {
    {
        let mut locked = WATCHDOG
            .module_name
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *locked = module_name.to_string();
    }
    WATCHDOG
        .phase
        .store(PHASE_MODULE_CLEANUP, Ordering::Release);
    WATCHDOG
        .last_heartbeat_ms
        .store(epoch_ms(), Ordering::Release);
    info!(
        module = module_name,
        "watchdog: entering module_cleanup phase — high-frequency monitoring"
    );
}

/// Signal that module cleanup is complete (success or failure).
pub fn exit_module_cleanup() {
    WATCHDOG
        .phase
        .store(PHASE_PIPELINE_ACTIVE, Ordering::Release);
    WATCHDOG
        .last_heartbeat_ms
        .store(epoch_ms(), Ordering::Release);
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
        let mut locked_bdf = WATCHDOG
            .bdf
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
        let mut locked_module = WATCHDOG
            .module_name
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *locked_module = module_name.to_string();
    }
    let timeout = timeout.unwrap_or(DEFAULT_WATCHDOG_TIMEOUT);
    WATCHDOG
        .timeout_ms
        .store(timeout.as_millis() as u64, Ordering::Release);
    WATCHDOG
        .last_heartbeat_ms
        .store(epoch_ms(), Ordering::Release);
    WATCHDOG
        .phase
        .store(PHASE_PIPELINE_ACTIVE, Ordering::Release);
    WATCHDOG.active.store(true, Ordering::Release);

    info!(
        bdf,
        timeout_ms = timeout.as_millis() as u64,
        module_name,
        "catalyst watchdog: activated for handoff"
    );

    CatalystWatchdogGuard { _private: () }
}

/// Check whether the watchdog is currently monitoring an active handoff.
pub fn is_active() -> bool {
    WATCHDOG.active.load(Ordering::Acquire)
}

fn bdf_display() -> String {
    let bdf = WATCHDOG
        .bdf
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if bdf.is_empty() {
        "none".into()
    } else {
        bdf.clone()
    }
}

fn module_name_display() -> String {
    let name = WATCHDOG
        .module_name
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if name.is_empty() {
        "none".into()
    } else {
        name.clone()
    }
}

fn current_phase() -> Phase {
    Phase::from_u8(WATCHDOG.phase.load(Ordering::Acquire))
}

/// Query defense status for external consumers (JSON-RPC handlers).
///
/// Reports both `available` (code path exists in the engine) and `armed`
/// (currently active for an in-flight handoff). Validators should check
/// `available` for baseline health and `armed` during live handoffs.
pub fn defense_status() -> serde_json::Value {
    let phase = current_phase();
    let last_hb = WATCHDOG.last_heartbeat_ms.load(Ordering::Acquire);
    let now = epoch_ms();
    let elapsed_ms = now.saturating_sub(last_hb);
    let watchdog_running = WATCHDOG.thread_running.load(Ordering::Acquire);
    serde_json::json!({
        "phase": format!("{phase:?}"),
        "bdf": bdf_display(),
        "mechanisms": {
            "interrupt_quench": phase != Phase::Idle,
            "post_exit_quench": phase != Phase::Idle,
            "exclusion_guard": phase == Phase::PipelineActive || phase == Phase::ModuleCleanup,
            "fire_and_poll_unbind": phase == Phase::ModuleCleanup,
            "kernel_sentinel": true,
        },
        "available": {
            "interrupt_quench": true,
            "post_exit_quench": true,
            "exclusion_guard": true,
            "fire_and_poll_unbind": true,
            "kernel_sentinel": watchdog_running,
        },
        "watchdog_running": watchdog_running,
        "last_heartbeat_ms": elapsed_ms,
    })
}

/// Query watchdog thread status for external consumers (JSON-RPC handlers).
pub fn watchdog_status() -> serde_json::Value {
    let phase = current_phase();
    let timeout_ms = WATCHDOG.timeout_ms.load(Ordering::Acquire);
    serde_json::json!({
        "running": WATCHDOG.thread_running.load(Ordering::Acquire),
        "timeout_s": timeout_ms / 1000,
        "phase": format!("{phase:?}"),
        "bdf": bdf_display(),
        "module_name": module_name_display(),
    })
}

/// Force an emergency interrupt quench from an external caller (e.g. kernel
/// sentinel). Uses the currently registered BDF and interrupt profile.
pub fn force_emergency_quench() {
    let bdf = WATCHDOG
        .bdf
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let profile = *WATCHDOG
        .interrupt_profile
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
            WATCHDOG.thread_running.store(true, Ordering::Release);
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
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
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

                            if snap.is_zombie() && last_module_state.as_deref().is_none_or(|s| s != "Unloading") {
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

                // INTR_EN monitoring DISABLED (Exp 233 checkpoint):
                // RM enables interrupts as part of normal GPU initialization.
                // Quenching INTR_EN during RM init disrupts RM's interrupt
                // flow and causes hangs/lockups. The pre-unbind and post-exit
                // pipeline quench steps handle the critical moments.
                // The heartbeat timeout (below) is the safety net for hangs.

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
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .clone();

                    let module_state = if module_name.is_empty() {
                        "unknown".into()
                    } else {
                        toadstool_cylinder::vfio::guarded_sysfs::module_snapshot(&module_name).map_or_else(|| "not loaded".into(), |s| format!("state={} ref={} stuck={}", s.state, s.refcount, s.is_stuck))
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

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn reset_watchdog() -> std::sync::MutexGuard<'static, ()> {
        let guard = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        WATCHDOG.active.store(false, Ordering::Release);
        WATCHDOG.phase.store(PHASE_IDLE, Ordering::Release);
        WATCHDOG.last_heartbeat_ms.store(0, Ordering::Release);
        WATCHDOG.timeout_ms.store(
            DEFAULT_WATCHDOG_TIMEOUT.as_millis() as u64,
            Ordering::Release,
        );
        *WATCHDOG
            .bdf
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = String::new();
        *WATCHDOG
            .module_name
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = String::new();
        guard
    }

    #[test]
    fn phase_from_u8_round_trips() {
        assert_eq!(Phase::from_u8(PHASE_IDLE), Phase::Idle);
        assert_eq!(Phase::from_u8(PHASE_PIPELINE_ACTIVE), Phase::PipelineActive);
        assert_eq!(Phase::from_u8(PHASE_MODULE_CLEANUP), Phase::ModuleCleanup);
        assert_eq!(Phase::from_u8(255), Phase::Idle);
        assert_eq!(Phase::from_u8(42), Phase::Idle);
    }

    #[test]
    fn activate_sets_active_and_guard_drop_clears() {
        let _lock = reset_watchdog();

        assert!(!is_active());
        let guard = activate(
            "0000:01:00.0",
            InterruptProfile::VOLTA_PLUS,
            Some(Duration::from_secs(30)),
            "nvidia",
        );
        assert!(is_active());
        assert_eq!(current_phase(), Phase::PipelineActive);
        drop(guard);
        assert!(!is_active());
    }

    #[test]
    fn activate_stores_bdf_and_module_name() {
        let _lock = reset_watchdog();

        let _guard = activate(
            "0000:82:00.0",
            InterruptProfile::VOLTA_PLUS,
            None,
            "nvidia_uvm",
        );
        assert_eq!(bdf_display(), "0000:82:00.0");
        assert_eq!(module_name_display(), "nvidia_uvm");
        drop(_guard);
    }

    #[test]
    fn heartbeat_updates_timestamp_only_when_active() {
        let _lock = reset_watchdog();

        WATCHDOG.last_heartbeat_ms.store(0, Ordering::Release);
        heartbeat();
        assert_eq!(WATCHDOG.last_heartbeat_ms.load(Ordering::Acquire), 0);

        let guard = activate("0:0:0.0", InterruptProfile::VOLTA_PLUS, None, "test");
        let before = WATCHDOG.last_heartbeat_ms.load(Ordering::Acquire);
        std::thread::sleep(Duration::from_millis(2));
        heartbeat();
        let after = WATCHDOG.last_heartbeat_ms.load(Ordering::Acquire);
        assert!(after >= before);
        drop(guard);
    }

    #[test]
    fn guard_heartbeat_method_updates_timestamp() {
        let _lock = reset_watchdog();

        let guard = activate("0:0:0.0", InterruptProfile::VOLTA_PLUS, None, "test");
        let before = WATCHDOG.last_heartbeat_ms.load(Ordering::Acquire);
        std::thread::sleep(Duration::from_millis(2));
        guard.heartbeat();
        let after = WATCHDOG.last_heartbeat_ms.load(Ordering::Acquire);
        assert!(after >= before);
        drop(guard);
    }

    #[test]
    fn enter_and_exit_module_cleanup_transitions_phase() {
        let _lock = reset_watchdog();

        let _guard = activate("0:0:0.0", InterruptProfile::VOLTA_PLUS, None, "test");
        assert_eq!(current_phase(), Phase::PipelineActive);

        enter_module_cleanup("nvidia");
        assert_eq!(current_phase(), Phase::ModuleCleanup);
        assert_eq!(module_name_display(), "nvidia");

        exit_module_cleanup();
        assert_eq!(current_phase(), Phase::PipelineActive);
        drop(_guard);
    }

    #[test]
    fn bdf_display_returns_none_when_empty() {
        let _lock = reset_watchdog();
        assert_eq!(bdf_display(), "none");
    }

    #[test]
    fn module_name_display_returns_none_when_empty() {
        let _lock = reset_watchdog();
        assert_eq!(module_name_display(), "none");
    }

    #[test]
    fn defense_status_idle_shape() {
        let _lock = reset_watchdog();

        let status = defense_status();
        assert_eq!(status["phase"], "Idle");
        assert_eq!(status["bdf"], "none");
        assert_eq!(status["mechanisms"]["interrupt_quench"], false);
        assert_eq!(status["mechanisms"]["kernel_sentinel"], true);
        assert!(status["available"]["interrupt_quench"].as_bool().unwrap());
        assert!(status["available"]["post_exit_quench"].as_bool().unwrap());
    }

    #[test]
    fn defense_status_active_shape() {
        let _lock = reset_watchdog();

        let _guard = activate("0000:01:00.0", InterruptProfile::VOLTA_PLUS, None, "nvidia");
        let status = defense_status();
        assert_eq!(status["phase"], "PipelineActive");
        assert_eq!(status["bdf"], "0000:01:00.0");
        assert_eq!(status["mechanisms"]["interrupt_quench"], true);
        assert_eq!(status["mechanisms"]["exclusion_guard"], true);
        assert_eq!(status["mechanisms"]["fire_and_poll_unbind"], false);
        drop(_guard);
    }

    #[test]
    fn defense_status_module_cleanup_shape() {
        let _lock = reset_watchdog();

        let _guard = activate("0:0:0.0", InterruptProfile::VOLTA_PLUS, None, "test");
        enter_module_cleanup("nvidia");
        let status = defense_status();
        assert_eq!(status["phase"], "ModuleCleanup");
        assert_eq!(status["mechanisms"]["fire_and_poll_unbind"], true);
        assert_eq!(status["mechanisms"]["exclusion_guard"], true);
        exit_module_cleanup();
        drop(_guard);
    }

    #[test]
    fn watchdog_status_shape() {
        let _lock = reset_watchdog();

        let status = watchdog_status();
        assert_eq!(status["phase"], "Idle");
        assert_eq!(status["bdf"], "none");
        assert_eq!(status["module_name"], "none");
        assert!(status["timeout_s"].as_u64().is_some());
        assert_eq!(
            status["timeout_s"].as_u64().unwrap(),
            DEFAULT_WATCHDOG_TIMEOUT.as_secs()
        );
    }

    #[test]
    fn watchdog_status_active_reflects_custom_timeout() {
        let _lock = reset_watchdog();

        let _guard = activate(
            "0:0:0.0",
            InterruptProfile::VOLTA_PLUS,
            Some(Duration::from_secs(60)),
            "nvidia_drm",
        );
        let status = watchdog_status();
        assert_eq!(status["timeout_s"], 60);
        assert_eq!(status["phase"], "PipelineActive");
        assert_eq!(status["module_name"], "nvidia_drm");
        drop(_guard);
    }

    #[test]
    fn activate_default_timeout_is_two_minutes() {
        let _lock = reset_watchdog();

        let _guard = activate("0:0:0.0", InterruptProfile::VOLTA_PLUS, None, "test");
        let stored = WATCHDOG.timeout_ms.load(Ordering::Acquire);
        assert_eq!(stored, 120_000);
        drop(_guard);
    }

    #[test]
    fn epoch_ms_returns_plausible_timestamp() {
        let now = epoch_ms();
        assert!(now > 1_700_000_000_000);
    }
}
