// SPDX-License-Identifier: AGPL-3.0-or-later

//! Multi-stage warm initialization plans for GPU sovereign compute.
//!
//! A [`WarmInitPlan`] describes how to seed a GPU with initialization
//! state from an established driver, then access it for sovereign compute.
//!
//! # Architecture
//!
//! The cold boot problem for Volta+ GPUs: FECS (Falcon Engine Control)
//! runs in Heavy Secure (HS) mode, blocking unsigned code execution.
//! Full FECS boot requires SEC2 → ACR → FECS authentication, which
//! depends on vendor firmware the host can't replay from static MMIO.
//!
//! # Seeder Strategies
//!
//! Seeders fall into two containment categories:
//!
//! ## Bare-metal seeders (host-safe)
//!
//! Drivers that coexist peacefully with the host's driver stack.
//! glowplug swaps directly on the host kernel:
//!
//! ```text
//! ┌──────────┐     ┌────────────┐     ┌──────────┐     ┌───────────────┐
//! │ unbound  │────▶│  nouveau   │────▶│ vfio-pci │────▶│ sovereign     │
//! │ (cold)   │     │ (safe,     │     │ (warm,   │     │ compute via   │
//! │          │     │  no conflict│    │  no FLR) │     │ cylinder BAR  │
//! └──────────┘     └────────────┘     └──────────┘     └───────────────┘
//! ```
//!
//! ## Contained seeders (hazardous — agentReagents VM)
//!
//! Drivers that conflict with the host's driver stack (e.g., nvidia-470
//! vs nvidia-580 sharing the same `nvidia.ko` module name). These run
//! **inside agentReagents VMs** with the GPU passed through via VFIO.
//! The host kernel is NEVER touched — containment guarantees stability.
//!
//! ```text
//! ┌──────────┐     ┌─────────────────────────────────┐     ┌───────────────┐
//! │ vfio-pci │────▶│  agentReagents VM                │────▶│ sovereign     │
//! │ (host)   │     │  ┌─────────────────────────┐     │     │ compute via   │
//! │          │     │  │ nvidia-470 (contained)   │     │     │ VM passthrough│
//! │          │     │  │ SEC2→ACR→FECS→COMPUTE    │     │     │ or IPC        │
//! │          │     │  └─────────────────────────┘     │     └───────────────┘
//! └──────────┘     └─────────────────────────────────┘
//! ```
//!
//! # Constraints
//!
//! - **Host DRM is sacred** — the display GPU's driver is NEVER unloaded
//! - **No kernel module swaps** — conflicting drivers go in VMs, period
//! - **Multiple drivers coexist** via isolation layers, not kernel fights

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// How a seeder driver is contained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeederContainment {
    /// Safe for bare-metal: the seeder's kernel module does not conflict
    /// with any loaded host module. glowplug swaps directly on the host.
    BareMetal,

    /// Hazardous material: the seeder conflicts with the host's driver
    /// stack (e.g., nvidia-470 vs nvidia-580). Must run inside an
    /// agentReagents VM with the GPU passed through via VFIO.
    ///
    /// The reagent template name references a YAML in agentReagents
    /// that defines the VM image, driver install steps, and verification.
    Contained {
        /// agentReagents template name (e.g., "reagent-nvidia470-titanv").
        reagent_template: String,
    },
}

/// A multi-stage warm initialization plan.
///
/// Describes the complete sequence from cold/unbound GPU to sovereign
/// compute access, respecting host stability constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmInitPlan {
    /// Target PCI BDF (e.g., "0000:02:00.0").
    pub bdf: String,

    /// The seeder driver that performs hardware initialization.
    pub seeder: SeederDriver,

    /// Containment strategy for the seeder.
    pub containment: SeederContainment,

    /// How long to wait after the seeder binds before warm-swapping.
    /// For bare-metal: settle time after kernel bind.
    /// For contained: time for VM boot + driver init.
    pub seeder_settle: Duration,

    /// Final access mode after initialization.
    /// For bare-metal: "vfio-pci" (warm swap preserves state).
    /// For contained: "vm-passthrough" (GPU stays in VM).
    pub final_target: String,
}

