// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign boot tier model — layered abstraction of what's sovereign.
//!
//! # Tier Model
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Tier 3: Full Sovereign Cold Boot                               │
//! │   HBM2 training without VBIOS, DEVINIT from software.          │
//! │   Status: long-term research goal.                              │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ Tier 2: Warm Sovereign Compute                                  │
//! │   GPC ungating → FECS → GR dispatch → shader execution.        │
//! │   Status: BLOCKED by GPC power gating after nouveau unbind.     │
//! │   The PRI ring to the GPC domain is dead; registers at          │
//! │   0x41xxxx (GPCCS) and 0x104xxx (CE) return 0xbadfXXXX.        │
//! │   Evidence: PBDMA intr_0=0x10011111 (DEVICE error, bit 28).    │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ Tier 1: Warm Sovereign Infrastructure                           │
//! │   VFIO bind, BAR0 MMIO, DMA allocation, PRAMIN read/write,     │
//! │   PFIFO scheduling, channel creation, pushbuffer encoding,      │
//! │   QMD building, FECS liveness (PC-confirm fire-and-forget).     │
//! │   CE runlist discovered but CE engine also power-gated.         │
//! │   Status: VALIDATED — everything up to dispatch works.           │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ Tier 0: Cold Boot (Vendor Wall)                                 │
//! │   Power-on reset → Boot ROM → HBM2 training.                   │
//! │   Silicon fuse calibration, no software path exists.            │
//! │   Same wall for NVIDIA, nouveau, and sovereign code.            │
//! │   Status: ACCEPTED — power cycle is the only path.               │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Paths to Tier 2
//!
//! 1. **PMU command**: Send a power-on command through the PMU falcon
//!    (which IS alive after warm handoff) to ungate the GPC power domain.
//!    Requires reverse-engineering the PMU mailbox protocol.
//!
//! 2. **Kernel patch**: Modify nouveau to NOT power-gate GPCs during
//!    unbind. This preserves GPC state for VFIO passthrough.
//!
//! 3. **nvidia-470 handoff**: Use the proprietary nvidia-470 driver
//!    (which keeps GPCs powered) and then hand off to VFIO.
//!
//! 4. **DRM dispatch**: Keep one GPU on a kernel driver (nouveau/nvidia)
//!    and dispatch compute through DRM instead of sovereign VFIO.
//!    Works for the RTX 5060 but not Titan V (NVK requires SM75+).

use serde::{Deserialize, Serialize};

/// Sovereign boot tier — what level of sovereignty is achieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SovereignTier {
    /// Tier 0: Cold boot required. Below the hardware line.
    /// Only a power cycle can restore GPU state.
    Cold,
    /// Tier 1: Warm sovereign infrastructure.
    /// VFIO bind, BAR0, DMA, PFIFO, channels, pushbuffers all work.
    /// Engines (GR, CE) are power-gated — dispatch fails with DEVICE error.
    WarmInfrastructure,
    /// Tier 2: Warm sovereign compute.
    /// GPCs are powered, FECS dispatches methods, GR executes shaders.
    /// Full compute pipeline: upload → dispatch → sync → readback.
    WarmCompute,
    /// Tier 3: Full sovereignty.
    /// Cold boot without vendor VBIOS — HBM2 training from software.
    /// Long-term research goal.
    FullSovereign,
}

impl SovereignTier {
    /// Human-readable description of this tier.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Cold => "Cold boot — below hardware line, power cycle required",
            Self::WarmInfrastructure => "Warm infrastructure — VFIO/DMA/PFIFO functional, engines gated",
            Self::WarmCompute => "Warm compute — full shader dispatch and readback",
            Self::FullSovereign => "Full sovereign — cold boot without vendor VBIOS",
        }
    }

    /// Numeric tier level (0-3).
    #[must_use]
    pub const fn level(&self) -> u8 {
        match self {
            Self::Cold => 0,
            Self::WarmInfrastructure => 1,
            Self::WarmCompute => 2,
            Self::FullSovereign => 3,
        }
    }

    /// What works at this tier.
    #[must_use]
    pub const fn capabilities(&self) -> TierCapabilities {
        match self {
            Self::Cold => TierCapabilities {
                bar0_mmio: false,
                dma_mapping: false,
                pfifo_scheduling: false,
                channel_creation: false,
                fecs_liveness: false,
                ce_dispatch: false,
                gr_dispatch: false,
                shader_execution: false,
                cold_boot: false,
            },
            Self::WarmInfrastructure => TierCapabilities {
                bar0_mmio: true,
                dma_mapping: true,
                pfifo_scheduling: true,
                channel_creation: true,
                fecs_liveness: true,
                ce_dispatch: false,
                gr_dispatch: false,
                shader_execution: false,
                cold_boot: false,
            },
            Self::WarmCompute => TierCapabilities {
                bar0_mmio: true,
                dma_mapping: true,
                pfifo_scheduling: true,
                channel_creation: true,
                fecs_liveness: true,
                ce_dispatch: true,
                gr_dispatch: true,
                shader_execution: true,
                cold_boot: false,
            },
            Self::FullSovereign => TierCapabilities {
                bar0_mmio: true,
                dma_mapping: true,
                pfifo_scheduling: true,
                channel_creation: true,
                fecs_liveness: true,
                ce_dispatch: true,
                gr_dispatch: true,
                shader_execution: true,
                cold_boot: true,
            },
        }
    }
}

