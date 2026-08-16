// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign boot strategy abstraction.
//!
//! Vendor and generation are external dependencies the pipeline consumes,
//! not intrinsic structure. [`SovereignStrategy`] encodes the per-phase
//! decisions that vary across hardware families, allowing `sovereign_init`
//! to be pure orchestration.
//!
//! # Kepler vs Volta (the founding divergence)
//!
//! | Concern | Kepler | Volta+ |
//! |---------|--------|--------|
//! | CG sweep before memory | no | yes |
//! | PGOB timing | inline during falcon boot | before memory training |
//! | Memory training | GDDR5 VBIOS devinit | HBM2 controller |
//! | PMC enable | conservative mask, rollback | full mask, no rollback |
//! | Falcon boot | PIO upload | ACR DMA HS chain |
//! | GR init after falcon | no (PIO covers it) | yes (FECS re-bootstrap) |
//! | Engine ungate sequences | yes (PGRAPH replay) | no |

use std::sync::Arc;

use crate::error::SovereignStagesError;
use crate::nv::generation::{BootStrategy, GenerationProfile, PowerSafetyProfile};
use crate::nv::gsp_bridge::GspBridge;
use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_stages::MemoryTrainingStrategy;
use crate::vfio::sovereign_types::StageResult;

/// How the falcon microcontrollers are brought up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FalconBootStyle {
    /// Direct PIO firmware upload (Kepler — no ACR/WPR).
    DirectPio,
    /// SEC2 DMA -> ACR chain -> FECS release (Volta+).
    AcrDmaHs,
    /// No falcon engines on this hardware family.
    NoFalcons,
}

/// Detected thermal state of falcon microcontrollers.
///
/// On warm GPUs (after a driver handoff), FECS may be in various states
/// depending on how the previous driver tore down. This enum replaces
/// inline BAR0 register checks in `falcon_boot()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FalconWarmState {
    /// Cold GPU — no prior driver session, falcons need full boot.
    Cold,
    /// FECS was frozen by livepatch before teardown (HALTED + MAILBOX0 != 0).
    /// Firmware is resident in IMEM/DMEM — skip all boot.
    WarmPreserved {
        /// FECS CPUCTL register value.
        cpuctl: u32,
        /// FECS MAILBOX0 register value.
        mailbox0: u32,
    },
    /// FECS is actively running its polling loop (no HALTED, no HRESET,
    /// PC advancing). Skip all boot.
    WarmRunning {
        /// FECS CPUCTL register value.
        cpuctl: u32,
        /// FECS program counter.
        pc: u32,
        /// FECS MAILBOX0 register value.
        mailbox0: u32,
    },
    /// Inconsistent teardown state: CPUCTL = STARTCPU | HRESET (0x12).
    /// Firmware may be in IMEM — try PIO re-bootstrap before cold path.
    Inconsistent {
        /// FECS CPUCTL register value (typically 0x12).
        cpuctl: u32,
    },
}

/// Identity extracted from a device probe.
#[derive(Debug, Clone, Copy)]
pub struct ProbeIdentity {
    /// Raw identity register value (BOOT0 for NVIDIA, GRBM_STATUS for AMD).
    pub identity_raw: u32,
    /// Decoded chip/device identifier.
    pub identity_chip: u32,
}

/// Vendor/generation-agnostic strategy for the sovereign boot pipeline.
///
/// Each method answers a question the pipeline asks at a branch point.
/// Implementations carry the per-family/generation data needed to answer.
pub trait SovereignStrategy: Send + Sync {
    /// Human-readable name for logging (e.g. "Kepler", "Volta", "Vega 20").
    fn family_name(&self) -> &str;

    /// PMC_ENABLE sequencing profile (mask, rollback, post-devinit behaviour).
    fn power_profile(&self) -> &PowerSafetyProfile;

    /// Whether CG sweep + PRI recovery should run before memory training.
    fn needs_cg_sweep(&self) -> bool;

