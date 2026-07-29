// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public types for the sovereign GPU initialization pipeline.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::vfio::boot_state::SovereignBootState;

/// Which stage to halt before (for debugging partial pipelines).
///
/// Stages execute in this order:
/// `PmcEnable → CgSweep → PgobUngate → MemoryTraining → EngineUngate → FalconBoot → GrInit → Verify`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HaltBefore {
    /// Halt before master clock/engine enable.
    PmcEnable,
    /// Halt before clock-gating sweep (observe raw post-PMC state).
    CgSweep,
    /// Halt before PGOB ungating (after CG sweep + PRI recovery).
    PgobUngate,
    /// Halt before memory controller bring-up (GDDR5 devinit, HBM2 training, etc.).
    MemoryTraining,
    /// Halt before engine ungating (init sequence replay).
    EngineUngate,
    /// Halt before microcontroller firmware boot (falcon, etc.).
    FalconBoot,
    /// Halt before GR engine register programming.
    GrInit,
    /// Halt before final memory/timer verification.
    Verify,
}

/// Options controlling the sovereign init pipeline.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SovereignInitOptions {
    /// Halt the pipeline before this stage (for experiments).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halt_before: Option<HaltBefore>,
    /// Golden register captures for differential HBM2 replay.
    #[serde(skip)]
    pub golden_state: Option<Vec<(usize, u32)>>,
    /// File path to a JSON golden-state capture (loaded by the RPC handler).
    /// Format: array of `[offset, value]` pairs, or a `TrainingRecipe` JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub golden_state_path: Option<String>,
    /// Explicit VBIOS ROM bytes (otherwise read from PROM/sysfs).
    #[serde(skip)]
    pub vbios_rom: Option<Vec<u8>>,
    /// File path to a raw VBIOS ROM dump (loaded by the RPC handler).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vbios_rom_path: Option<String>,
    /// Number of FBPA partitions (auto-detected if None).
    pub fbpa_count: Option<usize>,
    /// SM version for GR init (70 = GV100, 75 = TU102, etc.).
    pub sm_version: Option<u32>,
    /// Skip GR init even if falcon boot succeeds.
    #[serde(default)]
    pub skip_gr_init: bool,
    /// Skip doomed memory_training on cold GPUs (HBM2 requires power-on reset).
    /// When true and boot_state_probe returns Cold, the pipeline returns early
    /// after the probe stage with `compute_ready: false`.
    #[serde(default)]
    pub skip_cold_memory_training: bool,
    /// DMA backend for system-memory ACR boot (IOMMU-mapped buffers).
    /// When provided, the ACR boot solver can use strategies that place
    /// the WPR in system memory rather than VRAM-only paths.
    #[serde(skip)]
    pub dma_backend: Option<crate::vfio::device::DmaBackend>,
    /// Captured GR init sequence for Kepler PGRAPH ungating.
    /// When provided, the pipeline replays this sequence to ungate PGRAPH
    /// before falcon boot on NoAcr (Kepler) GPUs.
    ///
    /// Prefer `engine_init_sequences` for new code; this field is checked
    /// as a fallback for backward compatibility.
    #[serde(skip)]
    pub kepler_gr_init: Option<crate::nv::gr_init::GrInitSequence>,
    /// Per-engine init sequences for generalized ungating.
    ///
    /// Each entry is `(engine_name, sequence, optional_status_register)`.
    /// `engine_name` is used for logging (e.g. "PGRAPH", "CE", "NVDEC").
    /// `status_register` is a BAR0 offset to validate after replay —
    /// if it returns a PRI fault, the ungate is considered failed.
    #[serde(skip)]
    pub engine_init_sequences: Vec<(String, crate::nv::gr_init::GrInitSequence, Option<usize>)>,

    /// File path to a `GrInitSequence` JSON file for engine ungating.
    ///
    /// The silicon-deistic replay path: capture what a vendor driver
    /// initializes via `WarmStateCapture`, save the `GrInitSequence` as
    /// JSON, then replay it on every subsequent sovereign boot without
    /// the vendor driver. Loaded by the RPC handler into
    /// `engine_init_sequences`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_init_path: Option<String>,
}

/// Outcome of a single pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    /// Stage identifier (e.g. `"bar0_probe"`, `"memory_training"`).
    pub name: String,
    /// Whether the stage passed, was skipped, or failed.
    pub status: StageStatus,
    /// Human-readable detail about the stage outcome.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

/// Status of a sovereign init stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    /// Stage completed successfully.
    Ok,
    /// Stage was not needed or halted by request.
    Skipped,
    /// Stage failed (see `StageResult::detail`).
    Failed,
}

/// Full result of the sovereign init pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignInitResult {
    /// PCI BDF address of the device.
    pub bdf: String,
    /// Decoded device identity (e.g. 0x140 for GV100 from BOOT0).
    #[serde(alias = "chip_id")]
    pub identity_chip: u32,
    /// Raw identity register value (BOOT0 for NVIDIA, GRBM for AMD).
    #[serde(alias = "boot0")]
    pub identity_raw: u32,
    /// True if every executed stage passed.
    pub all_ok: bool,
    /// True if the full init pipeline completed without errors.
    ///
    /// **This is an init health check, NOT a compute dispatch readiness signal.**
    /// The pipeline verifies PTIMER, PRAMIN sentinel, and PMC_ENABLE readback,
    /// but does NOT check TPC PRI stations (`0x504000`) or whether shaders can
    /// actually dispatch. On VFIO Titan V, `compute_ready=true` with
    /// `classify_tier()` returning Tier 1 (WarmInfrastructure) is expected —
    /// TPC stations require GPCCS firmware execution which is HS fuse-locked.
    pub compute_ready: bool,
    /// Stage name at which the pipeline was halted (by request or failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halted_at: Option<String>,
    /// Per-stage results in execution order.
    pub stages: Vec<StageResult>,
    /// Total pipeline wall-clock time in milliseconds.
    pub total_ms: u64,
    /// Number of memory training register writes (if training ran).
    #[serde(alias = "hbm2_writes", skip_serializing_if = "Option::is_none")]
    pub training_writes: Option<usize>,
    /// Whether the GPU was detected as warm (training skipped/reduced).
    #[serde(default)]
    pub warm_detected: bool,
    /// Unified boot state classification.
    /// `None` for pipelines that ran before the boot state abstraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_state: Option<SovereignBootState>,
}

impl fmt::Display for SovereignInitResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.compute_ready {
            "COMPUTE_READY"
        } else if let Some(h) = &self.halted_at {
            return write!(f, "HALTED@{h} ({}ms)", self.total_ms);
        } else {
            "INCOMPLETE"
        };
        write!(
            f,
            "{status} chip=0x{:03x} stages={}/{} ({}ms)",
            self.identity_chip,
            self.stages
                .iter()
                .filter(|s| s.status == StageStatus::Ok)
                .count(),
            self.stages.len(),
            self.total_ms,
        )
    }
}