impl std::fmt::Display for SovereignTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tier {}: {}", self.level(), self.description())
    }
}

/// What's functional at a given sovereignty tier.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TierCapabilities {
    pub bar0_mmio: bool,
    pub dma_mapping: bool,
    pub pfifo_scheduling: bool,
    pub channel_creation: bool,
    pub fecs_liveness: bool,
    pub ce_dispatch: bool,
    pub gr_dispatch: bool,
    pub shader_execution: bool,
    pub cold_boot: bool,
}

/// Evidence collected during tier classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierEvidence {
    pub tier: SovereignTier,
    pub pmc_enable: u32,
    pub pmc_popcount: u32,
    pub pramin_accessible: bool,
    pub fecs_pc: Option<u32>,
    pub gpc_enables: Option<u32>,
    pub ce_status: Option<u32>,
    pub gr_status: Option<u32>,
    pub pbdma_intr: Option<u32>,
    pub ce_runlist: Option<u32>,
}

/// Classify the sovereignty tier from live register state.
///
/// Reads BAR0 registers to determine what level of sovereignty is
/// currently achievable. This is the primary classification function.
pub fn classify_tier(bar0: &crate::vfio::device::MappedBar) -> TierEvidence {
    let pmc_enable = bar0.read_u32(0x200).unwrap_or(0);
    let pmc_popcount = pmc_enable.count_ones();

    // Below 8 engines enabled = cold state
    if pmc_popcount < 8 {
        return TierEvidence {
            tier: SovereignTier::Cold,
            pmc_enable,
            pmc_popcount,
            pramin_accessible: false,
            fecs_pc: None,
            gpc_enables: None,
            ce_status: None,
            gr_status: None,
            pbdma_intr: None,
            ce_runlist: None,
        };
    }

    let pramin_accessible = crate::vfio::sovereign_stages::pramin_sentinel_test(bar0);

    // Check FECS liveness
    let fecs_pc = bar0.read_u32(0x409624).ok(); // FECS_BASE + PC

    // Check GPC power state — primary check via broadcast, fallback to per-unit
    let gpc_enables = bar0.read_u32(0x41A004).ok(); // GPC broadcast status
    let gpc_alive = {
        let bcast_alive = gpc_enables
            .map(|v| v & 0xBADF_0000 != 0xBADF_0000 && v != 0)
            .unwrap_or(false);
        if bcast_alive {
            true
        } else {
            // Fallback: probe individual GPC per-unit registers.
            // GPC_BCAST may read zero even when individual GPCs are accessible
            // (observed on Titan V after nouveau warm handoff).
            (0..6u32).any(|gpc| {
                let val = bar0.read_u32(0x500000 + gpc as usize * 0x8000).unwrap_or(0xDEAD_DEAD);
                !crate::nv::pri::is_pri_fault(val) && val != 0
            })
        }
    };

    // Check CE engine status — primary CE0, fallback to per-instance scan
    let ce_status = bar0.read_u32(0x104000).ok(); // CE0 base
    let ce_alive = {
        let ce0_alive = ce_status
            .map(|v| v & 0xBADF_0000 != 0xBADF_0000)
            .unwrap_or(false);
        if ce0_alive {
            true
        } else {
            // Fallback: scan CE1-CE5 for any alive instance
            (1..6u32).any(|i| {
                let val = bar0.read_u32(0x104000 + i as usize * 0x1000).unwrap_or(0xDEAD_DEAD);
                !crate::nv::pri::is_pri_fault(val) && val != 0
            })
        }
    };

    // Check PGRAPH status
    let gr_status = bar0.read_u32(0x400700).ok(); // PGRAPH_STATUS

    // Discover CE runlist
    let ce_runlist = crate::vfio::channel::pfifo::discover_ce_runlist(bar0);

    let tier = if gpc_alive && ce_alive {
        SovereignTier::WarmCompute
    } else if pramin_accessible {
        SovereignTier::WarmInfrastructure
    } else {
        SovereignTier::Cold
    };

    // Read first active PBDMA interrupt status for evidence
    let pbdma_map = bar0.read_u32(0x2004).unwrap_or(0);
    let first_pbdma = (0..32_u32).find(|&p| pbdma_map & (1 << p) != 0);
    let pbdma_intr = first_pbdma.and_then(|p| {
        bar0.read_u32(0x0004_0000 + (p as usize) * 0x2000 + 0x100).ok()
    });

    TierEvidence {
        tier,
        pmc_enable,
        pmc_popcount,
        pramin_accessible,
        fecs_pc,
        gpc_enables,
        ce_status,
        gr_status,
        pbdma_intr,
        ce_runlist,
    }
}

