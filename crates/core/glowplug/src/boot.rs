// SPDX-License-Identifier: AGPL-3.0-or-later

//! Sovereign boot orchestration types.
//!
//! Absorbed from `coral-glowplug::sovereign` — these types describe the
//! outcome of a full device boot sequence (detect → warm → swap → init).
//!
//! The boot lifecycle:
//! 1. **Detect** — read the current driver/personality from sysfs
//! 2. **Connect** — verify the ember subsystem is reachable
//! 3. **Warm** — if the device is cold, cycle through a warm driver and back
//! 4. **Recipe** — load a cached training recipe if one exists
//! 5. **Init** — run the sovereign init pipeline
//! 6. **Health** — verify the device is compute-ready
//!
//! Each step is recorded as a [`BootStep`] with timing and status.

use serde::{Deserialize, Serialize};

/// Result of a full sovereign boot sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootResult {
    /// Device identifier (PCI BDF or equivalent).
    pub device_id: String,
    /// Driver/personality bound when we started.
    pub initial_personality: Option<String>,
    /// Whether a warm cycle was performed to initialise hardware state.
    pub warm_cycle_performed: bool,
    /// Driver/personality bound after orchestration.
    pub final_personality: Option<String>,
    /// Sovereign init result (raw JSON from the init pipeline).
    pub init_result: Option<serde_json::Value>,
    /// Per-step log of what the orchestrator did.
    pub steps: Vec<BootStep>,
    /// Overall success.
    pub success: bool,
    /// Human-readable summary.
    pub summary: String,
}

/// A single step in the boot orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootStep {
    /// Step identifier (e.g. "detect_driver", "swap_to_vfio").
    pub name: String,
    /// Whether this step succeeded, was skipped, or failed.
    pub status: StepStatus,
    /// Human-readable detail about what happened.
    pub detail: Option<String>,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

/// Status of an orchestration step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Step completed successfully.
    Ok,
    /// Step was not needed and was skipped.
    Skipped,
    /// Step failed (see detail for cause).
    Failed,
}

impl BootResult {
    /// Create a failed boot result with a summary message.
    #[must_use]
    pub fn failed(device_id: String, steps: Vec<BootStep>, summary: String) -> Self {
        Self {
            device_id,
            initial_personality: None,
            warm_cycle_performed: false,
            final_personality: None,
            init_result: None,
            steps,
            success: false,
            summary,
        }
    }
}

impl BootStep {
    /// Create a successful step.
    #[must_use]
    pub fn ok(name: impl Into<String>, detail: impl Into<Option<String>>, duration_ms: u64) -> Self {
        Self {
            name: name.into(),
            status: StepStatus::Ok,
            detail: detail.into(),
            duration_ms,
        }
    }

    /// Create a skipped step.
    #[must_use]
    pub fn skipped(name: impl Into<String>, detail: impl Into<Option<String>>) -> Self {
        Self {
            name: name.into(),
            status: StepStatus::Skipped,
            detail: detail.into(),
            duration_ms: 0,
        }
    }

    /// Create a failed step.
    #[must_use]
    pub fn failed(name: impl Into<String>, detail: impl Into<Option<String>>, duration_ms: u64) -> Self {
        Self {
            name: name.into(),
            status: StepStatus::Failed,
            detail: detail.into(),
            duration_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_result_failed_constructor() {
        let result = BootResult::failed("0000:01:00.0".into(), vec![], "no ember".into());
        assert!(!result.success);
        assert_eq!(result.device_id, "0000:01:00.0");
        assert_eq!(result.summary, "no ember");
        assert!(result.steps.is_empty());
    }

    #[test]
    fn boot_step_ok() {
        let step = BootStep::ok("detect_driver", Some("nvidia".to_string()), 5);
        assert_eq!(step.status, StepStatus::Ok);
        assert_eq!(step.name, "detect_driver");
        assert_eq!(step.duration_ms, 5);
    }

    #[test]
    fn boot_step_skipped() {
        let step = BootStep::skipped("load_recipe", Some("no cached recipe".to_string()));
        assert_eq!(step.status, StepStatus::Skipped);
        assert_eq!(step.duration_ms, 0);
    }

    #[test]
    fn boot_step_failed() {
        let step = BootStep::failed("swap_to_vfio", Some("timeout".to_string()), 1000);
        assert_eq!(step.status, StepStatus::Failed);
        assert_eq!(step.duration_ms, 1000);
    }

    #[test]
    fn step_status_serde_roundtrip() {
        for status in [StepStatus::Ok, StepStatus::Skipped, StepStatus::Failed] {
            let json = serde_json::to_string(&status).unwrap();
            let back: StepStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    #[test]
    fn boot_result_serde_roundtrip() {
        let result = BootResult {
            device_id: "0000:01:00.0".into(),
            initial_personality: Some("nouveau".into()),
            warm_cycle_performed: true,
            final_personality: Some("vfio-pci".into()),
            init_result: Some(serde_json::json!({"compute_ready": true})),
            steps: vec![
                BootStep::ok("detect_driver", Some("nouveau".to_string()), 2),
                BootStep::ok("swap_to_vfio", Some("total_ms=150".to_string()), 155),
                BootStep::skipped("load_recipe", Some("no recipe".to_string())),
                BootStep::ok("sovereign_init", Some("compute ready".to_string()), 3000),
            ],
            success: true,
            summary: "sovereign pipeline succeeded".into(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: BootResult = serde_json::from_str(&json).unwrap();
        assert!(back.success);
        assert_eq!(back.steps.len(), 4);
        assert!(back.warm_cycle_performed);
        assert_eq!(back.final_personality.as_deref(), Some("vfio-pci"));
    }

    #[test]
    fn boot_step_none_detail() {
        let step = BootStep::ok("health_check", None, 10);
        assert!(step.detail.is_none());
    }
}
