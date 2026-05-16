// SPDX-License-Identifier: AGPL-3.0-or-later

//! Structured driver comparison tool — multi-driver trial results.
//!
//! Formalizes the "Driver Lab" concept from Experiment 195 into reusable
//! types. Each `TrialResult` captures the key hardware observations from
//! binding a specific driver, enabling systematic comparison across drivers
//! (nouveau, nvidia-470, nvidia-open, etc.).
//!
//! The key question each trial answers: "What did this driver initialize?"
//! — measured by PMC engine enables, PGRAPH liveness, falcon states, and
//! PFIFO availability.

use serde::{Deserialize, Serialize};

use crate::nv::pri::is_pri_fault;
use crate::vfio::device::MappedBar;
use crate::vfio::warm_capture::Bar0Snapshot;

/// Observed state of an NVIDIA falcon microcontroller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FalconState {
    /// CPUCTL reads zero — falcon has not been started.
    NotStarted,
    /// Falcon is halted at a specific program counter.
    Halted {
        /// Program counter value from FALCON_CPUCTL.
        pc: u32,
    },
    /// Falcon is running (STARTCPU bit set, not halted).
    Running {
        /// Last observed program counter.
        pc: u32,
    },
    /// Falcon is in Heavy Secure mode — PRI-gated, IMEM inaccessible.
    HsLocked {
        /// Raw SCTL register value.
        sctl: u32,
    },
    /// Falcon registers return PRI fault (0xBADFxxxx) — clock-gated or
    /// security-gated.
    PriGated,
}

impl FalconState {
    /// Probe falcon state from BAR0 using the CPUCTL register.
    ///
    /// `base` is the falcon's BAR0 base (e.g. 0x409000 for FECS,
    /// 0x10A000 for PMU, 0x840000 for SEC2).
    pub fn probe(bar0: &MappedBar, base: usize) -> Self {
        let cpuctl_offset = base + 0x100;
        let cpuctl = bar0.read_u32(cpuctl_offset).unwrap_or(0xDEAD_DEAD);

        if is_pri_fault(cpuctl) {
            return Self::PriGated;
        }

        // Check for HS lock via SCTL (base + 0x240)
        let sctl_offset = base + 0x240;
        let sctl = bar0.read_u32(sctl_offset).unwrap_or(0);
        if sctl & 0x02 != 0 {
            return Self::HsLocked { sctl };
        }

        if cpuctl == 0 {
            return Self::NotStarted;
        }

        // CPUCTL bit 4 = HALTED
        let halted = cpuctl & (1 << 4) != 0;

        // Read PC from FALCON_PC (base + 0x130)
        let pc = bar0.read_u32(base + 0x130).unwrap_or(0);

        if halted {
            Self::Halted { pc }
        } else {
            Self::Running { pc }
        }
    }

    /// Whether the falcon is usable for firmware upload.
    pub fn accepts_firmware(&self) -> bool {
        matches!(self, Self::NotStarted | Self::Halted { .. })
    }

    /// Whether the falcon is PRI-gated (clock-gated or security-gated).
    pub fn is_gated(&self) -> bool {
        matches!(self, Self::PriGated)
    }

    /// Human-readable short description.
    pub fn short_desc(&self) -> String {
        match self {
            Self::NotStarted => "not started".into(),
            Self::Halted { pc } => format!("halted @ {pc:#x}"),
            Self::Running { pc } => format!("running @ {pc:#x}"),
            Self::HsLocked { sctl } => format!("HS locked (sctl={sctl:#x})"),
            Self::PriGated => "PRI gated".into(),
        }
    }
}

/// Result of a single driver trial — what hardware state a driver achieved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialResult {
    /// Driver identifier (e.g. "nouveau", "nvidia-470.256.02").
    pub driver: String,
    /// BAR0 snapshot after driver init.
    pub snapshot: Bar0Snapshot,
    /// PMC_ENABLE register value (engine enable mask).
    pub pmc_enable: u32,
    /// Count of active (enabled) engines.
    pub active_engines: u32,
    /// Whether PGRAPH registers are responsive (not PRI-gated).
    pub pgraph_alive: bool,
    /// FECS falcon state.
    pub fecs_state: FalconState,
    /// PMU falcon state.
    pub pmu_state: FalconState,
    /// SEC2 falcon state (Volta+).
    pub sec2_state: FalconState,
    /// Whether PFIFO_ENABLE is set.
    pub pfifo_enabled: bool,
}

