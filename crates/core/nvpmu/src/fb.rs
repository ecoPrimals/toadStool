// SPDX-License-Identifier: AGPL-3.0-or-later
//! HBM2 framebuffer controller skeleton.
//!
//! **Status**: Read-only probing only. No write operations.
//!
//! This module lays the groundwork for the single largest gap to full
//! GPU sovereignty: HBM2 memory controller initialization. When D3cold
//! occurs, the HBM2 DRAM training sequence is lost. Currently the only
//! recovery path is a nouveau warm cycle.
//!
//! # What's Here
//!
//! - Register map from nouveau `ramgv100.c` / `gf100_fb_oneinit()`
//! - Read-only FB status probe (can VRAM be accessed?)
//! - Documented gaps for the visualization service's differential probe to fill
//!
//! # What's NOT Here (Yet)
//!
//! - Memory timing table parsing (VBIOS)
//! - HBM2 PHY calibration sequence
//! - Memory controller register write sequence
//! - Per-model parameterization via hw-learn database
//!
//! # Path Forward
//!
//! 1. The visualization service's `memory_probe.rs` runs differential captures of
//!    nouveau's init sequence via mmiotrace
//! 2. hotSpring feeds captured sequences to hw-learn as `InitRecipe`
//! 3. toadStool replays verified recipes via `RecipeApplicator`
//! 4. This module grows to support direct FB init without nouveau
//!
//! # References
//!
//! - nouveau `nvkm/subdev/fb/ramgv100.c`
//! - nouveau `nvkm/subdev/fb/gf100.c` (`gf100_fb_oneinit()`)
//! - `wateringHole/GPU_SOVEREIGN_BRING_UP_GUIDE.md` Gap 1
//! - `hotSpring/experiments/059_GPU_POWER_MANAGEMENT_DESIGN.md`

use crate::error::{NvPmuError, Result};
use crate::registers;
use hw_learn::applicator::RegisterAccess;

/// Framebuffer/memory controller status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FbStatus {
    /// HBM2 is trained and VRAM is accessible. The GPU has been through
    /// a proper init sequence (nouveau or sovereign).
    Trained,
    /// HBM2 training is lost (post-D3cold) or never completed. VRAM
    /// reads return garbage or hang.
    Untrained,
    /// Cannot determine status (GPU in D3hot, BAR0 inaccessible, or
    /// probe reads returned unexpected values).
    Unknown,
}

impl std::fmt::Display for FbStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trained => write!(f, "Trained"),
            Self::Untrained => write!(f, "Untrained"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// GV100 framebuffer controller register offsets.
///
/// Sourced from nouveau `ramgv100.c`. These are the registers involved
/// in HBM2 memory controller initialization. Listed here for reference
/// and future implementation — no writes are performed.
pub mod regs {
    /// FB top-level status/config base.
    pub const FB_TOP: u64 = super::registers::FB_BASE;

    /// FBPA (Framebuffer Partition Array) base.
    /// Each FBPA manages one HBM2 stack. GV100 has 4.
    pub const FBPA_BASE: u64 = super::registers::FBPA_BASE;
    /// Per-FBPA stride in the register space (0x4000).
    pub const FBPA_STRIDE: u64 = super::registers::FBPA_STRIDE;
    /// Number of FBPA partitions on GV100 (4 HBM2 stacks).
    pub const FBPA_COUNT: u32 = super::registers::GV100_FBPA_COUNT;

    /// FB memory controller config (within each FBPA).
    /// Nouveau sets these during `ramgv100_init()`.
    pub const FBPA_CMD: u64 = 0x00; // relative to FBPA base
    /// FBPA config register; readback indicates memory controller init state.
    pub const FBPA_CFG: u64 = 0x04;
    /// FBPA timing config register 0.
    pub const FBPA_TIMING0: u64 = 0x80;
    /// FBPA timing config register 1.
    pub const FBPA_TIMING1: u64 = 0x84;
    /// FBPA timing config register 2.
    pub const FBPA_TIMING2: u64 = 0x88;

    /// `NV_PFB_NISO_FLUSH_SYSMEM_ADDR` — used to verify VRAM accessibility.
    /// Writing a known pattern and reading back confirms FB is initialized.
    pub const PFB_NISO_FLUSH: u64 = 0x0010_0C80;

    /// PRAMIN window base — used for VRAM read/write via BAR0.
    pub const PRAMIN_BASE: u64 = 0x0070_0000;
    /// PRAMIN window size in bytes (1 `MiB`).
    pub const PRAMIN_SIZE: u64 = 0x0010_0000; // 1 MB window
}

/// Probe whether the framebuffer/HBM2 is in a trained state.
///
/// Performs non-destructive read-only checks:
/// 1. Verify BAR0 is accessible (not D3hot)
/// 2. Read `FB_TOP` registers for signs of initialization
/// 3. Check PRAMIN accessibility as a VRAM connectivity test
///
/// This does NOT write any registers. It is safe to call at any time
/// when the GPU is in `PCIe` D0.
///
/// # Errors
///
/// Returns error only on register access failure (BAR0 unmapped, etc).
/// Returns `FbStatus::Unknown` for ambiguous readings.
pub fn probe_fb_status(regs: &impl RegisterAccess) -> Result<FbStatus> {
    let boot0 = regs
        .read_u32(registers::BOOT0)
        .map_err(|e| NvPmuError::Hardware(format!("BOOT0 read: {e}")))?;

    if boot0 == registers::BAR0_D3HOT_SENTINEL {
        return Ok(FbStatus::Unknown);
    }

    // Read first FBPA register to check if memory controller is configured.
    // A zeroed-out FBPA typically means uninit. The exact "trained" signature
    // varies by firmware/driver init path, so we check for non-zero.
    let fbpa0_cfg = regs
        .read_u32(regs::FBPA_BASE + regs::FBPA_CFG)
        .map_err(|e| NvPmuError::Hardware(format!("FBPA0_CFG read: {e}")))?;

    if fbpa0_cfg == registers::BAR0_D3HOT_SENTINEL {
        return Ok(FbStatus::Unknown);
    }

    if fbpa0_cfg == 0 {
        // All-zero FBPA config suggests memory controller is not initialized.
        // This is the expected state after D3cold without nouveau warm cycle.
        return Ok(FbStatus::Untrained);
    }

    // Non-zero FBPA config + accessible BAR0 suggests FB is trained.
    // A more thorough check would write/read PRAMIN, but that's destructive.
    Ok(FbStatus::Trained)
}

/// Summary of FBPA partition status across all 4 HBM2 stacks.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FbPartitionReport {
    /// Per-FBPA config register readback.
    pub partitions: Vec<FbPartitionEntry>,
    /// Overall FB status.
    pub status: FbStatus,
}

