// SPDX-License-Identifier: AGPL-3.0-or-later
//! PMU init sequence applicator.
//!
//! Replays hw-learn recipes via BAR0 MMIO to initialize the compute
//! engine on GPUs without PMU firmware (Volta desktop).
//!
//! This module delegates to [`hw_learn::RecipeApplicator`] for the actual
//! register write loop, converting the legacy JSON recipe format to the
//! canonical [`hw_learn::distiller::InitRecipe`].
//!
//! # Register Snapshot & Rollback
//!
//! Before applying a recipe, [`apply_with_recovery`] captures the current
//! values of all registers that will be written. On failure, it attempts
//! to restore the original values. This is best-effort — hardware may
//! not accept rollback writes after entering a bad state.
//!
//! # Usage
//!
//! ```rust,no_run
//! # fn example() -> nvpmu::error::Result<()> {
//! use nvpmu::init::{apply_recipe, InitResult};
//! use nvpmu::Bar0Access;
//!
//! let recipe_json = std::fs::read_to_string("gv100.json")?;
//! let mut bar0 = Bar0Access::open("0000:65:00.0")?;
//! let result = apply_recipe(&recipe_json, &mut bar0)?;
//! assert!(result.success);
//! # Ok(())
//! # }
//! ```
//!
//! # Safety
//!
//! This module performs direct register writes to GPU hardware.
//! Thermal safety is checked before and after the sequence.

use crate::error::{NvPmuError, Result};
use crate::hwmon::HwmonSensors;
use crate::monitor::{MonitorConfig, assert_thermal_safe};
use hw_learn::applicator::{ApplyVerdict, RecipeApplicator, RegisterAccess};
use hw_learn::distiller::{
    DriverKind, GpuArch, InitRecipe, InitStep, RegFunction, Vendor, VerifyCheck,
};

/// Default DRI device node for PMU init (first GPU render node).
/// Future: accept device path as parameter for multi-GPU support.
const DEFAULT_DRI_DEVICE: &str = "/dev/dri/card0";

/// Result of applying a PMU init recipe.
#[derive(Debug, serde::Serialize)]
pub struct InitResult {
    /// Target chip codename.
    pub chip: String,
    /// Number of register write steps successfully applied.
    pub steps_applied: usize,
    /// Number of steps that failed.
    pub steps_failed: usize,
    /// Number of verify steps that passed.
    pub verify_passed: usize,
    /// Number of verify steps that failed.
    pub verify_failed: usize,
    /// Whether the recipe completed successfully.
    pub success: bool,
    /// Whether rollback was attempted (only with `apply_with_recovery`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_attempted: Option<bool>,
    /// Whether rollback succeeded (only when `rollback_attempted` is true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_succeeded: Option<bool>,
}

/// Pre-init register snapshot for rollback.
#[derive(Debug)]
pub struct RegisterSnapshot {
    entries: Vec<(u64, u32)>,
}

impl RegisterSnapshot {
    /// Capture current values of all registers that the recipe will write.
    pub fn capture(recipe: &InitRecipe, access: &dyn RegisterAccess) -> Self {
        let mut entries = Vec::new();
        for step in &recipe.steps {
            if let InitStep::RegisterWrite { offset, .. } = step
                && let Ok(val) = access.read_u32(*offset)
            {
                entries.push((*offset, val));
            }
        }
        tracing::debug!(registers = entries.len(), "captured register snapshot");
        Self { entries }
    }

    /// Restore registers to their captured values (best-effort).
    ///
    /// Returns `true` if all writes succeeded, `false` if any failed.
    pub fn rollback(&self, access: &mut dyn RegisterAccess) -> bool {
        let mut all_ok = true;
        for &(offset, value) in self.entries.iter().rev() {
            if let Err(e) = access.write_u32(offset, value) {
                tracing::error!(offset = %format!("{offset:#x}"), "rollback write failed: {e}");
                all_ok = false;
            }
        }
        if all_ok {
            tracing::info!(
                registers = self.entries.len(),
                "rollback completed successfully"
            );
        } else {
            tracing::error!("rollback partially failed — GPU may be in inconsistent state");
        }
        all_ok
    }

