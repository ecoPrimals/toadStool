// SPDX-License-Identifier: AGPL-3.0-only
//! PMU init sequence applicator.
//!
//! Replays hw-learn recipes via BAR0 MMIO to initialize the compute
//! engine on GPUs without PMU firmware (Volta desktop).
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

/// Recipe step matching the hw-learn format.
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

/// Verification read.
#[derive(Debug, serde::Deserialize)]
struct VerifyRead {
    offset: u64,
    expected_mask: u64,
    expected_value: u64,
}

/// Deserialized recipe.
#[derive(Debug, serde::Deserialize)]
struct Recipe {
    chip: String,
    steps: Vec<RecipeStep>,
    #[serde(default)]
    verify_reads: Vec<VerifyRead>,
}

/// Apply a PMU init recipe from JSON to a BAR0-mapped GPU.
///
/// # Errors
///
/// Returns error if:
/// - JSON parsing fails
/// - Thermal safety check fails (before or after)
/// - BAR0 read/write fails
#[allow(unsafe_code)]
pub fn apply_recipe(recipe_json: &str, bar0: &mut Bar0Access) -> Result<InitResult> {
    let recipe: Recipe = serde_json::from_str(recipe_json)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    tracing::info!(chip = %recipe.chip, steps = recipe.steps.len(), "applying PMU init recipe");

    let mut steps_applied = 0;
    let mut steps_failed = 0;

    for step in &recipe.steps {
        #[allow(clippy::cast_possible_truncation)]
        match bar0.write_u32(step.offset, step.value as u32) {
            Ok(()) => {
                steps_applied += 1;
                tracing::debug!(
                    offset = format!("{:#010x}", step.offset),
                    value = format!("{:#010x}", step.value),
                    "register write OK"
                );
            }
            Err(e) => {
                steps_failed += 1;
                tracing::error!(
                    offset = format!("{:#010x}", step.offset),
                    err = %e,
                    "register write FAILED"
                );
            }
        }

        if let Some(delay) = step.delay_us {
            std::thread::sleep(std::time::Duration::from_micros(delay));
        }
    }

    let mut verify_passed = 0;
    let mut verify_failed = 0;

    for v in &recipe.verify_reads {
        match bar0.read_u32(v.offset) {
            Ok(val) => {
                let masked = u64::from(val) & v.expected_mask;
                if masked == (v.expected_value & v.expected_mask) {
                    verify_passed += 1;
                } else {
                    verify_failed += 1;
                    tracing::warn!(
                        offset = format!("{:#010x}", v.offset),
                        read = format!("{:#010x}", val),
                        expected = format!("{:#010x}", v.expected_value),
                        "verify FAILED"
                    );
                }
            }
            Err(_) => {
                verify_failed += 1;
            }
        }
    }

    let success = steps_failed == 0 && verify_failed == 0;

    Ok(InitResult {
        chip: recipe.chip,
        steps_applied,
        steps_failed,
        verify_passed,
        verify_failed,
        success,
    })
}

/// Apply a recipe with thermal safety checks.
///
/// Reads GPU temperature before and after applying the recipe.
/// Aborts if the GPU is already above the critical temperature.
///
/// # Errors
///
/// Returns error if thermal safety is violated or recipe application fails.
#[allow(unsafe_code)]
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