    /// Whether PGOB ungating should run before memory training.
    /// On Kepler, PGOB runs inside `kepler_falcon_boot` instead.
    fn needs_pgob_before_memory(&self) -> bool;

    /// Memory training strategy (GDDR5 devinit, HBM2 controller, etc.).
    fn memory_strategy(&self) -> MemoryTrainingStrategy;

    /// How falcon microcontrollers are brought up.
    fn falcon_boot_style(&self) -> FalconBootStyle;

    /// Whether falcon boot (ACR/DMA) must run before memory training.
    ///
    /// On Volta+ with HBM2 and secure boot, the PMU falcon must be loaded
    /// via the ACR chain before it can run the signed devinit firmware that
    /// trains HBM2.  Without this reorder, cold boot fails: memory_training
    /// runs first, can't train HBM2 without the PMU, and returns early
    /// before falcon_boot ever executes.
    fn needs_falcon_before_memory(&self) -> bool {
        false
    }

    /// Whether GR init (FECS re-bootstrap) should run after falcon boot.
    /// False for NoAcr (Kepler) where PIO covers it.
    fn needs_gr_init_after_falcon(&self) -> bool;

    /// Engine ungate sequences to replay before falcon boot (Kepler PGRAPH).
    /// Returns `None` if this generation doesn't need pre-falcon ungating.
    fn engine_ungate_sequences(
        &self,
    ) -> Option<&[(String, crate::nv::gr_init::GrInitSequence, Option<usize>)]>;

    /// SM version for firmware/GR init lookups.
    fn sm_version(&self) -> u32;

    /// Firmware chip codename for bridge lookups (e.g. "gv100", "gk210").
    fn firmware_chip(&self) -> &str;

    /// The firmware bridge (GspBridge) for this strategy.
    fn bridge(&self) -> &dyn GspBridge;

    /// Probe device identity from BAR0 registers.
    ///
    /// Default: reads NVIDIA BOOT0 at offset 0. Override for AMD
    /// (GRBM_STATUS), NPU, or other register layouts.
    fn probe_identity(&self, bar0: &MappedBar) -> Result<ProbeIdentity, SovereignStagesError> {
        crate::vfio::sovereign_stages::bar0_probe(bar0).map(|(raw, chip)| ProbeIdentity {
            identity_raw: raw,
            identity_chip: chip,
        })
    }

    /// Verify device health after all boot stages complete.
    ///
    /// Default: NVIDIA PTIMER + PMC_ENABLE + PRAMIN sentinel check.
    /// Override for AMD (GRBM idle), NPU, or other verification schemes.
    fn verify_device(&self, bar0: &MappedBar) -> Result<String, SovereignStagesError> {
        crate::vfio::sovereign_stages::verify(bar0)
    }

    /// Pre-channel initialization: runs on raw BAR0 before any PFIFO
    /// channel or factory device creation.
    ///
    /// Strategies that need CG sweep before channel creation (Volta+)
    /// implement this to ungate clock domains that would PRI-fault
    /// during channel setup. Returns stage results for logging.
    ///
    /// Default: no-op (empty stages).
    fn pre_channel_init(&self, _bar0: &MappedBar) -> Vec<StageResult> {
        Vec::new()
    }

    /// Select PFIFO configuration based on GPU thermal state.
    ///
    /// Warm GPUs need gentler initialization (preserve FECS state,
    /// skip empty runlist flush, skip PBDMA force-clear). Cold GPUs
    /// use aggressive fault clearing and full glow plug.
    ///
    /// Default: dispatches on `warm` flag — `warm_handoff()` vs `default()`.
    /// Strategies may override for finer control (e.g. `warm_fecs_alive()`
    /// when FECS preservation is confirmed).
    fn pfifo_config(
        &self,
        warm: bool,
        falcon_state: &FalconWarmState,
    ) -> crate::vfio::channel::PfifoInitConfig {
        use crate::vfio::channel::PfifoInitConfig;

        if !warm {
            return PfifoInitConfig::default();
        }
        match falcon_state {
            FalconWarmState::WarmPreserved { .. } | FalconWarmState::WarmRunning { .. } => {
                PfifoInitConfig::warm_fecs_alive()
            }
            _ => PfifoInitConfig::warm_handoff(),
        }
    }