    /// Number of register values captured.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the snapshot captured any registers.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Legacy JSON recipe step format (kept for backward-compatible deserialization).
#[derive(Debug, serde::Deserialize)]
struct RecipeStep {
    offset: u64,
    value: u64,
    #[expect(
        dead_code,
        reason = "preserved for recipe format compatibility; future use for 8/16-bit writes"
    )]
    width: u8,
    #[serde(default)]
    delay_us: Option<u64>,
}

/// Legacy verification read format.
#[derive(Debug, serde::Deserialize)]
struct VerifyRead {
    offset: u64,
    expected_mask: u64,
    expected_value: u64,
}

/// Legacy JSON recipe format.
#[derive(Debug, serde::Deserialize)]
struct Recipe {
    chip: String,
    steps: Vec<RecipeStep>,
    #[serde(default)]
    verify_reads: Vec<VerifyRead>,
}

/// Convert a legacy `Recipe` to the canonical `InitRecipe` format.
fn to_init_recipe(recipe: &Recipe) -> InitRecipe {
    let mut steps: Vec<InitStep> =
        Vec::with_capacity(recipe.steps.len() + recipe.verify_reads.len());

    for s in &recipe.steps {
        steps.push(InitStep::RegisterWrite {
            offset: s.offset,
            value: s.value,
            function: RegFunction::Unknown,
        });
        if let Some(us) = s.delay_us {
            steps.push(InitStep::Delay { us });
        }
    }

    for v in &recipe.verify_reads {
        steps.push(InitStep::Verify {
            check: VerifyCheck::RegisterMatch {
                offset: v.offset,
                expected: v.expected_value,
                mask: v.expected_mask,
            },
        });
    }

    let arch = GpuArch {
        vendor: Vendor::Nvidia,
        generation: String::new(),
        chip: recipe.chip.clone(),
        compute_class: String::new(),
    };

    InitRecipe {
        source_arch: arch.clone(),
        source_driver: DriverKind::Nouveau,
        target_arch: arch,
        steps,
        confidence: 0.0,
        description: format!("Legacy nvpmu recipe for {}", recipe.chip),
    }
}

fn tally_results(chip: String, result: &hw_learn::applicator::ApplyResult) -> InitResult {
    InitResult {
        chip,
        steps_applied: result.step_results.iter().filter(|r| r.success).count(),
        steps_failed: result.step_results.iter().filter(|r| !r.success).count(),
        verify_passed: result
            .step_results
            .iter()
            .filter(|r| r.success && r.detail.contains("verify"))
            .count(),
        verify_failed: result
            .step_results
            .iter()
            .filter(|r| !r.success && r.detail.contains("verify"))
            .count(),
        success: result.verdict == ApplyVerdict::Success,
        rollback_attempted: None,
        rollback_succeeded: None,
    }
}

