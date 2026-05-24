// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::WarmInitPlan;

/// Where the seeder's kernel module comes from.
///
/// `System` means the module is already loaded (or loadable via `modprobe`).
/// `Patched` means the diesel engine finds the stock `.ko`, binary-patches
/// it at runtime, and loads the patched version via `insmod`. After the
/// warm handoff completes, the patched module is `rmmod`'d and cleaned up.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleSource {
    /// Module already loaded in the kernel (current behavior).
    /// No module lifecycle management needed.
    #[default]
    System,

    /// Load a binary-patched version of a stock module.
    /// The diesel engine handles find → patch → insmod → rmmod.
    Patched {
        /// Stock module name to find via `modinfo -n` (e.g., "nouveau").
        stock_module: String,
        /// Patch set name resolved by `cylinder::vfio::module_patch::PatchSet::by_name`
        /// (e.g., "volta_warm_handoff", "kepler_warm_handoff").
        patch_set: String,
    },
}

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

/// A driver that seeds GPU hardware initialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeederDriver {
    /// Driver name as recognized by glowplug (e.g., "nouveau", "nvidia-470").
    pub name: String,

    /// Kernel module name (e.g., "nouveau", "nvidia").
    pub module: String,

    /// Where the kernel module comes from — system or patched at runtime.
    #[serde(default)]
    pub module_source: ModuleSource,

    /// What this seeder initializes (documentation).
    pub initializes: Vec<String>,

    /// Known limitations of this seeder (documentation).
    pub limitations: Vec<String>,
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
                module_source: ModuleSource::Patched {
                    stock_module: "nouveau".into(),
                    patch_set: "volta_warm_handoff".into(),
                },
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
                module_source: ModuleSource::System,
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

    /// Create a plan for Titan V (GV100) using nvidia bare-metal seeder.
    ///
    /// Uses the already-loaded nvidia module (nvidia-580-open). nvidia's
    /// legacy RM path for Volta fully initializes: SEC2→ACR→FECS→GR engine
    /// including all TPC PRI ring stations. This tests whether nvidia's
    /// standard unbind path preserves more state than nouveau.
    ///
    /// Safe for bare-metal: nvidia module is already loaded for the display GPU.
    #[must_use]
    pub fn nvidia_bare_metal_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.to_string(),
            seeder: SeederDriver {
                name: "nvidia".into(),
                module: "nvidia".into(),
                module_source: ModuleSource::System,
                initializes: vec![
                    "SEC2 → ACR → FECS authentication chain".into(),
                    "GR engine (80 SMs, sm_70, full TPC PRI ring stations)".into(),
                    "HBM2 memory training (12GB, 3072-bit bus)".into(),
                    "All Copy Engines".into(),
                    "PFIFO + PBDMA full configuration".into(),
                    "PMU power management".into(),
                ],
                limitations: vec![
                    "nvidia's nv_pci_remove MAY tear down TPC state on unbind".into(),
                    "No teardown NOP — this tests nvidia's native unbind behavior".into(),
                    "Module cannot be unloaded (display GPU depends on it)".into(),
                ],
            },
            containment: SeederContainment::BareMetal,
            seeder_settle: Duration::from_secs(10),
            final_target: "vfio-pci".into(),
        }
    }

    /// Create a plan for Titan V (GV100) using patched nvidia injection.
    ///
    /// Copies the nvidia-580-open `.ko`, patches teardown functions to NOP,
    /// and renames the module identity from "nvidia" to "nvsov". This allows
    /// loading alongside the running nvidia module without conflict.
    ///
    /// The "diesel engine injection" approach: sovereign seeder runs in
    /// parallel with the display GPU's nvidia driver.
    #[must_use]
    pub fn nvidia_patched_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.to_string(),
            seeder: SeederDriver {
                name: "nvsov".into(),
                module: "nvsov".into(),
                module_source: ModuleSource::Patched {
                    stock_module: "nvidia".into(),
                    patch_set: "nvidia_warm_handoff".into(),
                },
                initializes: vec![
                    "SEC2 → ACR → FECS authentication chain (full)".into(),
                    "GR engine (80 SMs, sm_70, full TPC PRI ring stations)".into(),
                    "HBM2 memory training (12GB, 3072-bit bus)".into(),
                    "All Copy Engines".into(),
                    "PFIFO + PBDMA full configuration".into(),
                    "PMU power management".into(),
                ],
                limitations: vec![
                    "Dual-load: renamed module runs alongside display GPU's nvidia".into(),
                    "PCI driver registration may conflict — driver_override required".into(),
                    "Module rename relies on .modinfo NUL-bounded replacement".into(),
                ],
            },
            containment: SeederContainment::BareMetal,
            seeder_settle: Duration::from_secs(10),
            final_target: "vfio-pci".into(),
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
                module_source: ModuleSource::System,
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
}