impl TrialResult {
    /// Probe current hardware state and build a trial result.
    ///
    /// `bar0` must be a mapped BAR0 from a VFIO device that was just
    /// swapped from the `driver` being probed.
    pub fn probe(bar0: &MappedBar, bdf: &str, driver: &str, scan_offsets: &[usize]) -> Self {
        let snapshot = Bar0Snapshot::capture(bar0, bdf, &format!("{driver}-warm"), scan_offsets);

        let pmc_enable = bar0.read_u32(0x200).unwrap_or(0);
        let active_engines = pmc_enable.count_ones();

        // PGRAPH status register
        let pgraph_status = bar0.read_u32(0x0040_0700).unwrap_or(0xDEAD_DEAD);
        let pgraph_alive = !is_pri_fault(pgraph_status);

        let fecs_state = FalconState::probe(bar0, 0x0040_9000);
        let pmu_state = FalconState::probe(bar0, 0x0010_A000);
        let sec2_state = FalconState::probe(bar0, 0x0084_0000);

        let pfifo_enable = bar0.read_u32(0x2200).unwrap_or(0);
        let pfifo_enabled = pfifo_enable & 1 != 0;

        Self {
            driver: driver.to_string(),
            snapshot,
            pmc_enable,
            active_engines,
            pgraph_alive,
            fecs_state,
            pmu_state,
            sec2_state,
            pfifo_enabled,
        }
    }

    /// Summary for logging.
    pub fn summary(&self) -> String {
        format!(
            "Trial({driver}): PMC={pmc:#010x} ({engines} engines), \
             PGRAPH={pgraph}, FECS={fecs}, PMU={pmu}, SEC2={sec2}, \
             PFIFO={pfifo}",
            driver = self.driver,
            pmc = self.pmc_enable,
            engines = self.active_engines,
            pgraph = if self.pgraph_alive { "alive" } else { "gated" },
            fecs = self.fecs_state.short_desc(),
            pmu = self.pmu_state.short_desc(),
            sec2 = self.sec2_state.short_desc(),
            pfifo = if self.pfifo_enabled { "enabled" } else { "disabled" },
        )
    }
}

/// Multi-driver probe comparison for a single GPU.
#[derive(Debug, Clone)]
pub struct DriverProbe {
    /// BDF of the device under test.
    pub bdf: String,
    /// Results from each driver trial.
    pub trials: Vec<TrialResult>,
}

impl DriverProbe {
    /// Create a new probe for a device.
    pub fn new(bdf: &str) -> Self {
        Self {
            bdf: bdf.to_string(),
            trials: Vec::new(),
        }
    }

    /// Add a completed trial result.
    pub fn add_trial(&mut self, trial: TrialResult) {
        self.trials.push(trial);
    }

    /// Find the trial that achieved the most active engines.
    pub fn best_by_engines(&self) -> Option<&TrialResult> {
        self.trials.iter().max_by_key(|t| t.active_engines)
    }

    /// Find trials where PGRAPH is alive.
    pub fn pgraph_alive_trials(&self) -> Vec<&TrialResult> {
        self.trials.iter().filter(|t| t.pgraph_alive).collect()
    }

    /// Find trials where FECS accepts firmware upload.
    pub fn fecs_uploadable_trials(&self) -> Vec<&TrialResult> {
        self.trials
            .iter()
            .filter(|t| t.fecs_state.accepts_firmware())
            .collect()
    }

    /// Comparison summary across all trials.
    pub fn comparison_summary(&self) -> String {
        let mut lines = vec![format!("DriverProbe({}) — {} trials:", self.bdf, self.trials.len())];
        for trial in &self.trials {
            lines.push(format!("  {}", trial.summary()));
        }
        if let Some(best) = self.best_by_engines() {
            lines.push(format!("  Best by engines: {} ({} engines)", best.driver, best.active_engines));
        }
        lines.join("\n")
    }
}