    /// Detect falcon warm-state from BAR0 registers.
    ///
    /// Reads FECS CPUCTL, MAILBOX0, and optionally PC to classify the
    /// current falcon thermal state. This replaces inline register checks
    /// in `falcon_boot()`, allowing per-family detection logic.
    ///
    /// Default: NVIDIA FECS register-based detection (Volta+ ACR path).
    /// Kepler overrides to always return `Cold` (PIO path doesn't need
    /// warm detection). Future AMD/NPU strategies return `Cold` as well.
    fn detect_falcon_warm_state(&self, bar0: &MappedBar, warm_detected: bool) -> FalconWarmState {
        if !warm_detected {
            return FalconWarmState::Cold;
        }
        use crate::vfio::channel::registers::falcon;

        let cpuctl = bar0
            .read_u32(falcon::FECS_BASE + falcon::CPUCTL)
            .unwrap_or(0xDEAD_DEAD);
        let mailbox0 = bar0
            .read_u32(falcon::FECS_BASE + falcon::MAILBOX0)
            .unwrap_or(0);

        // PRI fault: 0xBADFxxxx means the PRI ring path to FECS is down
        // (PGRAPH gated, GPC not powered). Not real register contents.
        let is_pri_fault = |v: u32| v & 0xFFFF_0000 == 0xBADF_0000;
        if is_pri_fault(cpuctl) || is_pri_fault(mailbox0) {
            tracing::info!(
                cpuctl = format!("{cpuctl:#010x}"),
                mailbox0 = format!("{mailbox0:#010x}"),
                "detect_falcon_warm_state: PRI faulted (0xBADFxxxx) — returning Cold"
            );
            return FalconWarmState::Cold;
        }

        let halted = cpuctl & falcon::CPUCTL_HALTED != 0;
        let in_hreset = cpuctl & falcon::CPUCTL_HRESET != 0;
        let is_0x12 = cpuctl == (falcon::CPUCTL_STARTCPU | falcon::CPUCTL_HRESET);

        let pc = bar0.read_u32(falcon::FECS_BASE + falcon::PC).unwrap_or(0);
        // ACR-loaded firmware PCs are typically 0x80+ (firmware code section).
        // Post-FLR residual PCs are 0x00-0x10 (boot ROM artifacts).
        // Require PC >= 0x40 to avoid false positives from FLR residuals.
        let pc_valid = pc >= 0x40 && pc != 0xDEAD_DEAD && (pc & 0xBADF_0000) != 0xBADF_0000;

        if halted && !in_hreset {
            FalconWarmState::WarmPreserved { cpuctl, mailbox0 }
        } else if in_hreset && pc_valid {
            // HS ACR falcon: CPUCTL shows HRESET (0x10) but PC is advancing
            // and firmware is executing. This is the normal post-ACR-boot state
            // where the HS-secure falcon's true status isn't reflected in CPUCTL.
            FalconWarmState::WarmRunning {
                cpuctl,
                pc,
                mailbox0,
            }
        } else if !halted && !in_hreset && pc_valid {
            FalconWarmState::WarmRunning {
                cpuctl,
                pc,
                mailbox0,
            }
        } else if is_0x12 {
            FalconWarmState::Inconsistent { cpuctl }
        } else {
            FalconWarmState::Cold
        }
    }
}

// ── NVIDIA Kepler Strategy ──────────────────────────────────────────────

/// Pre-firmware NVIDIA (Kepler): conservative PMC, PIO falcon, GDDR5 devinit.
pub struct NvKeplerStrategy {
    profile: GenerationProfile,
    gsp_bridge: Arc<dyn GspBridge>,
    sm: u32,
    golden_sequences: Vec<(String, crate::nv::gr_init::GrInitSequence, Option<usize>)>,
}