/// A driver that seeds GPU hardware initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeederDriver {
    /// Driver name as recognized by glowplug (e.g., "nouveau", "nvidia-470").
    pub name: String,

    /// Kernel module name (e.g., "nouveau", "nvidia").
    pub module: String,

    /// What this seeder initializes (documentation).
    pub initializes: Vec<String>,

    /// Known limitations of this seeder (documentation).
    pub limitations: Vec<String>,
}

/// Result of executing a warm init plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmInitResult {
    /// PCI BDF of the target device.
    pub bdf: String,

    /// Whether the full sequence completed successfully.
    pub success: bool,

    /// Which step the sequence halted at (if any).
    pub halted_at: Option<String>,

    /// The seeder driver that was used.
    pub seeder_used: String,

    /// Whether warm state was preserved through the swap.
    pub warm_preserved: bool,

    /// Per-step outcomes.
    pub steps: Vec<WarmInitStep>,

    /// Total wall-clock time.
    pub total_ms: u64,
}

/// One step in a warm init sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmInitStep {
    /// Step name.
    pub name: String,

    /// Whether it succeeded.
    pub ok: bool,

    /// Optional detail string.
    pub detail: Option<String>,

    /// Duration in milliseconds.
    pub duration_ms: u64,
}

impl WarmInitPlan {
    /// Create a plan for Titan V (GV100) using nouveau as the seeder.
    ///
    /// nouveau initializes: PRI ring, PGRAPH, GPC clusters, memory
    /// controller, Copy Engine. It does NOT boot FECS (lacks PMU firmware
    /// for Volta ACR). This gives BAR0/BAR1/BAR3 access with warm PGRAPH.
    ///
    /// Safe for bare-metal: nouveau coexists with nvidia-580.
    #[must_use]
    pub fn nouveau_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.to_string(),
            seeder: SeederDriver {
                name: "nouveau".into(),
                module: "nouveau".into(),
                initializes: vec![
                    "PRI ring (internal register bus)".into(),
                    "PGRAPH hub + GPC clusters".into(),
                    "Memory controller (12GB HBM2)".into(),
                    "Copy Engine (CE) DMA".into(),
                    "PFIFO channel infrastructure".into(),
                ],
                limitations: vec![
                    "No FECS boot (lacks GV100 PMU firmware)".into(),
                    "SEC2 PRI route remains dead (0xbadf1100)".into(),
                    "All falcons stay in HS mode (SCTL=0x3000)".into(),
                ],
            },
            containment: SeederContainment::BareMetal,
            seeder_settle: Duration::from_secs(5),
            final_target: "vfio-pci".into(),
        }
    }

    /// Create a plan for Titan V (GV100) using nvidia-470 as the seeder.
    ///
    /// nvidia-470 fully initializes: SEC2→ACR→FECS authentication chain,
    /// HBM2 training, GR engine, all Copy Engines. This is the ONLY path
    /// to FECS compute on Volta without a custom ACR implementation.
    ///
    /// **HAZARDOUS**: nvidia-470 conflicts with nvidia-580 (same module
    /// name `nvidia.ko`). Must be contained in an agentReagents VM.
    /// The GPU is passed through via VFIO; compute runs inside the VM.
    /// Host DRM (RTX 5060) is never disturbed.
    #[must_use]
    pub fn nvidia470_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.to_string(),
            seeder: SeederDriver {
                name: "nvidia-470".into(),
                module: "nvidia".into(),
                initializes: vec![
                    "SEC2 → ACR → FECS authentication chain".into(),
                    "HBM2 memory training (12GB, 3072-bit bus)".into(),
                    "GR engine (80 SMs, sm_70)".into(),
                    "All Copy Engines".into(),
                    "PFIFO + PBDMA full configuration".into(),
                    "PMU power management".into(),
                ],
                limitations: vec![
                    "HAZARDOUS: conflicts with nvidia-580 (same nvidia.ko)".into(),
                    "Must run in agentReagents VM — never on bare metal".into(),
                    "GPU compute happens inside VM via CUDA Driver API".into(),
                    "SBR on VM shutdown destroys trained state".into(),
                ],
            },
            containment: SeederContainment::Contained {
                reagent_template: "reagent-nvidia470-titanv".into(),
            },
            seeder_settle: Duration::from_secs(30),
            final_target: "vm-passthrough".into(),
        }
    }

    /// Create a plan for Tesla K80 (GK210) using nouveau as the seeder.
    ///
    /// K80 is Kepler (NoAcr): FECS boots via direct PIO without ACR.
    /// nouveau can fully initialize GR/FECS. The K80 sits behind a PLX
    /// PEX 8747 bridge that requires SwapGuard burst keepalive during swaps.
    ///
    /// Safe for bare-metal: nouveau coexists with nvidia-580.
    #[must_use]
    pub fn nouveau_k80(bdf: &str) -> Self {
        Self {
            bdf: bdf.to_string(),
            seeder: SeederDriver {
                name: "nouveau".into(),
                module: "nouveau".into(),
                initializes: vec![
                    "PRI ring".into(),
                    "PGRAPH + FECS (direct PIO, no ACR needed)".into(),
                    "GDDR5 memory controller".into(),
                    "Copy Engine".into(),
                ],
                limitations: vec![
                    "PLX PEX 8747 bridge enters D3cold on unbind".into(),
                    "Requires SwapGuard burst keepalive during swap".into(),
                    "Manual bridge pinning insufficient (kernel PM overrides)".into(),
                ],
            },
            containment: SeederContainment::BareMetal,
            seeder_settle: Duration::from_secs(5),
            final_target: "vfio-pci".into(),
        }
    }

    /// BAR0 scan offsets for warm-state capture.
    ///
    /// Returns every 4-byte offset up to `scan_size` for use with
    /// cylinder's `WarmStateCapture::capture` / `Bar0Snapshot::capture`.
    #[must_use]
    pub fn capture_offsets(&self, scan_size: usize) -> Vec<usize> {
        (0..scan_size).step_by(4).collect()
    }

    /// Domain hints for warm-state capture labeling.
    ///
    /// Returns the `NV_BAR0_DOMAINS` table as `(&str, usize, usize)` tuples,
    /// suitable for passing to cylinder's `GrInitSequence::from_bar0_diff`.
    #[must_use]
    pub fn domain_hints(&self) -> &'static [(&'static str, usize, usize)] {
        NV_BAR0_DOMAINS
    }

    /// Default BAR0 scan size for warm-state capture (256KB covers all
    /// critical engine domains without full 16MB scan overhead).
    pub const DEFAULT_CAPTURE_SIZE: usize = 256 * 1024;

    /// Whether this plan is safe for bare-metal execution.
    #[must_use]
    pub fn is_bare_metal(&self) -> bool {
        self.containment == SeederContainment::BareMetal
    }

    /// Whether this plan requires agentReagents VM containment.
    #[must_use]
    pub fn requires_containment(&self) -> bool {
        matches!(self.containment, SeederContainment::Contained { .. })
    }

    /// The agentReagents template name, if contained.
    #[must_use]
    pub fn reagent_template(&self) -> Option<&str> {
        match &self.containment {
            SeederContainment::Contained { reagent_template } => Some(reagent_template),
            SeederContainment::BareMetal => None,
        }
    }

    /// The steps this plan will execute, in order.
    #[must_use]
    pub fn describe_steps(&self) -> Vec<String> {
        let mut steps = Vec::new();

        match &self.containment {
            SeederContainment::BareMetal => {
                steps.push(format!("bind {} → {}", self.bdf, self.seeder.name));
                steps.push(format!(
                    "settle {}ms ({})",
                    self.seeder_settle.as_millis(),
                    self.seeder
                        .initializes
                        .first()
                        .map_or("init", |s| s.as_str())
                ));
                steps.push(format!("disable_flr {}", self.bdf));
                steps.push(format!(
                    "warm swap {} → {}",
                    self.seeder.name, self.final_target
                ));
            }
            SeederContainment::Contained { reagent_template } => {
                steps.push(format!("bind {} → vfio-pci (host side)", self.bdf));
                steps.push(format!(
                    "agentReagents: launch VM from {reagent_template}"
                ));
                steps.push(format!("passthrough {} to VM", self.bdf));
                steps.push(format!(
                    "VM: {} initializes GPU ({}ms settle)",
                    self.seeder.name,
                    self.seeder_settle.as_millis()
                ));
                steps.push("compute: dispatch via VM IPC".into());
            }
        }

        steps
    }
}

