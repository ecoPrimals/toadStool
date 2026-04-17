// SPDX-License-Identifier: AGPL-3.0-or-later
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

/// Abstraction over GPU register I/O.
///
/// Implementations provide direct register read/write — e.g.
/// `nvpmu::Bar0Access` maps BAR0 via sysfs for MMIO, test mocks
/// return canned values. The applicator uses this for `RegisterWrite`
/// steps and `Verify::RegisterMatch` checks.
pub trait RegisterAccess {
    /// Read a 32-bit register at the given BAR-relative offset.
    ///
    /// # Errors
    /// Returns an error string if the read fails (e.g. out of bounds).
    fn read_u32(&self, offset: u64) -> Result<u32, String>;

    /// Write a 32-bit register at the given BAR-relative offset.
    ///
    /// # Errors
    /// Returns an error string if the write fails.
    fn write_u32(&mut self, offset: u64, value: u32) -> Result<(), String>;
}

/// Placeholder `RegisterAccess` for applicators constructed without BAR0 MMIO.
#[derive(Debug, Default)]
pub struct NoRegisterAccess;

impl RegisterAccess for NoRegisterAccess {
    fn read_u32(&self, _offset: u64) -> Result<u32, String> {
        Err("no register access".into())
    }

    fn write_u32(&mut self, _offset: u64, _value: u32) -> Result<(), String> {
        Err("no register access".into())
    }
}

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
    /// Index of the step in the recipe.
    pub step_index: usize,
    /// Whether the step succeeded.
    pub success: bool,
    /// Human-readable result or error message.
    pub detail: String,
}

/// Recipe applicator — applies init recipes to target GPUs.
///
/// Supports two modes:
/// - **DRM-only** (`new(dry_run)`) — uses ioctl path via `nouveau_drm`.
/// - **BAR0-backed** (`with_register_access`) — uses a `RegisterAccess`
///   implementation (e.g. `nvpmu::Bar0Access`) for direct register writes
///   and verification.
/// - Optional **GPU readback** (`with_gpu_readback`) — wires [`verify::GpuReadbackAccess`]
///   for [`VerifyCheck::ComputeReadback`] when a scratch / VRAM read path exists.
pub struct RecipeApplicator<'a, R: RegisterAccess = NoRegisterAccess> {
    dry_run: bool,
    register_access: Option<&'a mut R>,
    gpu_readback: Option<&'a dyn verify::GpuReadbackAccess>,
}

impl<'a> RecipeApplicator<'a, NoRegisterAccess> {
    /// Create an applicator without BAR0 access.
    ///
    /// Register writes will be routed through the ioctl path.
    /// Set `dry_run` to true to simulate without writing to hardware.
    #[must_use]
    pub fn new(dry_run: bool) -> Self {
        Self {
            dry_run,
            register_access: None,
            gpu_readback: None,
        }
    }

    /// Attach a `RegisterAccess` implementation for direct register I/O.
    ///
    /// When attached, `RegisterWrite` steps use BAR0 MMIO directly and
    /// `Verify::RegisterMatch` uses typed verification via [`verify::run_verification`].
    #[must_use]
    pub fn with_register_access<A: RegisterAccess>(
        self,
        access: &'a mut A,
    ) -> RecipeApplicator<'a, A> {
        RecipeApplicator {
            dry_run: self.dry_run,
            register_access: Some(access),
            gpu_readback: self.gpu_readback,
        }
    }
}

impl<'a, R: RegisterAccess> RecipeApplicator<'a, R> {
    /// Attach an optional GPU readback implementation for compute / VRAM verification.
    #[must_use]
    pub fn with_gpu_readback(mut self, gpu: &'a dyn verify::GpuReadbackAccess) -> Self {
        self.gpu_readback = Some(gpu);
        self
    }
}

impl<R: RegisterAccess> RecipeApplicator<'_, R> {
    /// Apply a recipe to the target GPU.
    ///
    /// Returns an `ApplyResult` with per-step feedback.
    pub fn apply(&mut self, recipe: &InitRecipe, card_path: &str) -> ApplyResult {
        let mut step_results = Vec::new();
        let recipe_id = format!(
            "{}_{}_{}",
            recipe.target_arch.vendor, recipe.target_arch.compute_class, recipe.target_arch.chip
        );

        for (i, step) in recipe.steps.iter().enumerate() {
            let result = if self.dry_run {
                Self::simulate_step(i, step)
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

    fn simulate_step(index: usize, step: &InitStep) -> StepResult {
        let detail = match step {
            InitStep::RegisterWrite {
                offset,
                value,
                function,
            } => {
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

    #[expect(
        clippy::cast_possible_truncation,
        reason = "u64→u32 for 32-bit register values; init recipes use 32-bit writes"
    )]
    fn execute_step(&mut self, index: usize, step: &InitStep, card_path: &str) -> StepResult {
        match step {
            InitStep::RegisterWrite {
                offset,
                value,
                function,
            } => {
                tracing::info!(offset, value, ?function, "register write");
                self.register_access.as_mut().map_or_else(
                    || StepResult {
                        step_index: index,
                        success: false,
                        detail: format!(
                            "register write 0x{offset:08x}: no RegisterAccess attached \
                             (attach Bar0Access via with_register_access())"
                        ),
                    },
                    |access| match access.write_u32(*offset, *value as u32) {
                        Ok(()) => StepResult {
                            step_index: index,
                            success: true,
                            detail: format!(
                                "BAR0 write 0x{offset:08x} = 0x{value:08x} ({function:?})"
                            ),
                        },
                        Err(e) => StepResult {
                            step_index: index,
                            success: false,
                            detail: format!("BAR0 write 0x{offset:08x} failed: {e}"),
                        },
                    },
                )
            }
            InitStep::IoctlCall { ioctl_nr, args } => {
                nouveau_drm::execute_ioctl(index, card_path, *ioctl_nr, args)
            }
            InitStep::FirmwareLoad { engine, path } => StepResult {
                step_index: index,
                success: path.exists(),
                detail: format!(
                    "firmware {engine:?}: {}",
                    if path.exists() { "found" } else { "MISSING" }
                ),
            },
            InitStep::Delay { us } => {
                std::thread::sleep(std::time::Duration::from_micros(*us));
                StepResult {
                    step_index: index,
                    success: true,
                    detail: format!("delayed {us}us"),
                }
            }
            InitStep::Verify { check } => verify::run_verification(
                index,
                card_path,
                check,
                self.register_access
                    .as_mut()
                    .map(|access| &mut **access as &mut dyn RegisterAccess),
                self.gpu_readback,
            ),
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

        let mut applicator = RecipeApplicator::new(true);
        let result = applicator.apply(&recipe, "/dev/dri/card0");
        assert_eq!(result.verdict, ApplyVerdict::Success);
        assert_eq!(result.steps_executed, 3);
    }
}