impl NvKeplerStrategy {
    /// Create from a Kepler generation profile + firmware bridge.
    pub fn new(profile: GenerationProfile, bridge: Arc<dyn GspBridge>, sm: u32) -> Self {
        Self {
            profile,
            gsp_bridge: bridge,
            sm,
            golden_sequences: Vec::new(),
        }
    }

    /// Attach golden-state engine init sequences for silicon-deistic replay.
    pub fn with_golden_sequences(
        mut self,
        seqs: Vec<(String, crate::nv::gr_init::GrInitSequence, Option<usize>)>,
    ) -> Self {
        self.golden_sequences = seqs;
        self
    }
}

impl SovereignStrategy for NvKeplerStrategy {
    fn family_name(&self) -> &str {
        self.profile.name
    }

    fn power_profile(&self) -> &PowerSafetyProfile {
        &self.profile.power_safety
    }

    /// Kepler needs this precisely because its PBUS ring comes up faulted.
    ///
    /// A cold K80 answers 0xbad0011f across PBUS, which puts the on-die VBIOS
    /// at PROM out of reach — and PROM is the only VBIOS source on a Tesla
    /// card, since it exposes no PCI expansion ROM BAR. Without devinit there
    /// is no memory training, so the PRI fault is the first domino.
    ///
    /// This was `false`, so `pri_bus_recover` never ran on Kepler at all and
    /// the ring was simply left faulted. It is safe to run only now that the
    /// sweep and the recovery check for fault patterns before writing; until
    /// then both would have written into the dead ring.
    fn needs_cg_sweep(&self) -> bool {
        true
    }

    fn needs_pgob_before_memory(&self) -> bool {
        false
    }

    fn memory_strategy(&self) -> MemoryTrainingStrategy {
        MemoryTrainingStrategy::for_memory_type(self.profile.memory_type)
    }

    fn falcon_boot_style(&self) -> FalconBootStyle {
        FalconBootStyle::DirectPio
    }

    fn needs_gr_init_after_falcon(&self) -> bool {
        false
    }

    fn engine_ungate_sequences(
        &self,
    ) -> Option<&[(String, crate::nv::gr_init::GrInitSequence, Option<usize>)]> {
        if self.golden_sequences.is_empty() {
            None
        } else {
            Some(&self.golden_sequences)
        }
    }

    fn sm_version(&self) -> u32 {
        self.sm
    }

    fn firmware_chip(&self) -> &str {
        self.profile.firmware_chip
    }

    fn bridge(&self) -> &dyn GspBridge {
        &*self.gsp_bridge
    }

    fn detect_falcon_warm_state(&self, _bar0: &MappedBar, _warm_detected: bool) -> FalconWarmState {
        FalconWarmState::Cold
    }
}

// ── NVIDIA ACR Strategy (Volta, Turing, Ampere, Ada, Hopper) ────────────

/// Firmware-managed NVIDIA (Volta+): full PMC, CG sweep, ACR DMA HS falcon.
pub struct NvAcrStrategy {
    profile: GenerationProfile,
    gsp_bridge: Arc<dyn GspBridge>,
    sm: u32,
    golden_sequences: Vec<(String, crate::nv::gr_init::GrInitSequence, Option<usize>)>,
}

impl NvAcrStrategy {
    /// Create from an ACR-capable generation profile + firmware bridge.
    pub fn new(profile: GenerationProfile, bridge: Arc<dyn GspBridge>, sm: u32) -> Self {
        Self {
            profile,
            gsp_bridge: bridge,
            sm,
            golden_sequences: Vec::new(),
        }
    }

    /// Attach golden-state engine init sequences for silicon-deistic replay.
    pub fn with_golden_sequences(
        mut self,
        seqs: Vec<(String, crate::nv::gr_init::GrInitSequence, Option<usize>)>,
    ) -> Self {
        self.golden_sequences = seqs;
        self
    }
}

