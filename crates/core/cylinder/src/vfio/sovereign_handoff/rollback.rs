// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

use crate::vfio::guarded_sysfs;
use crate::vfio::module_patch::ModulePatchResult;

use super::types::{HandoffResult, HandoffStep};

/// Halt result with rollback. Runs best-effort recovery before returning.
///
/// Rollback triggers when any of:
/// - `module_loaded` is true (need to rmmod)
/// - `sibling_state` is non-empty (siblings were unbound)
/// - `needs_device_rollback` is true (device was unbound from its original
///   driver and needs to be restored to vfio-pci)
#[allow(clippy::too_many_arguments, reason = "WIP upstream — parameter struct refactor pending")]
pub(crate) fn halt_result(
    bdf: &str,
    halted_at: &str,
    steps: Vec<HandoffStep>,
    patch_result: Option<ModulePatchResult>,
    module_loaded: bool,
    module_unloaded: bool,
    start: Instant,
    sibling_state: &[(String, Option<String>)],
    module_name: &str,
    needs_device_rollback: bool,
) -> HandoffResult {
    halt_result_inner(bdf, halted_at, steps, patch_result, module_loaded,
                      module_unloaded, start, sibling_state, module_name,
                      needs_device_rollback, false)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn halt_result_poisoned(
    bdf: &str,
    halted_at: &str,
    steps: Vec<HandoffStep>,
    patch_result: Option<ModulePatchResult>,
    module_loaded: bool,
    module_unloaded: bool,
    start: Instant,
    sibling_state: &[(String, Option<String>)],
    module_name: &str,
    needs_device_rollback: bool,
) -> HandoffResult {
    halt_result_inner(bdf, halted_at, steps, patch_result, module_loaded,
                      module_unloaded, start, sibling_state, module_name,
                      needs_device_rollback, true)
}

#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub(crate) fn halt_result_inner(
    bdf: &str,
    halted_at: &str,
    mut steps: Vec<HandoffStep>,
    patch_result: Option<ModulePatchResult>,
    module_loaded: bool,
    module_unloaded: bool,
    start: Instant,
    sibling_state: &[(String, Option<String>)],
    module_name: &str,
    needs_device_rollback: bool,
    device_poisoned: bool,
) -> HandoffResult {
    let needs_rollback = module_loaded || !sibling_state.is_empty() || needs_device_rollback;
    if needs_rollback {
        let t = Instant::now();
        let mod_name = if module_loaded { Some(module_name) } else { None };
        guarded_sysfs::handoff_rollback(bdf, mod_name, sibling_state, device_poisoned);
        let kind = if device_poisoned { "poisoned-abandon" } else { "best-effort recovery" };
        steps.push(HandoffStep {
            name: "rollback".into(), ok: !device_poisoned,
            detail: Some(format!("{kind} (module={}, siblings={}, device={}, poisoned={})",
                module_loaded, sibling_state.len(), needs_device_rollback, device_poisoned)),
            duration_ms: t.elapsed().as_millis() as u64,
        });
    }

    HandoffResult {
        bdf: bdf.into(),
        success: false,
        halted_at: Some(halted_at.into()),
        steps,
        patch_result,
        tier: None,
        module_loaded,
        module_unloaded,
        catalyst_snapshot_path: None,
        catalyst_alive_count: None,
        catalyst_tier: None,
        boot_service_evidence: None,
        pri_ring_anchor: None,
        total_ms: start.elapsed().as_millis() as u64,
    }
}

/// Overall deadline exceeded — run rollback and return.
pub(crate) fn deadline_exceeded(
    bdf: &str,
    mut steps: Vec<HandoffStep>,
    patch_result: Option<ModulePatchResult>,
    module_loaded: bool,
    module_name: &str,
    sibling_state: &[(String, Option<String>)],
    start: Instant,
) -> HandoffResult {
    tracing::error!(bdf, elapsed_ms = start.elapsed().as_millis() as u64,
                    "handoff deadline exceeded — running rollback");
    steps.push(HandoffStep {
        name: "deadline".into(), ok: false,
        detail: Some(format!("{}ms deadline exceeded at {}ms",
            guarded_sysfs::HANDOFF_DEADLINE.as_millis(),
            start.elapsed().as_millis())),
        duration_ms: 0,
    });

    let mod_name = if module_loaded { Some(module_name) } else { None };
    guarded_sysfs::handoff_rollback(bdf, mod_name, sibling_state, false);

    HandoffResult {
        bdf: bdf.into(),
        success: false,
        halted_at: Some("deadline".into()),
        steps,
        patch_result,
        tier: None,
        module_loaded,
        module_unloaded: false,
        catalyst_snapshot_path: None,
        catalyst_alive_count: None,
        catalyst_tier: None,
        boot_service_evidence: None,
        pri_ring_anchor: None,
        total_ms: start.elapsed().as_millis() as u64,
    }
}