/// Apply a PMU init recipe from JSON via any `RegisterAccess` backend.
///
/// Accepts both the legacy nvpmu JSON format (`{ chip, steps, verify_reads }`)
/// and the canonical hw-learn `InitRecipe` format. The actual write loop is
/// delegated to [`hw_learn::RecipeApplicator`].
///
/// Works with `Bar0Access` (sysfs), `VfioBar0Access` (VFIO), or any custom
/// `RegisterAccess` implementation.
///
/// # Errors
///
/// Returns error if JSON parsing fails or BAR0 read/write fails.
pub fn apply_recipe(
    recipe_json: &str,
    register_access: &mut dyn RegisterAccess,
) -> Result<InitResult> {
    let (chip, init_recipe) = if let Ok(legacy) = serde_json::from_str::<Recipe>(recipe_json) {
        let chip = legacy.chip.clone();
        (chip, to_init_recipe(&legacy))
    } else {
        let canonical: InitRecipe = serde_json::from_str(recipe_json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let chip = canonical.target_arch.chip.clone();
        (chip, canonical)
    };

    tracing::info!(chip = %chip, steps = init_recipe.steps.len(), "applying PMU init recipe via hw-learn");

    let mut applicator = RecipeApplicator::new(false).with_register_access(register_access);
    let result = applicator.apply(&init_recipe, DEFAULT_DRI_DEVICE);

    if result.verdict != ApplyVerdict::Success {
        for sr in &result.step_results {
            if !sr.success {
                tracing::error!(step = sr.step_index, detail = %sr.detail, "step FAILED");
            }
        }
    }

    Ok(tally_results(chip, &result))
}

/// Apply a recipe with thermal safety checks.
///
/// Reads GPU temperature before applying. Aborts if the GPU is above
/// the critical temperature threshold.
///
/// # Errors
///
/// Returns error if thermal safety is violated or recipe application fails.
pub fn apply_recipe_safe(
    recipe_json: &str,
    register_access: &mut dyn RegisterAccess,
    sensors: &HwmonSensors,
    config: &MonitorConfig,
) -> Result<InitResult> {
    assert_thermal_safe(sensors, config)?;

    let result = apply_recipe(recipe_json, register_access)?;

    if !result.success {
        tracing::error!(
            chip = %result.chip,
            failed = result.steps_failed,
            "PMU init recipe partially failed — GPU may be in inconsistent state"
        );
    }

    Ok(result)
}

/// Apply a canonical `InitRecipe` with thermal check and snapshot/rollback.
///
/// 1. Checks thermal safety
/// 2. Captures register snapshot (pre-init values)
/// 3. Applies recipe
/// 4. On failure: rolls back to snapshot (best-effort)
///
/// This is the recommended entry point for all init operations.
///
/// # Errors
///
/// Returns error if thermal safety is violated. On partial init failure,
/// returns `NvPmuError::PartialInit` with rollback status.
pub fn apply_with_recovery(
    recipe: &InitRecipe,
    register_access: &mut dyn RegisterAccess,
    sensors: &HwmonSensors,
    config: &MonitorConfig,
) -> Result<InitResult> {
    assert_thermal_safe(sensors, config)?;

    let chip = recipe.target_arch.chip.clone();
    tracing::info!(chip = %chip, steps = recipe.steps.len(), "applying init recipe with recovery");

    let snapshot = RegisterSnapshot::capture(recipe, register_access);

    let mut applicator = RecipeApplicator::new(false).with_register_access(register_access);
    let result = applicator.apply(recipe, DEFAULT_DRI_DEVICE);

    let success = result.verdict == ApplyVerdict::Success;

    if success {
        return Ok(InitResult {
            rollback_attempted: Some(false),
            rollback_succeeded: None,
            ..tally_results(chip, &result)
        });
    }

    for sr in &result.step_results {
        if !sr.success {
            tracing::error!(step = sr.step_index, detail = %sr.detail, "step FAILED");
        }
    }

    tracing::warn!(chip = %chip, "init failed — attempting rollback");
    let rollback_ok = snapshot.rollback(register_access);

    let mut init_result = tally_results(chip, &result);
    init_result.rollback_attempted = Some(true);
    init_result.rollback_succeeded = Some(rollback_ok);

    Err(NvPmuError::PartialInit {
        applied: init_result.steps_applied,
        total: init_result.steps_applied + init_result.steps_failed,
        rollback_status: if rollback_ok {
            "succeeded".to_string()
        } else {
            "partial — GPU may need reset".to_string()
        },
    })
}

/// Apply a canonical `InitRecipe` directly (no JSON parsing).
///
/// Prefer [`apply_with_recovery`] for production use — it adds snapshot/rollback.
/// This simpler entry point is for backward compatibility.
///
/// # Errors
///
/// Returns error if thermal safety is violated or recipe application fails.
pub fn apply_init_recipe(
    recipe: &InitRecipe,
    register_access: &mut dyn RegisterAccess,
    sensors: &HwmonSensors,
    config: &MonitorConfig,
) -> Result<InitResult> {
    assert_thermal_safe(sensors, config)?;

    let chip = recipe.target_arch.chip.clone();
    tracing::info!(chip = %chip, steps = recipe.steps.len(), "applying init recipe via hw-learn");

    let mut applicator = RecipeApplicator::new(false).with_register_access(register_access);
    let result = applicator.apply(recipe, DEFAULT_DRI_DEVICE);

    if result.verdict != ApplyVerdict::Success {
        for sr in &result.step_results {
            if !sr.success {
                tracing::error!(step = sr.step_index, detail = %sr.detail, "step FAILED");
            }
        }
    }

    Ok(tally_results(chip, &result))
}

#[cfg(test)]
#[path = "init_tests.rs"]
mod tests;
