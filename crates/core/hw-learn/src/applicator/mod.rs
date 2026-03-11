// SPDX-License-Identifier: AGPL-3.0-only
//! Apply learned init recipes to target GPUs.
//!
//! The applicator replays recipe steps on a target GPU and verifies
//! whether the init succeeded. Safety-first: each step is validated
//! before proceeding to the next, and the applicator can bail out
//! at any point without leaving the GPU in an undefined state.

pub mod nouveau_drm;
pub mod verify;

use crate::distiller::{InitRecipe, InitStep};
use serde::{Deserialize, Serialize};

/// Result of applying a recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    /// Recipe ID that was applied.
    pub recipe_id: String,
    /// How many steps were executed.
    pub steps_executed: usize,
    /// Total steps in the recipe.
    pub steps_total: usize,
    /// Final verdict.
    pub verdict: ApplyVerdict,
    /// Per-step results.
    pub step_results: Vec<StepResult>,
}

/// Verdict after applying a recipe.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApplyVerdict {
    /// All steps succeeded and verification passed.
    Success,
    /// Steps executed but verification failed.
    PartialSuccess,
    /// A step failed — init incomplete.
    Failed,
    /// Aborted by safety check before any writes.
    Aborted,
}

/// Result of a single init step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_index: usize,
    pub success: bool,
    pub detail: String,
}

/// Recipe applicator — applies init recipes to target GPUs.
pub struct RecipeApplicator {
    dry_run: bool,
}

impl RecipeApplicator {
    /// Create an applicator.
    ///
    /// Set `dry_run` to true to simulate without writing to hardware.
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    /// Apply a recipe to the target GPU.
    ///
    /// Returns an `ApplyResult` with per-step feedback.
    pub fn apply(
        &self,
        recipe: &InitRecipe,
        card_path: &str,
    ) -> ApplyResult {
        let mut step_results = Vec::new();
        let recipe_id = format!(
            "{}_{}_{}",
            recipe.target_arch.vendor,
            recipe.target_arch.compute_class,
            recipe.target_arch.chip
        );

        for (i, step) in recipe.steps.iter().enumerate() {
            let result = if self.dry_run {
                self.simulate_step(i, step)
            } else {
                self.execute_step(i, step, card_path)
            };

            let success = result.success;
            step_results.push(result);

            if !success {
                return ApplyResult {
                    recipe_id,
                    steps_executed: i + 1,
                    steps_total: recipe.steps.len(),
                    verdict: ApplyVerdict::Failed,
                    step_results,
                };
            }
        }

        let verdict = if step_results.iter().all(|r| r.success) {
            ApplyVerdict::Success
        } else {
            ApplyVerdict::PartialSuccess
        };

        ApplyResult {
            recipe_id,
            steps_executed: recipe.steps.len(),
            steps_total: recipe.steps.len(),
            verdict,
            step_results,
        }
    }

    fn simulate_step(&self, index: usize, step: &InitStep) -> StepResult {
        let detail = match step {
            InitStep::RegisterWrite { offset, value, function } => {
                format!("[DRY] reg write 0x{offset:08x} = 0x{value:08x} ({function:?})")
            }
            InitStep::IoctlCall { ioctl_nr, .. } => {
                format!("[DRY] ioctl 0x{ioctl_nr:x}")
            }
            InitStep::FirmwareLoad { engine, path } => {
                format!("[DRY] load {engine:?} firmware from {}", path.display())
            }
            InitStep::Delay { us } => {
                format!("[DRY] delay {us}us")
            }
            InitStep::Verify { check } => {
                format!("[DRY] verify: {check:?}")
            }
        };

        StepResult {
            step_index: index,
            success: true,
            detail,
        }
    }

    fn execute_step(&self, index: usize, step: &InitStep, card_path: &str) -> StepResult {
        match step {
            InitStep::RegisterWrite { offset, value, function } => {
                tracing::info!(offset, value, ?function, "register write");
                // Actual register writes require debugfs or mapped BAR access.
                // For now, this is a placeholder — the nouveau_drm module
                // handles ioctl-based approaches.
                StepResult {
                    step_index: index,
                    success: false,
                    detail: format!(
                        "direct register write 0x{offset:08x} not yet implemented \
                         (use ioctl path via nouveau_drm module)"
                    ),
                }
            }
            InitStep::IoctlCall { ioctl_nr, args } => {
                nouveau_drm::execute_ioctl(index, card_path, *ioctl_nr, args)
            }
            InitStep::FirmwareLoad { engine, path } => {
                StepResult {
                    step_index: index,
                    success: path.exists(),
                    detail: format!(
                        "firmware {engine:?}: {}",
                        if path.exists() { "found" } else { "MISSING" }
                    ),
                }
            }
            InitStep::Delay { us } => {
                std::thread::sleep(std::time::Duration::from_micros(*us));
                StepResult {
                    step_index: index,
                    success: true,
                    detail: format!("delayed {us}us"),
                }
            }
            InitStep::Verify { check } => {
                verify::run_verification(index, card_path, check)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distiller::*;

    #[test]
    fn dry_run_always_succeeds() {
        let recipe = InitRecipe {
            source_arch: GpuArch {
                vendor: Vendor::Nvidia,
                generation: "Ada".into(),
                chip: "AD104".into(),
                compute_class: "sm89".into(),
            },
            source_driver: DriverKind::Nouveau,
            target_arch: GpuArch {
                vendor: Vendor::Nvidia,
                generation: "Volta".into(),
                chip: "GV100".into(),
                compute_class: "sm70".into(),
            },
            steps: vec![
                InitStep::RegisterWrite {
                    offset: 0x20000,
                    value: 1,
                    function: RegFunction::PowerGate,
                },
                InitStep::Delay { us: 100 },
                InitStep::Verify {
                    check: VerifyCheck::ComputeReadback,
                },
            ],
            confidence: 0.0,
            description: "test".into(),
        };

        let applicator = RecipeApplicator::new(true);
        let result = applicator.apply(&recipe, "/dev/dri/card0");
        assert_eq!(result.verdict, ApplyVerdict::Success);
        assert_eq!(result.steps_executed, 3);
    }
}
