// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign boot state — the unified warm/cold model.
//!
//! # The Hardware Line
//!
//! Every GPU has a boundary between what software can reach and what
//! only hardware power-on reset can achieve. On Volta+ with HBM2, the
//! boot ROM trains memory during power-on reset using silicon-specific
//! calibration data burned into fuses. **No software path exists to
//! replicate this** — not for NVIDIA's proprietary driver, not for
//! nouveau, and not for sovereign code. This is the hardware line.
//!
//! ```text
//!   ┌──────────────────────────────────────────────────┐
//!   │              HARDWARE LINE (Cold)                 │
//!   │                                                  │
//!   │  Power-On Reset → Boot ROM → HBM2 Training      │
//!   │  (fuse-calibrated, silicon-specific)             │
//!   │                                                  │
//!   │  Vendor faces the same wall. If they can boot    │
//!   │  cold, it's because the machine was power-cycled.│
//!   └────────────────────┬─────────────────────────────┘
//!                        │ GPU enters warm state
//!   ┌────────────────────▼─────────────────────────────┐
//!   │            SOVEREIGN ZONE (Warm)                  │
//!   │                                                  │
//!   │  PMC engines enabled, PRAMIN accessible,         │
//!   │  falcon firmware loadable, compute dispatchable. │
//!   │                                                  │
//!   │  Keepalive prevents transitions back to cold.    │
//!   │  Clutch engages/disengages BAR0+DMA on demand.   │
//!   └──────────────────────────────────────────────────┘
//! ```
//!
//! # Boot State Model
//!
//! [`SovereignBootState`] is the single source of truth for where a GPU
//! sits relative to the hardware line. All pipeline decisions (skip
//! memory training? run ACR before memory? attempt falcon re-boot?)
//! derive from this state.
//!
//! [`ColdBootReason`] explains *why* a GPU is cold, mapping each cause
//! to a prevention strategy:
//!
//! | Reason | Prevention |
//! |--------|------------|
//! | `PowerOnReset` | None — this IS the cold boot. Accept it. |
//! | `BusReset` | systemd fd store keeps VFIO binding alive |
//! | `D3Cold` | PCIe keepalive + power pinning |
//! | `FdLost` | Use `systemctl restart` (not stop+start) |
//! | `Unknown` | Investigate — check dmesg for reset messages |

use serde::{Deserialize, Serialize};

use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_stages::{PMC_ENABLE, pramin_sentinel_test};
use crate::vfio::sovereign_strategy::FalconWarmState;

/// Where a GPU sits relative to the hardware line.
///
/// This is the single source of truth for warm/cold classification.
/// The sovereign init pipeline, keepalive system, and dispatch handler
/// all derive their decisions from this state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SovereignBootState {
    /// GPU is above the hardware line — engines enabled, VRAM accessible,
    /// falcon firmware loadable. Full sovereignty.
    Warm {
        /// Number of bits set in PMC_ENABLE (engines enabled).
        pmc_popcount: u32,
        /// PRAMIN window is accessible and responsive.
        pramin_ok: bool,
        /// Falcon microcontroller state (FECS lifecycle).
        falcon: FalconWarmState,
    },
    /// GPU is below the hardware line — cold boot state.
    /// Memory is untrained, engines may be off. Same wall the vendor faces.
    Cold {
        /// Why the GPU is cold.
        reason: ColdBootReason,
        /// PMC_ENABLE register value (low popcount = cold).
        pmc_enable: u32,
    },
}