impl std::fmt::Display for WarmInitPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mode = match &self.containment {
            SeederContainment::BareMetal => "bare-metal",
            SeederContainment::Contained { .. } => "contained",
        };
        write!(
            f,
            "WarmInitPlan({} via {} → {} [{}])",
            self.bdf, self.seeder.name, self.final_target, mode
        )
    }
}

impl std::fmt::Display for WarmInitResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.success {
            "SUCCESS"
        } else if let Some(ref halt) = self.halted_at {
            return write!(
                f,
                "HALTED@{halt} ({} via {}, {}ms)",
                self.bdf, self.seeder_used, self.total_ms
            );
        } else {
            "FAILED"
        };
        write!(
            f,
            "{status} ({} via {}, warm={}, {}ms)",
            self.bdf, self.seeder_used, self.warm_preserved, self.total_ms
        )
    }
}

// ── Driver Laboratory ────────────────────────────────────────────────

/// A single trial in a driver comparison lab.
///
/// Each trial binds a different seeder to the same GPU and captures
/// a BAR0 snapshot after initialization. Comparing snapshots across
/// trials reveals exactly what each driver initializes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverTrial {
    /// Human-readable label for this trial (e.g., "nouveau-warm", "nvidia470-vm").
    pub label: String,

    /// The warm init plan that seeds this trial.
    pub plan: WarmInitPlan,

    /// BAR0 register ranges to scan after seeder init.
    /// Each tuple is (domain_name, start_offset, end_offset).
    /// If empty, a default set of NVIDIA domains is used.
    pub scan_ranges: Vec<(String, usize, usize)>,

    /// Whether to perform a full BAR0 scan (slow, ~4M reads on 16MB BAR).
    /// If false, only `scan_ranges` are probed.
    pub full_scan: bool,

    /// Whether a GPU power cycle is needed before this trial.
    /// Required when transitioning from a warm state to test cold behavior,
    /// or when the previous trial's seeder leaves state that would confound results.
    pub needs_power_cycle: bool,
}

