// SPDX-License-Identifier: AGPL-3.0-only
//! PMU init sequence applicator.
//!
//! Replays hw-learn recipes via BAR0 MMIO to initialize the compute
//! engine on GPUs without PMU firmware (Volta desktop).
//!
//! This module delegates to [`hw_learn::RecipeApplicator`] for the actual
//! register write loop, converting the legacy JSON recipe format to the
//! canonical [`hw_learn::distiller::InitRecipe`].
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

use crate::bar0::Bar0Access;
use crate::error::Result;
use crate::hwmon::HwmonSensors;
use crate::monitor::{assert_thermal_safe, MonitorConfig};
use hw_learn::applicator::{ApplyVerdict, RecipeApplicator};
use hw_learn::distiller::{
    DriverKind, GpuArch, InitRecipe, InitStep, RegFunction, Vendor, VerifyCheck,
};

/// Result of applying a PMU init recipe.
#[derive(Debug, serde::Serialize)]
pub struct InitResult {
    pub chip: String,
    pub steps_applied: usize,
    pub steps_failed: usize,
    pub verify_passed: usize,
    pub verify_failed: usize,
    pub success: bool,
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

/// Apply a PMU init recipe from JSON to a BAR0-mapped GPU.
///
/// Accepts both the legacy nvpmu JSON format (`{ chip, steps, verify_reads }`)
/// and the canonical hw-learn `InitRecipe` format. The actual write loop is
/// delegated to [`hw_learn::RecipeApplicator`].
///
/// # Errors
///
/// Returns error if:
/// - JSON parsing fails
/// - BAR0 read/write fails
#[allow(
    unsafe_code,
    reason = "register writes via Bar0Access require unsafe mmap operations"
)]
pub fn apply_recipe(recipe_json: &str, bar0: &mut Bar0Access) -> Result<InitResult> {
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

    let mut applicator = RecipeApplicator::new(false).with_register_access(bar0);
    let result = applicator.apply(&init_recipe, "/dev/dri/card0");

    let steps_applied = result.step_results.iter().filter(|r| r.success).count();
    let steps_failed = result.step_results.iter().filter(|r| !r.success).count();

    let verify_passed = result
        .step_results
        .iter()
        .filter(|r| r.success && r.detail.contains("verify"))
        .count();
    let verify_failed = result
        .step_results
        .iter()
        .filter(|r| !r.success && r.detail.contains("verify"))
        .count();

    let success = result.verdict == ApplyVerdict::Success;

    if !success {
        for sr in &result.step_results {
            if !sr.success {
                tracing::error!(step = sr.step_index, detail = %sr.detail, "step FAILED");
            }
        }
    }

    Ok(InitResult {
        chip,
        steps_applied,
        steps_failed,
        verify_passed,
        verify_failed,
        success,
    })
}

/// Apply a recipe with thermal safety checks.
///
/// Reads GPU temperature before applying the recipe. Aborts if the
/// GPU is already above the critical temperature.
///
/// # Errors
///
/// Returns error if thermal safety is violated or recipe application fails.
#[allow(
    unsafe_code,
    reason = "delegates to apply_recipe which writes BAR0 registers"
)]
pub fn apply_recipe_safe(
    recipe_json: &str,
    bar0: &mut Bar0Access,
    sensors: &HwmonSensors,
    config: &MonitorConfig,
) -> Result<InitResult> {
    assert_thermal_safe(sensors, config)?;

    let result = apply_recipe(recipe_json, bar0)?;

    if !result.success {
        tracing::error!(
            chip = %result.chip,
            failed = result.steps_failed,
            "PMU init recipe partially failed — GPU may be in inconsistent state"
        );
    }

    Ok(result)
}

/// Apply a canonical `InitRecipe` directly (no JSON parsing).
///
/// Prefer this over `apply_recipe` when working with hw-learn's
/// knowledge store or distiller output.
///
/// # Errors
///
/// Returns error if thermal safety is violated or recipe application fails.
#[allow(
    unsafe_code,
    reason = "register writes via Bar0Access require unsafe mmap operations"
)]
pub fn apply_init_recipe(
    recipe: &InitRecipe,
    bar0: &mut Bar0Access,
    sensors: &HwmonSensors,
    config: &MonitorConfig,
) -> Result<InitResult> {
    assert_thermal_safe(sensors, config)?;

    let chip = recipe.target_arch.chip.clone();
    tracing::info!(chip = %chip, steps = recipe.steps.len(), "applying init recipe via hw-learn");

    let mut applicator = RecipeApplicator::new(false).with_register_access(bar0);
    let result = applicator.apply(recipe, "/dev/dri/card0");

    let success = result.verdict == ApplyVerdict::Success;
    if !success {
        for sr in &result.step_results {
            if !sr.success {
                tracing::error!(step = sr.step_index, detail = %sr.detail, "step FAILED");
            }
        }
    }

    Ok(InitResult {
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
        success,
    })
}
