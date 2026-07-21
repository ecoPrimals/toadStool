// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use super::types::{HandoffConfig, ModuleSourceConfig};

/// nvidia-470 is the last driver branch that supports GV100 (Titan V).
/// nvidia-580+ dropped GV100 support entirely. Catalyst strategies for
/// Titan V MUST use this version regardless of what's installed.
const NVIDIA_470_DKMS_VERSION: &str = "470.256.02";

impl HandoffConfig {
    /// Create a config for Titan V warm handoff via patched nouveau.
    #[must_use]
    pub fn nouveau_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nouveau".into(),
            module_name: "nouveau".into(),
            module_source: ModuleSourceConfig::Patched {
                stock_module: "nouveau".into(),
                patch_set: "volta_warm_handoff".into(),
            },
            settle: Duration::from_secs(5),
            final_driver: "vfio-pci".into(),
            patch_set_override: None,
            skip_preflight: false,
            sm_version: Some(70),
        }
    }

    /// Create a config for K80 warm handoff via stock nouveau.
    #[must_use]
    pub fn nouveau_k80(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nouveau".into(),
            module_name: "nouveau".into(),
            module_source: ModuleSourceConfig::System,
            settle: Duration::from_secs(5),
            final_driver: "vfio-pci".into(),
            patch_set_override: None,
            skip_preflight: false,
            sm_version: Some(35),
        }
    }

    /// Create a config for Titan V warm handoff via nvidia (system module, no patching).
    ///
    /// Uses the already-loaded nvidia driver as the seeder. nvidia's legacy RM
    /// fully initializes Volta (SEC2→ACR→FECS→GR→TPC). On unbind, nvidia's
    /// `nv_pci_remove` WILL tear down state — this strategy tests what nvidia
    /// preserves versus nouveau, without requiring module patching.
    ///
    /// The nvidia module is already loaded (RTX 5060 display GPU), so this
    /// uses `ModuleSourceConfig::System`.
    #[must_use]
    pub fn nvidia_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nvidia".into(),
            module_name: "nvidia".into(),
            module_source: ModuleSourceConfig::System,
            settle: Duration::from_secs(10),
            final_driver: "vfio-pci".into(),
            patch_set_override: None,
            skip_preflight: false,
            sm_version: Some(70),
        }
    }

    /// Create a config for Titan V warm handoff via patched nvidia-470 (dual-load injection).
    ///
    /// Uses the DKMS-built nvidia-470 proprietary `.ko` (which supports Volta
    /// via legacy RM, unlike nvidia-580-open which requires GSP). Patches
    /// `nv_pci_remove` and other teardown functions to NOP, and renames the
    /// module identity from "nvidia" to "nvsov" so it can be loaded alongside
    /// the running nvidia-580 module.
    ///
    /// This is the "agent reagents diesel engine" approach: the patched module
    /// is injected as a sovereign seeder while the display GPU's nvidia driver
    /// runs undisturbed.
    #[must_use]
    pub fn nvidia_patched_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nvsov".into(),
            module_name: "nvsov".into(),
            module_source: ModuleSourceConfig::DkmsPatched {
                dkms_module: "nvidia".into(),
                dkms_version: NVIDIA_470_DKMS_VERSION.into(),
                patch_set: "nvidia_warm_handoff".into(),
            },
            settle: Duration::from_secs(60),
            final_driver: "vfio-pci".into(),
            patch_set_override: None,
            skip_preflight: false,
            sm_version: Some(70),
        }
    }

    /// Create a config for Titan V catalyst handoff via selectively un-NOPed nvidia-470.
    ///
    /// Uses the catalyst patch set (`nvidia_catalyst_handoff`) which bypasses
    /// `nv_cap_validate_and_dup_fd` for RM alloc support and stubs cap init
    /// to fake handles, allowing RM to fully initialize the compute pipeline
    /// (SEC2/ACR/PMU/GPCCS/FECS/TPC) and establish compute channels.
    /// The pipeline captures BAR0 state while the catalyst owns the GPU,
    /// then warm-swaps to vfio-pci and classifies.
    ///
    /// Settle time is 60s to ensure RM completes the full init chain on
    /// GV100 (SEC2→ACR→FECS→GPCCS→TPC PRI station creation).
    #[must_use]
    pub fn nvidia_catalyst_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nvsov".into(),
            module_name: "nvsov".into(),
            module_source: ModuleSourceConfig::DkmsPatched {
                dkms_module: "nvidia".into(),
                dkms_version: NVIDIA_470_DKMS_VERSION.into(),
                patch_set: "nvidia_catalyst_handoff".into(),
            },
            settle: Duration::from_secs(60),
            final_driver: "vfio-pci".into(),
            patch_set_override: None,
            skip_preflight: false,
            sm_version: Some(70),
        }
    }

    /// Exp 234: Minimal un-NOP variant — restores the cap subsystem so RM
    /// can populate its device table. Uses `nvidia_catalyst_minimal_nop`
    /// patch set which un-NOPs nv_cap_init, nv_cap_drv_init,
    /// nv_cap_create_dir_entry, nv_cap_create_file_entry.
    #[must_use]
    pub fn nvidia_catalyst_minimal_nop_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nvsov".into(),
            module_name: "nvsov".into(),
            module_source: ModuleSourceConfig::DkmsPatched {
                dkms_module: "nvidia".into(),
                dkms_version: NVIDIA_470_DKMS_VERSION.into(),
                patch_set: "nvidia_catalyst_minimal_nop".into(),
            },
            settle: Duration::from_secs(60),
            final_driver: "vfio-pci".into(),
            patch_set_override: None,
            skip_preflight: false,
            sm_version: Some(70),
        }
    }

    /// Create a config for Titan V boot-services handoff — preserves PRI
    /// ring stations across the swap by NOPing all state-destroying calls
    /// in nv_pci_remove. This is the ExitBootServices pattern.
    #[must_use]
    pub fn nvidia_boot_services_titanv(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nvsov".into(),
            module_name: "nvsov".into(),
            module_source: ModuleSourceConfig::DkmsPatched {
                dkms_module: "nvidia".into(),
                dkms_version: NVIDIA_470_DKMS_VERSION.into(),
                patch_set: "nvidia_boot_services".into(),
            },
            settle: Duration::from_secs(60),
            final_driver: "vfio-pci".into(),
            patch_set_override: None,
            skip_preflight: false,
            sm_version: Some(70),
        }
    }

    /// Resolve a config from a strategy name and BDF.
    #[must_use]
    pub fn from_strategy(strategy: &str, bdf: &str) -> Option<Self> {
        match strategy {
            "nouveau_titanv" => Some(Self::nouveau_titanv(bdf)),
            "nouveau_k80" => Some(Self::nouveau_k80(bdf)),
            "nvidia_titanv" => Some(Self::nvidia_titanv(bdf)),
            "nvidia_patched_titanv" => Some(Self::nvidia_patched_titanv(bdf)),
            "nvidia_catalyst_titanv" => Some(Self::nvidia_catalyst_titanv(bdf)),
            "nvidia_catalyst_minimal_nop_titanv" => {
                Some(Self::nvidia_catalyst_minimal_nop_titanv(bdf))
            }
            "nvidia_boot_services_titanv" => Some(Self::nvidia_boot_services_titanv(bdf)),
            "nvidia_runtime_services" => Some(Self::nvidia_runtime_services(bdf)),
            _ => None,
        }
    }

    /// Create a config for nvidia runtime services mode — nvidia stays
    /// bound as a persistent compute backend. No unbind/swap occurs.
    ///
    /// toadStool manages infrastructure (PFIFO, DMA, VRAM, PRI ring)
    /// while nvidia's FECS/GPCCS context remains live for Tier 2 compute.
    /// Reagent capture runs in parallel to extract firmware chemical agents.
    #[must_use]
    pub fn nvidia_runtime_services(bdf: &str) -> Self {
        Self {
            bdf: bdf.into(),
            seeder_driver: "nvidia".into(),
            module_name: "nvidia".into(),
            module_source: ModuleSourceConfig::System,
            settle: Duration::from_secs(0),
            final_driver: "nvidia".into(),
            patch_set_override: None,
            skip_preflight: true,
            sm_version: None,
        }
    }

    /// Whether this config uses runtime services mode (nvidia stays bound).
    #[must_use]
    pub fn is_runtime_services(&self) -> bool {
        self.final_driver == "nvidia" || self.final_driver == self.seeder_driver
    }
}