impl SovereignStrategy for NvAcrStrategy {
    fn family_name(&self) -> &str {
        self.profile.name
    }

    fn power_profile(&self) -> &PowerSafetyProfile {
        &self.profile.power_safety
    }

    fn needs_cg_sweep(&self) -> bool {
        true
    }

    fn needs_pgob_before_memory(&self) -> bool {
        true
    }

    fn memory_strategy(&self) -> MemoryTrainingStrategy {
        MemoryTrainingStrategy::for_memory_type(self.profile.memory_type)
    }

    fn falcon_boot_style(&self) -> FalconBootStyle {
        FalconBootStyle::AcrDmaHs
    }

    fn needs_falcon_before_memory(&self) -> bool {
        true
    }

    fn needs_gr_init_after_falcon(&self) -> bool {
        true
    }

    fn engine_ungate_sequences(
        &self,
    ) -> Option<&[(String, crate::nv::gr_init::GrInitSequence, Option<usize>)]> {
        if self.golden_sequences.is_empty() {
            None
        } else {
            Some(&self.golden_sequences)
        }
    }

    fn sm_version(&self) -> u32 {
        self.sm
    }

    fn firmware_chip(&self) -> &str {
        self.profile.firmware_chip
    }

    fn bridge(&self) -> &dyn GspBridge {
        &*self.gsp_bridge
    }

    fn pre_channel_init(&self, bar0: &MappedBar) -> Vec<StageResult> {
        use crate::vfio::sovereign_stages::{cg_sweep, pgob_ungating, pri_bus_recover};
        use crate::vfio::sovereign_types::StageStatus;
        use std::time::Instant;

        let mut stages = Vec::new();

        let t = Instant::now();
        let cg_result = cg_sweep(bar0);
        stages.push(StageResult {
            name: "pre_channel:cg_sweep".into(),
            status: StageStatus::Ok,
            detail: Some(cg_result.detail),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        let t = Instant::now();
        let pri_result = pri_bus_recover(bar0);
        stages.push(StageResult {
            name: "pre_channel:pri_recovery".into(),
            status: if pri_result.recovered {
                StageStatus::Ok
            } else {
                StageStatus::Failed
            },
            detail: Some(format!(
                "{} alive, {} faulted, recovered={}",
                pri_result.alive, pri_result.faulted, pri_result.recovered
            )),
            duration_ms: t.elapsed().as_millis() as u64,
        });

        let t = Instant::now();
        match pgob_ungating(bar0, &*self.gsp_bridge) {
            Ok(detail) => {
                stages.push(StageResult {
                    name: "pre_channel:pgob_ungating".into(),
                    status: StageStatus::Ok,
                    detail: Some(detail),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                stages.push(StageResult {
                    name: "pre_channel:pgob_ungating".into(),
                    status: StageStatus::Failed,
                    detail: Some(e.to_string()),
                    duration_ms: t.elapsed().as_millis() as u64,
                });
            }
        }

        stages
    }
}

// ── Strategy Factory ────────────────────────────────────────────────────

/// Build the appropriate `SovereignStrategy` from a generation profile.
///
/// This is the single factory that maps `GenerationProfile` data into
/// strategy objects. Adding a new NVIDIA generation means adding a profile
/// entry in `generation.rs` and (if it diverges from existing patterns)
/// a new strategy impl here.
pub fn strategy_for_profile(
    profile: &GenerationProfile,
    bridge: Arc<dyn GspBridge>,
    sm: u32,
) -> Box<dyn SovereignStrategy> {
    match profile.boot_strategy {
        BootStrategy::NoAcr => Box::new(NvKeplerStrategy::new(profile.clone(), bridge, sm)),
        BootStrategy::AcrSec2 | BootStrategy::KmodPromote | BootStrategy::Untested => {
            Box::new(NvAcrStrategy::new(profile.clone(), bridge, sm))
        }
    }
}