/// Why a GPU ended up below the hardware line.
///
/// Each reason maps to a prevention strategy (or acceptance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColdBootReason {
    /// Fresh power-on reset. Boot ROM trained HBM2 from fuse data.
    /// This is the expected cold state after a power cycle.
    /// **Prevention:** none — this IS cold boot. Accept it.
    PowerOnReset,
    /// VFIO group release triggered a Secondary Bus Reset through
    /// the upstream PCIe bridge, killing HBM2 training state.
    /// **Prevention:** systemd FileDescriptorStore keeps VFIO fds alive.
    BusReset,
    /// PCIe power management transitioned the device to D3cold.
    /// **Prevention:** PCIe keepalive + sysfs `d3cold_allowed=0` + `power/control=on`.
    D3Cold,
    /// VFIO file descriptors were lost (e.g. `systemctl stop` instead of `restart`).
    /// **Prevention:** use `systemctl restart` or upgrade to systemd 256+ with
    /// `FileDescriptorStorePreserve=yes`.
    FdLost,
    /// Cannot determine why the GPU is cold. Check `dmesg` for reset messages.
    Unknown,
}

/// What the current boot state allows the pipeline to do.
#[derive(Debug, Clone, Copy)]
pub struct BootCapability {
    /// Memory training can be skipped (HBM2/GDDR5 already trained).
    pub skip_memory_training: bool,
    /// ACR DMA boot is available (warm engines + DMA backend).
    pub acr_boot_available: bool,
    /// Falcon firmware is resident — skip re-upload.
    pub falcon_resident: bool,
    /// GR context state is preserved — skip full GR init.
    pub gr_preserved: bool,
}

impl SovereignBootState {
    /// Is this a warm state (above the hardware line)?
    #[must_use]
    pub fn is_warm(&self) -> bool {
        matches!(self, Self::Warm { .. })
    }

    /// Is this a cold state (below the hardware line)?
    #[must_use]
    pub fn is_cold(&self) -> bool {
        matches!(self, Self::Cold { .. })
    }

    /// Derive what the pipeline can do from the current state.
    #[must_use]
    pub fn capabilities(&self) -> BootCapability {
        match self {
            Self::Warm { falcon, .. } => BootCapability {
                skip_memory_training: true,
                acr_boot_available: true,
                falcon_resident: matches!(
                    falcon,
                    FalconWarmState::WarmPreserved { .. } | FalconWarmState::WarmRunning { .. }
                ),
                gr_preserved: matches!(falcon, FalconWarmState::WarmRunning { .. }),
            },
            Self::Cold { .. } => BootCapability {
                skip_memory_training: false,
                acr_boot_available: false,
                falcon_resident: false,
                gr_preserved: false,
            },
        }
    }

    /// Short human-readable summary for logging.
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::Warm {
                pmc_popcount,
                pramin_ok,
                falcon,
            } => {
                format!("warm (pmc={pmc_popcount} engines, pramin={pramin_ok}, falcon={falcon:?})")
            }
            Self::Cold { reason, pmc_enable } => {
                format!("cold ({reason:?}, pmc=0x{pmc_enable:08x})")
            }
        }
    }
}

/// Strategy-provided closure that reads falcon registers and classifies state.
pub type FalconDetector<'a> = &'a dyn Fn(&MappedBar, bool) -> FalconWarmState;

/// Probe the GPU's boot state from BAR0 registers.
///
/// This is the authoritative warm/cold classifier. It reads PMC_ENABLE,
/// tests PRAMIN accessibility, and queries falcon state to produce a
/// [`SovereignBootState`] that the pipeline uses for all decisions.
///
/// `detect_falcon` is a closure provided by the strategy to read FECS
/// registers and classify falcon state. Pass `None` to skip falcon
/// probing (the state will default to `FalconWarmState::Cold`).
pub fn probe_boot_state(
    bar0: &MappedBar,
    detect_falcon: Option<FalconDetector<'_>>,
) -> SovereignBootState {
    let pmc_read = crate::nv::register_read::RegisterRead::from_result(bar0.read_u32(PMC_ENABLE));
    let pmc_enable = pmc_read.raw().unwrap_or(0);
    let pmc_popcount = pmc_read.count_ones().unwrap_or(0);

    if pmc_popcount < 8 {
        let reason = classify_cold_reason(pmc_enable);
        return SovereignBootState::Cold { reason, pmc_enable };
    }

    let pramin_ok = pramin_sentinel_test(bar0);
    if !pramin_ok {
        return SovereignBootState::Cold {
            reason: ColdBootReason::Unknown,
            pmc_enable,
        };
    }

    let falcon = detect_falcon
        .map(|f| f(bar0, true))
        .unwrap_or(FalconWarmState::Cold);

    SovereignBootState::Warm {
        pmc_popcount,
        pramin_ok,
        falcon,
    }
}