/// A plan to compare multiple drivers/seeders on the same GPU.
///
/// The diesel engine executes each trial in sequence, capturing BAR0
/// snapshots after each seeder initializes the GPU. The resulting
/// snapshots can be diffed to understand what each driver does differently.
///
/// ```text
/// Trial 1: vfio-pci (cold)     → BAR0 snapshot A (baseline)
/// Trial 2: nouveau (warm)      → BAR0 snapshot B
/// Trial 3: nvidia-470 (VM)     → BAR0 snapshot C (via agentReagents)
///
/// Diff A→B: what nouveau initializes from cold
/// Diff A→C: what nvidia-470 initializes from cold
/// Diff B→C: what nvidia-470 adds beyond nouveau (FECS, SEC2, ACR)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverLabPlan {
    /// Target GPU's PCI BDF.
    pub bdf: String,

    /// GPU description (e.g., "Titan V (GV100)").
    pub gpu_description: String,

    /// Ordered list of trials to execute.
    pub trials: Vec<DriverTrial>,

    /// Where to persist snapshots (directory path).
    pub output_dir: String,
}

/// NVIDIA BAR0 domain ranges for cartography labeling.
///
/// These map BAR0 offsets to the internal engine names used by NVIDIA GPUs
/// (Fermi through Turing/Ampere — register layout is largely stable across
/// architectures, though some offsets shift on Volta+).
pub const NV_BAR0_DOMAINS: &[(&str, usize, usize)] = &[
    ("PMC",         0x0000_0000, 0x0000_1000),
    ("PBUS",        0x0000_1000, 0x0000_2000),
    ("PFIFO",       0x0000_2000, 0x0000_4000),
    ("PTIMER",      0x0000_9000, 0x0000_A000),
    ("PFB",         0x0010_0000, 0x0010_1000),
    ("PFB_PRI",     0x0010_1000, 0x0010_2000),
    ("PBUS_PRI",    0x0010_2000, 0x0010_3000),
    ("PMCR",        0x0010_4000, 0x0010_5000),
    ("PFIFO_PRI",   0x0010_5000, 0x0010_6000),
    ("PMU",         0x0010_A000, 0x0010_B000),
    ("FECS",        0x0040_9000, 0x0040_A000),
    ("GPCCS",       0x0041_A000, 0x0041_B000),
    ("PRI_RING",    0x0012_0000, 0x0012_4000),
    ("PGRAPH",      0x0040_0000, 0x0040_1000),
    ("PGRAPH_GPC",  0x0041_0000, 0x0042_0000),
    ("PBDMA0",      0x0004_0000, 0x0004_1000),
    ("PBDMA1",      0x0004_1000, 0x0004_2000),
    ("PBDMA2",      0x0004_2000, 0x0004_3000),
    ("PBDMA3",      0x0004_3000, 0x0004_4000),
    ("CE0",         0x0010_4000, 0x0010_5000),
    ("PDISP",       0x0061_0000, 0x0061_2000),
    ("SEC2",        0x0010_AC00, 0x0010_B000),
    ("TOP",         0x0002_2400, 0x0002_2800),
    ("PRAMIN",      0x0070_0000, 0x0080_0000),
    ("PCFG",        0x0008_8000, 0x0008_9000),
];