impl std::fmt::Display for DriverProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.comparison_summary())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falcon_state_not_started() {
        let state = FalconState::NotStarted;
        assert!(state.accepts_firmware());
        assert!(!state.is_gated());
        assert_eq!(state.short_desc(), "not started");
    }

    #[test]
    fn falcon_state_halted() {
        let state = FalconState::Halted { pc: 0x1234 };
        assert!(state.accepts_firmware());
        assert!(!state.is_gated());
        assert!(state.short_desc().contains("0x1234"));
    }

    #[test]
    fn falcon_state_running() {
        let state = FalconState::Running { pc: 0x42 };
        assert!(!state.accepts_firmware());
        assert!(state.short_desc().contains("running"));
    }

    #[test]
    fn falcon_state_hs_locked() {
        let state = FalconState::HsLocked { sctl: 0x02 };
        assert!(!state.accepts_firmware());
        assert!(!state.is_gated());
        assert!(state.short_desc().contains("HS locked"));
    }

    #[test]
    fn falcon_state_pri_gated() {
        let state = FalconState::PriGated;
        assert!(!state.accepts_firmware());
        assert!(state.is_gated());
        assert_eq!(state.short_desc(), "PRI gated");
    }

    #[test]
    fn trial_result_summary() {
        let trial = TrialResult {
            driver: "nouveau".into(),
            snapshot: Bar0Snapshot {
                bdf: "0000:41:00.0".into(),
                label: "nouveau-warm".into(),
                registers: vec![],
                timestamp_ms: 0,
            },
            pmc_enable: 0x5fec_dff1,
            active_engines: 23,
            pgraph_alive: true,
            fecs_state: FalconState::Running { pc: 0x100 },
            pmu_state: FalconState::Running { pc: 0x200 },
            sec2_state: FalconState::HsLocked { sctl: 0x02 },
            pfifo_enabled: true,
        };
        let s = trial.summary();
        assert!(s.contains("nouveau"));
        assert!(s.contains("23 engines"));
        assert!(s.contains("alive"));
        assert!(s.contains("enabled"));
    }

    #[test]
    fn driver_probe_comparison() {
        let mut probe = DriverProbe::new("0000:41:00.0");
        probe.add_trial(TrialResult {
            driver: "nouveau".into(),
            snapshot: Bar0Snapshot {
                bdf: "0000:41:00.0".into(),
                label: "nouveau-warm".into(),
                registers: vec![],
                timestamp_ms: 0,
            },
            pmc_enable: 0x5fec_dff1,
            active_engines: 23,
            pgraph_alive: true,
            fecs_state: FalconState::Running { pc: 0x100 },
            pmu_state: FalconState::Running { pc: 0x200 },
            sec2_state: FalconState::NotStarted,
            pfifo_enabled: true,
        });
        probe.add_trial(TrialResult {
            driver: "nvidia-470".into(),
            snapshot: Bar0Snapshot {
                bdf: "0000:41:00.0".into(),
                label: "nvidia-warm".into(),
                registers: vec![],
                timestamp_ms: 0,
            },
            pmc_enable: 0xFFFF_FFFF,
            active_engines: 32,
            pgraph_alive: true,
            fecs_state: FalconState::HsLocked { sctl: 0x02 },
            pmu_state: FalconState::Running { pc: 0x300 },
            sec2_state: FalconState::Running { pc: 0x400 },
            pfifo_enabled: true,
        });

        let best = probe.best_by_engines().unwrap();
        assert_eq!(best.driver, "nvidia-470");
        assert_eq!(best.active_engines, 32);

        let pgraph_alive = probe.pgraph_alive_trials();
        assert_eq!(pgraph_alive.len(), 2);

        let fecs_up = probe.fecs_uploadable_trials();
        assert_eq!(fecs_up.len(), 0); // both are Running/HsLocked

        let summary = probe.comparison_summary();
        assert!(summary.contains("2 trials"));
        assert!(summary.contains("nouveau"));
        assert!(summary.contains("nvidia-470"));
    }

    #[test]
    fn falcon_state_serde_roundtrip() {
        let states = vec![
            FalconState::NotStarted,
            FalconState::Halted { pc: 0x42 },
            FalconState::Running { pc: 0x100 },
            FalconState::HsLocked { sctl: 0x02 },
            FalconState::PriGated,
        ];
        for state in &states {
            let json = serde_json::to_string(state).unwrap();
            let back: FalconState = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, state);
        }
    }
}
