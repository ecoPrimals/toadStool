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

mod seeders;
mod trials;

pub use seeders::*;
pub use trials::*;

use std::time::Duration;

use serde::{Deserialize, Serialize};

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

#[cfg(target_os = "linux")]
impl WarmInitPlan {
    /// Derive a `WarmInitPlan` from the diesel engine's `HandoffConfig`.
    ///
    /// This is the canonical conversion path: the `HandoffConfig` (cylinder)
    /// is the authoritative source for BDF, driver, module source, settle
    /// time, and final target. `WarmInitPlan` adds glowplug-specific
    /// documentation (initializes/limitations) and containment policy.
    ///
    /// All bare-metal strategies (nouveau, nvidia system, nvidia patched/nvsov)
    /// produce `SeederContainment::BareMetal`. Only the agentReagents VM path
    /// uses `Contained`, which the diesel engine doesn't handle.
    #[must_use]
    pub fn from_handoff_config(
        config: &toadstool_cylinder::vfio::sovereign_handoff::HandoffConfig,
    ) -> Self {
        use toadstool_cylinder::vfio::sovereign_handoff::ModuleSourceConfig;

        let module_source = match &config.module_source {
            ModuleSourceConfig::System => ModuleSource::System,
            ModuleSourceConfig::Patched {
                stock_module,
                patch_set,
            } => ModuleSource::Patched {
                stock_module: stock_module.clone(),
                patch_set: patch_set.clone(),
            },
            ModuleSourceConfig::DkmsPatched {
                dkms_module,
                patch_set,
                ..
            } => ModuleSource::Patched {
                stock_module: dkms_module.clone(),
                patch_set: patch_set.clone(),
            },
        };

        Self {
            bdf: config.bdf.clone(),
            seeder: SeederDriver {
                name: config.seeder_driver.clone(),
                module: config.module_name.clone(),
                module_source,
                initializes: vec![format!("derived from HandoffConfig strategy")],
                limitations: vec![],
            },
            containment: SeederContainment::BareMetal,
            seeder_settle: config.settle,
            final_target: config.final_driver.clone(),
        }
    }
}

impl WarmInitPlan {
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
                steps.push(format!("agentReagents: launch VM from {reagent_template}"));
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