impl DriverLabPlan {
    /// Standard comparison plan: cold baseline vs nouveau vs nvidia-470.
    ///
    /// Trial 1 captures the cold/vfio-pci baseline (no seeder — GPU as
    /// VFIO leaves it after bind). Trial 2 seeds with nouveau for mesa/open
    /// comparison. Trial 3 uses nvidia-470 in an agentReagents VM for the
    /// full vendor init comparison.
    #[must_use]
    pub fn standard_titanv(bdf: &str, output_dir: &str) -> Self {
        let domains: Vec<(String, usize, usize)> = NV_BAR0_DOMAINS
            .iter()
            .map(|&(name, start, end)| (name.to_string(), start, end))
            .collect();

        Self {
            bdf: bdf.to_string(),
            gpu_description: "Titan V (GV100)".into(),
            trials: vec![
                DriverTrial {
                    label: "cold-vfio".into(),
                    plan: WarmInitPlan {
                        bdf: bdf.to_string(),
                        seeder: SeederDriver {
                            name: "vfio-pci".into(),
                            module: "vfio-pci".into(),
                            initializes: vec!["nothing — cold baseline".into()],
                            limitations: vec![
                                "PRI ring dead after FLR".into(),
                                "All PGRAPH/FECS registers return 0xbadf".into(),
                            ],
                        },
                        containment: SeederContainment::BareMetal,
                        seeder_settle: Duration::from_secs(2),
                        final_target: "vfio-pci".into(),
                    },
                    scan_ranges: domains.clone(),
                    full_scan: false,
                    needs_power_cycle: true,
                },
                DriverTrial {
                    label: "nouveau-warm".into(),
                    plan: WarmInitPlan::nouveau_titanv(bdf),
                    scan_ranges: domains.clone(),
                    full_scan: false,
                    needs_power_cycle: true,
                },
                DriverTrial {
                    label: "nvidia470-vm".into(),
                    plan: WarmInitPlan::nvidia470_titanv(bdf),
                    scan_ranges: domains,
                    full_scan: false,
                    needs_power_cycle: true,
                },
            ],
            output_dir: output_dir.to_string(),
        }
    }

    /// Describe the lab plan as a human-readable sequence.
    #[must_use]
    pub fn describe(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(format!(
            "Driver Lab: {} ({}) — {} trials",
            self.gpu_description,
            self.bdf,
            self.trials.len(),
        ));

        for (i, trial) in self.trials.iter().enumerate() {
            let containment = if trial.plan.requires_containment() {
                " [contained/VM]"
            } else {
                " [bare-metal]"
            };
            let power = if trial.needs_power_cycle {
                " (power cycle first)"
            } else {
                ""
            };
            let scan = if trial.full_scan {
                "full BAR0"
            } else {
                &format!("{} ranges", trial.scan_ranges.len())
            };
            out.push(format!(
                "  Trial {}: {} via {}{}{} — scan {}",
                i + 1,
                trial.label,
                trial.plan.seeder.name,
                containment,
                power,
                scan,
            ));
        }

        out.push(format!("Output: {}", self.output_dir));

        let n = self.trials.len();
        if n >= 2 {
            out.push(format!("Diffs to generate: {} pairwise comparisons", n * (n - 1) / 2));
        }

        out
    }