/// Infer why the GPU is cold from the PMC_ENABLE register state.
fn classify_cold_reason(pmc_enable: u32) -> ColdBootReason {
    if pmc_enable == 0 {
        // All engines off — likely power-on reset or bus reset
        ColdBootReason::PowerOnReset
    } else if pmc_enable.count_ones() <= 2 {
        // Very few engines — D3cold recovery or partial reset
        ColdBootReason::D3Cold
    } else {
        ColdBootReason::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_state_summary() {
        let state = SovereignBootState::Cold {
            reason: ColdBootReason::PowerOnReset,
            pmc_enable: 0,
        };
        assert!(state.is_cold());
        assert!(!state.is_warm());
        assert!(state.summary().contains("cold"));
    }

    #[test]
    fn warm_state_summary() {
        let state = SovereignBootState::Warm {
            pmc_popcount: 24,
            pramin_ok: true,
            falcon: FalconWarmState::Cold,
        };
        assert!(state.is_warm());
        assert!(!state.is_cold());
        assert!(state.summary().contains("warm"));
    }

    #[test]
    fn capabilities_warm_cold_falcon() {
        let warm_cold_falcon = SovereignBootState::Warm {
            pmc_popcount: 20,
            pramin_ok: true,
            falcon: FalconWarmState::Cold,
        };
        let caps = warm_cold_falcon.capabilities();
        assert!(caps.skip_memory_training);
        assert!(caps.acr_boot_available);
        assert!(!caps.falcon_resident);
        assert!(!caps.gr_preserved);
    }

    #[test]
    fn capabilities_warm_preserved_falcon() {
        let warm_preserved = SovereignBootState::Warm {
            pmc_popcount: 20,
            pramin_ok: true,
            falcon: FalconWarmState::WarmPreserved {
                cpuctl: 0x10,
                mailbox0: 0x300,
            },
        };
        let caps = warm_preserved.capabilities();
        assert!(caps.skip_memory_training);
        assert!(caps.falcon_resident);
        assert!(!caps.gr_preserved);
    }

    #[test]
    fn capabilities_cold() {
        let cold = SovereignBootState::Cold {
            reason: ColdBootReason::BusReset,
            pmc_enable: 0,
        };
        let caps = cold.capabilities();
        assert!(!caps.skip_memory_training);
        assert!(!caps.falcon_resident);
    }

    #[test]
    fn classify_cold_zero_pmc() {
        assert_eq!(classify_cold_reason(0), ColdBootReason::PowerOnReset);
    }

    #[test]
    fn classify_cold_low_pmc() {
        assert_eq!(classify_cold_reason(0x3), ColdBootReason::D3Cold);
    }

    #[test]
    fn classify_cold_mid_pmc() {
        assert_eq!(classify_cold_reason(0xFF), ColdBootReason::Unknown);
    }

    #[test]
    fn serde_round_trip_warm() {
        let state = SovereignBootState::Warm {
            pmc_popcount: 24,
            pramin_ok: true,
            falcon: FalconWarmState::Cold,
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"state\":\"warm\""));
        let back: SovereignBootState = serde_json::from_str(&json).unwrap();
        assert!(back.is_warm());
    }

    #[test]
    fn serde_round_trip_cold() {
        let state = SovereignBootState::Cold {
            reason: ColdBootReason::BusReset,
            pmc_enable: 0,
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"state\":\"cold\""));
        let back: SovereignBootState = serde_json::from_str(&json).unwrap();
        assert!(back.is_cold());
    }
}