/// Classify the sovereignty tier using generation-aware register offsets.
///
/// Uses offsets from `GenerationProfile` instead of hardcoded Volta values,
/// enabling correct tier classification across Kepler, Volta, Turing, and
/// future architectures. The `classify_tier()` function is kept as a
/// convenience wrapper for the Volta default case.
pub fn classify_tier_for_profile(
    bar0: &crate::vfio::device::MappedBar,
    profile: &crate::nv::generation::GenerationProfile,
) -> TierEvidence {
    let pmc_enable = bar0.read_u32(0x200).unwrap_or(0);
    let pmc_popcount = pmc_enable.count_ones();

    if pmc_popcount < 8 {
        return TierEvidence {
            tier: SovereignTier::Cold,
            pmc_enable,
            pmc_popcount,
            pramin_accessible: false,
            fecs_pc: None,
            gpc_enables: None,
            ce_status: None,
            gr_status: None,
            pbdma_intr: None,
            ce_runlist: None,
        };
    }

    let pramin_accessible = crate::vfio::sovereign_stages::pramin_sentinel_test(bar0);

    let fecs_pc = bar0.read_u32(profile.fecs_pc_offset as usize).ok();
    let gpc_enables = bar0.read_u32(profile.gpc_broadcast_offset as usize).ok();
    let gpc_alive = {
        let bcast_alive = gpc_enables
            .map(|v| v & 0xBADF_0000 != 0xBADF_0000 && v != 0)
            .unwrap_or(false);
        if bcast_alive {
            true
        } else {
            (0..6u32).any(|gpc| {
                let val = bar0.read_u32(0x500000 + gpc as usize * 0x8000).unwrap_or(0xDEAD_DEAD);
                !crate::nv::pri::is_pri_fault(val) && val != 0
            })
        }
    };

    let ce_status = bar0.read_u32(profile.ce0_base_offset as usize).ok();
    let ce_alive = {
        let ce0_alive = ce_status
            .map(|v| v & 0xBADF_0000 != 0xBADF_0000)
            .unwrap_or(false);
        if ce0_alive {
            true
        } else {
            (1..6u32).any(|i| {
                let val = bar0.read_u32(0x104000 + i as usize * 0x1000).unwrap_or(0xDEAD_DEAD);
                !crate::nv::pri::is_pri_fault(val) && val != 0
            })
        }
    };

    let gr_status = bar0.read_u32(profile.pgraph_status_offset as usize).ok();
    let ce_runlist = crate::vfio::channel::pfifo::discover_ce_runlist(bar0);

    let tier = if gpc_alive && ce_alive {
        SovereignTier::WarmCompute
    } else if pramin_accessible {
        SovereignTier::WarmInfrastructure
    } else {
        SovereignTier::Cold
    };

    let pbdma_map = bar0.read_u32(0x2004).unwrap_or(0);
    let first_pbdma = (0..32_u32).find(|&p| pbdma_map & (1 << p) != 0);
    let pbdma_intr = first_pbdma.and_then(|p| {
        bar0.read_u32(0x0004_0000 + (p as usize) * 0x2000 + 0x100).ok()
    });

    TierEvidence {
        tier,
        pmc_enable,
        pmc_popcount,
        pramin_accessible,
        fecs_pc,
        gpc_enables,
        ce_status,
        gr_status,
        pbdma_intr,
        ce_runlist,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_ordering() {
        assert!(SovereignTier::Cold < SovereignTier::WarmInfrastructure);
        assert!(SovereignTier::WarmInfrastructure < SovereignTier::WarmCompute);
        assert!(SovereignTier::WarmCompute < SovereignTier::FullSovereign);
    }

    #[test]
    fn tier_levels() {
        assert_eq!(SovereignTier::Cold.level(), 0);
        assert_eq!(SovereignTier::WarmInfrastructure.level(), 1);
        assert_eq!(SovereignTier::WarmCompute.level(), 2);
        assert_eq!(SovereignTier::FullSovereign.level(), 3);
    }

    #[test]
    fn tier_capabilities_monotonic() {
        let t1 = SovereignTier::WarmInfrastructure.capabilities();
        assert!(t1.bar0_mmio);
        assert!(!t1.shader_execution);

        let t2 = SovereignTier::WarmCompute.capabilities();
        assert!(t2.bar0_mmio);
        assert!(t2.shader_execution);
        assert!(!t2.cold_boot);
    }

    #[test]
    fn display_format() {
        let s = format!("{}", SovereignTier::WarmInfrastructure);
        assert!(s.contains("Tier 1"));
    }
}