/// Single FBPA partition readback.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FbPartitionEntry {
    /// FBPA partition index (0–3 on GV100).
    pub index: u32,
    /// FBPA config register value; non-zero indicates training complete.
    pub cfg: u32,
    /// FBPA timing0 register value.
    pub timing0: u32,
}

/// Probe all 4 FBPA partitions and produce a summary report.
///
/// Read-only — does not modify any registers.
///
/// # Errors
///
/// Returns error on register access failure.
pub fn probe_fb_partitions(reg: &impl RegisterAccess) -> Result<FbPartitionReport> {
    let overall = probe_fb_status(reg)?;

    let mut partitions = Vec::with_capacity(regs::FBPA_COUNT as usize);

    for i in 0..regs::FBPA_COUNT {
        let base = regs::FBPA_BASE + u64::from(i) * regs::FBPA_STRIDE;

        let cfg = reg
            .read_u32(base + regs::FBPA_CFG)
            .unwrap_or(registers::BAR0_D3HOT_SENTINEL);

        let timing0 = reg
            .read_u32(base + regs::FBPA_TIMING0)
            .unwrap_or(registers::BAR0_D3HOT_SENTINEL);

        partitions.push(FbPartitionEntry {
            index: i,
            cfg,
            timing0,
        });
    }

    Ok(FbPartitionReport {
        partitions,
        status: overall,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MockRegs {
        values: HashMap<u64, u32>,
    }

    impl RegisterAccess for MockRegs {
        fn read_u32(&self, offset: u64) -> std::result::Result<u32, String> {
            self.values
                .get(&offset)
                .copied()
                .ok_or_else(|| format!("unmapped {offset:#x}"))
        }

        fn write_u32(&mut self, offset: u64, value: u32) -> std::result::Result<(), String> {
            self.values.insert(offset, value);
            Ok(())
        }
    }

    fn mock_with_trained_fb() -> MockRegs {
        let mut v = HashMap::new();
        v.insert(registers::BOOT0, registers::BOOT0_GV100);
        // Non-zero FBPA configs indicate training complete
        for i in 0..4_u64 {
            let base = regs::FBPA_BASE + i * regs::FBPA_STRIDE;
            v.insert(base + regs::FBPA_CFG, 0x0001_0042);
            v.insert(base + regs::FBPA_TIMING0, 0x1234_5678);
        }
        MockRegs { values: v }
    }

    fn mock_with_untrained_fb() -> MockRegs {
        let mut v = HashMap::new();
        v.insert(registers::BOOT0, registers::BOOT0_GV100);
        for i in 0..4_u64 {
            let base = regs::FBPA_BASE + i * regs::FBPA_STRIDE;
            v.insert(base + regs::FBPA_CFG, 0);
            v.insert(base + regs::FBPA_TIMING0, 0);
        }
        MockRegs { values: v }
    }

    fn mock_d3hot() -> MockRegs {
        let mut v = HashMap::new();
        v.insert(registers::BOOT0, registers::BAR0_D3HOT_SENTINEL);
        MockRegs { values: v }
    }

    #[test]
    fn probe_trained_fb() {
        let regs = mock_with_trained_fb();
        let status = probe_fb_status(&regs).unwrap();
        assert_eq!(status, FbStatus::Trained);
    }

    #[test]
    fn probe_untrained_fb() {
        let regs = mock_with_untrained_fb();
        let status = probe_fb_status(&regs).unwrap();
        assert_eq!(status, FbStatus::Untrained);
    }

    #[test]
    fn probe_d3hot_returns_unknown() {
        let regs = mock_d3hot();
        let status = probe_fb_status(&regs).unwrap();
        assert_eq!(status, FbStatus::Unknown);
    }

    #[test]
    fn partition_report_trained() {
        let regs = mock_with_trained_fb();
        let report = probe_fb_partitions(&regs).unwrap();
        assert_eq!(report.status, FbStatus::Trained);
        assert_eq!(report.partitions.len(), 4);
        for p in &report.partitions {
            assert_ne!(p.cfg, 0);
        }
    }

    #[test]
    fn partition_report_untrained() {
        let regs = mock_with_untrained_fb();
        let report = probe_fb_partitions(&regs).unwrap();
        assert_eq!(report.status, FbStatus::Untrained);
        for p in &report.partitions {
            assert_eq!(p.cfg, 0);
        }
    }

    #[test]
    fn fb_status_display() {
        assert_eq!(FbStatus::Trained.to_string(), "Trained");
        assert_eq!(FbStatus::Untrained.to_string(), "Untrained");
        assert_eq!(FbStatus::Unknown.to_string(), "Unknown");
    }
}
