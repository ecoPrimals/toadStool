// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from catalyst_watchdog.rs (S333).

use std::sync::atomic::Ordering;
use std::time::Duration;

use super::catalyst_watchdog::*;
use toadstool_cylinder::nv::registers::pmc::InterruptProfile;

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

    let guard = activate(
        "0000:82:00.0",
        InterruptProfile::VOLTA_PLUS,
        None,
        "nvidia_uvm",
    );
    assert_eq!(bdf_display(), "0000:82:00.0");
    assert_eq!(module_name_display(), "nvidia_uvm");
    drop(guard);
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

    let guard = activate("0:0:0.0", InterruptProfile::VOLTA_PLUS, None, "test");
    assert_eq!(current_phase(), Phase::PipelineActive);

    enter_module_cleanup("nvidia");
    assert_eq!(current_phase(), Phase::ModuleCleanup);
    assert_eq!(module_name_display(), "nvidia");

    exit_module_cleanup();
    assert_eq!(current_phase(), Phase::PipelineActive);
    drop(guard);
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

    let guard = activate("0000:01:00.0", InterruptProfile::VOLTA_PLUS, None, "nvidia");
    let status = defense_status();
    assert_eq!(status["phase"], "PipelineActive");
    assert_eq!(status["bdf"], "0000:01:00.0");
    assert_eq!(status["mechanisms"]["interrupt_quench"], true);
    assert_eq!(status["mechanisms"]["exclusion_guard"], true);
    assert_eq!(status["mechanisms"]["fire_and_poll_unbind"], false);
    drop(guard);
}

#[test]
fn defense_status_module_cleanup_shape() {
    let _lock = reset_watchdog();

    let guard = activate("0:0:0.0", InterruptProfile::VOLTA_PLUS, None, "test");
    enter_module_cleanup("nvidia");
    let status = defense_status();
    assert_eq!(status["phase"], "ModuleCleanup");
    assert_eq!(status["mechanisms"]["fire_and_poll_unbind"], true);
    assert_eq!(status["mechanisms"]["exclusion_guard"], true);
    exit_module_cleanup();
    drop(guard);
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

    let guard = activate(
        "0:0:0.0",
        InterruptProfile::VOLTA_PLUS,
        Some(Duration::from_mins(1)),
        "nvidia_drm",
    );
    let status = watchdog_status();
    assert_eq!(status["timeout_s"], 60);
    assert_eq!(status["phase"], "PipelineActive");
    assert_eq!(status["module_name"], "nvidia_drm");
    drop(guard);
}

#[test]
fn activate_default_timeout_is_two_minutes() {
    let _lock = reset_watchdog();

    let guard = activate("0:0:0.0", InterruptProfile::VOLTA_PLUS, None, "test");
    let stored = WATCHDOG.timeout_ms.load(Ordering::Acquire);
    assert_eq!(stored, 120_000);
    drop(guard);
}

#[test]
fn epoch_ms_returns_plausible_timestamp() {
    let now = epoch_ms();
    assert!(now > 1_700_000_000_000);
}