    /// List all pairwise diff combinations that should be generated.
    #[must_use]
    pub fn diff_pairs(&self) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        for i in 0..self.trials.len() {
            for j in (i + 1)..self.trials.len() {
                pairs.push((i, j));
            }
        }
        pairs
    }
}

impl std::fmt::Display for DriverLabPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DriverLab({}, {} trials: {})",
            self.bdf,
            self.trials.len(),
            self.trials
                .iter()
                .map(|t| t.label.as_str())
                .collect::<Vec<_>>()
                .join(" → "),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nouveau_titanv_is_bare_metal() {
        let plan = WarmInitPlan::nouveau_titanv("0000:02:00.0");
        assert!(plan.is_bare_metal());
        assert!(!plan.requires_containment());
        assert!(plan.reagent_template().is_none());
        assert_eq!(plan.seeder.name, "nouveau");
        assert_eq!(plan.final_target, "vfio-pci");
    }

    #[test]
    fn nvidia470_titanv_requires_containment() {
        let plan = WarmInitPlan::nvidia470_titanv("0000:02:00.0");
        assert!(!plan.is_bare_metal());
        assert!(plan.requires_containment());
        assert_eq!(
            plan.reagent_template(),
            Some("reagent-nvidia470-titanv")
        );
        assert_eq!(plan.final_target, "vm-passthrough");
    }

    #[test]
    fn nvidia470_seeder_documents_hazard() {
        let plan = WarmInitPlan::nvidia470_titanv("0000:02:00.0");
        assert!(plan
            .seeder
            .limitations
            .iter()
            .any(|l| l.contains("HAZARDOUS")));
        assert!(plan
            .seeder
            .limitations
            .iter()
            .any(|l| l.contains("never on bare metal")));
    }

    #[test]
    fn k80_plan_is_bare_metal_with_plx_warning() {
        let plan = WarmInitPlan::nouveau_k80("0000:4b:00.0");
        assert!(plan.is_bare_metal());
        assert!(plan
            .seeder
            .limitations
            .iter()
            .any(|l| l.contains("PLX")));
    }

    #[test]
    fn describe_steps_bare_metal_has_warm_swap() {
        let plan = WarmInitPlan::nouveau_titanv("0000:02:00.0");
        let steps = plan.describe_steps();
        assert!(steps.iter().any(|s| s.contains("bind")));
        assert!(steps.iter().any(|s| s.contains("disable_flr")));
        assert!(steps.iter().any(|s| s.contains("warm swap")));
        assert!(!steps.iter().any(|s| s.contains("agentReagents")));
    }

    #[test]
    fn describe_steps_contained_uses_vm() {
        let plan = WarmInitPlan::nvidia470_titanv("0000:02:00.0");
        let steps = plan.describe_steps();
        assert!(steps.iter().any(|s| s.contains("agentReagents")));
        assert!(steps.iter().any(|s| s.contains("passthrough")));
        assert!(steps.iter().any(|s| s.contains("VM")));
        assert!(!steps.iter().any(|s| s.contains("disable_flr")));
    }

    #[test]
    fn plan_display_shows_containment() {
        let plan = WarmInitPlan::nouveau_titanv("0000:02:00.0");
        let s = plan.to_string();
        assert!(s.contains("bare-metal"));

        let plan2 = WarmInitPlan::nvidia470_titanv("0000:02:00.0");
        let s2 = plan2.to_string();
        assert!(s2.contains("contained"));
    }

    #[test]
    fn result_display_success() {
        let r = WarmInitResult {
            bdf: "0000:02:00.0".into(),
            success: true,
            halted_at: None,
            seeder_used: "nouveau".into(),
            warm_preserved: true,
            steps: vec![],
            total_ms: 42,
        };
        assert!(r.to_string().contains("SUCCESS"));
        assert!(r.to_string().contains("warm=true"));
    }

    #[test]
    fn result_display_halted() {
        let r = WarmInitResult {
            bdf: "0000:02:00.0".into(),
            success: false,
            halted_at: Some("seeder_bind".into()),
            seeder_used: "nvidia-470".into(),
            warm_preserved: false,
            steps: vec![],
            total_ms: 100,
        };
        assert!(r.to_string().contains("HALTED@seeder_bind"));
    }

    #[test]
    fn warm_init_step_serde_roundtrip() {
        let step = WarmInitStep {
            name: "seeder_bind".into(),
            ok: true,
            detail: Some("bound nouveau".into()),
            duration_ms: 3200,
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: WarmInitStep = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "seeder_bind");
        assert!(back.ok);
    }

    // ── Driver Lab tests ──────────────────────────────────────────

    #[test]
    fn standard_titanv_lab_has_three_trials() {
        let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
        assert_eq!(lab.trials.len(), 3);
        assert_eq!(lab.trials[0].label, "cold-vfio");
        assert_eq!(lab.trials[1].label, "nouveau-warm");
        assert_eq!(lab.trials[2].label, "nvidia470-vm");
    }

    #[test]
    fn standard_lab_cold_is_bare_metal() {
        let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
        assert!(lab.trials[0].plan.is_bare_metal());
    }

    #[test]
    fn standard_lab_nvidia470_is_contained() {
        let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
        assert!(lab.trials[2].plan.requires_containment());
    }

    #[test]
    fn lab_diff_pairs_three_trials() {
        let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
        let pairs = lab.diff_pairs();
        assert_eq!(pairs, vec![(0, 1), (0, 2), (1, 2)]);
    }

    #[test]
    fn lab_describe_lists_all_trials() {
        let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
        let desc = lab.describe();
        assert!(desc[0].contains("3 trials"));
        assert!(desc.iter().any(|s| s.contains("cold-vfio")));
        assert!(desc.iter().any(|s| s.contains("nouveau-warm")));
        assert!(desc.iter().any(|s| s.contains("nvidia470-vm")));
        assert!(desc.iter().any(|s| s.contains("[contained/VM]")));
        assert!(desc.iter().any(|s| s.contains("[bare-metal]")));
    }

    #[test]
    fn lab_display_format() {
        let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
        let s = lab.to_string();
        assert!(s.contains("cold-vfio → nouveau-warm → nvidia470-vm"));
    }

    #[test]
    fn nv_domain_hints_cover_key_regions() {
        let names: Vec<&str> = NV_BAR0_DOMAINS.iter().map(|d| d.0).collect();
        assert!(names.contains(&"PMC"));
        assert!(names.contains(&"PGRAPH"));
        assert!(names.contains(&"FECS"));
        assert!(names.contains(&"PRI_RING"));
        assert!(names.contains(&"SEC2"));
        assert!(names.contains(&"PMU"));
        assert!(names.contains(&"PFB"));
    }

    #[test]
    fn nv_domain_hints_all_valid_ranges() {
        for &(name, start, end) in NV_BAR0_DOMAINS {
            assert!(
                start < end,
                "{name}: start {start:#x} >= end {end:#x}"
            );
            assert!(
                start % 4 == 0 && end % 4 == 0,
                "{name}: not 4-byte aligned"
            );
            assert!(
                end <= 0x0100_0000,
                "{name}: exceeds 16MB BAR0"
            );
        }
    }

    #[test]
    fn driver_trial_serde_roundtrip() {
        let trial = DriverTrial {
            label: "nouveau-test".into(),
            plan: WarmInitPlan::nouveau_titanv("0000:02:00.0"),
            scan_ranges: vec![("PMC".into(), 0, 0x1000)],
            full_scan: false,
            needs_power_cycle: false,
        };
        let json = serde_json::to_string(&trial).unwrap();
        let back: DriverTrial = serde_json::from_str(&json).unwrap();
        assert_eq!(back.label, "nouveau-test");
        assert!(!back.full_scan);
    }

    // ── Containment tests ────────────────────────────────────────

    #[test]
    fn containment_serde_roundtrip() {
        let bm = SeederContainment::BareMetal;
        let json = serde_json::to_string(&bm).unwrap();
        let back: SeederContainment = serde_json::from_str(&json).unwrap();
        assert_eq!(back, SeederContainment::BareMetal);

        let contained = SeederContainment::Contained {
            reagent_template: "reagent-nvidia470-titanv".into(),
        };
        let json = serde_json::to_string(&contained).unwrap();
        let back: SeederContainment = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, SeederContainment::Contained { .. }));
    }
}
