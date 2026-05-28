// SPDX-License-Identifier: AGPL-3.0-or-later
use super::super::types::{PatchSet, PatchStrategy, PatchTarget};

impl PatchSet {
    /// Patch set for Volta (GV100) warm handoff via nvidia open kernel module.
    ///
    /// Targets the nvidia-580-open (or compatible) module's PCI remove path.
    /// `nv_pci_remove` is the per-device teardown entry — NOPing it preserves
    /// the full RM-initialized state (SEC2→ACR→FECS→GR→TPC PRI ring stations).
    ///
    /// Also targets `gpuStateUnload_IMPL` (master engine unload dispatcher)
    /// and `gpuStateDestroy_IMPL` as fallbacks if `nv_pci_remove` cannot be
    /// resolved (symbol visibility varies across driver versions).
    #[must_use]
    pub fn nvidia_warm_handoff() -> Self {
        Self {
            name: "nvidia_warm_handoff".into(),
            module_name: "nvidia".into(),
            targets: vec![
                // Teardown NOPs — preserve GPU state on unbind
                PatchTarget {
                    symbol: "nv_pci_remove".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "gpuStateUnload_IMPL".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "gpuStateDestroy_IMPL".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "_deviceTeardown".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "clTeardown_IMPL".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "fecsBufferTeardown".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                // Co-load isolation NOPs — prevent conflicts with host nvidia.
                // nv_cap_init returns an opaque handle; nvidia_init_module
                // treats 0 as failure. Use Ret1AtEntry so the init check
                // passes while skipping the procfs registration that
                // conflicts with host nvidia's /proc/driver/nvidia/.
                PatchTarget {
                    symbol: "nv_cap_init".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                PatchTarget {
                    symbol: "nv_cap_drv_init".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                PatchTarget {
                    symbol: "nv_procfs_init".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                // nvidia_register_module must run — it populates the
                // module instance table that nv_pci_probe needs.
                PatchTarget {
                    symbol: "nv_cap_procfs_init".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                // nvlink/nvswitch subsystem procfs conflicts
                PatchTarget {
                    symbol: "nvlink_core_init".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                PatchTarget {
                    symbol: "nvswitch_init".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                // ACPI init tries to register duplicate handlers
                PatchTarget {
                    symbol: "nv_acpi_init".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                // Cap subsystem stubs — RM calls these but we NOPed
                // the init; return NULL so RM skips cap operations.
                PatchTarget {
                    symbol: "nv_cap_create_dir_entry".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "nv_cap_create_file_entry".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "nv_cap_destroy_entry".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                // NOP the __register_chrdev call inside init_module
                // (nvidia_frontend_init_module). Host nvidia owns major 195;
                // a second registration fails, causing module init failure.
                PatchTarget {
                    symbol: "init_module".into(),
                    strategy: PatchStrategy::NopCallAt(0x7f),
                },
            ],
            min_applied: 1,
        }
    }

    /// Catalyst variant — allows full RM compute channel setup for
    /// sovereign dispatch. Stubs cap init/create to fake handles and
    /// bypasses `nv_cap_validate_and_dup_fd` so RM alloc ioctls succeed
    /// with `status=0`. Keeps procfs/chardev isolation NOPs to prevent
    /// host conflicts.
    pub fn nvidia_catalyst_handoff() -> Self {
        Self {
            name: "nvidia_catalyst_handoff".into(),
            module_name: "nvidia".into(),
            targets: vec![
                // Preserve GPU hardware state through close + unbind.
                // Both rm_disable_adapter and rm_shutdown_adapter must be
                // NOP'd:
                //   rm_disable_adapter: disables PRI ring stations and
                //     engine interrupts, which trips 0xbadf PRI faults
                //     and partially cools PMC_ENABLE (popcount 25→10).
                //   rm_shutdown_adapter: resets GPU hardware, powers down
                //     engines, unloads FECS/GPCCS firmware.
                // With both NOP'd, the GPU keeps full warm state (all
                // engines enabled, FECS firmware loaded, PRI ring live)
                // through nvidia close AND unbind. The brief window of
                // unhandled MSI interrupts between free_irq and vfio-pci
                // bind is acceptable for catalyst.
                PatchTarget {
                    symbol: "rm_disable_adapter".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "rm_shutdown_adapter".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                // Co-load isolation NOPs — prevent host conflicts.
                // nv_cap_init/drv_init are NOPed to Ret1 (return success)
                // because their internal calls to nv_cap_create_dir_entry
                // (also NOPed) would return NULL, causing nv_cap_init to
                // fail. The cap system is for /dev/nvidia-caps access
                // control, not GPU hardware init. RM proceeds to probe
                // and full compute init (SEC2/ACR/FECS/GPCCS/TPC)
                // without functional caps.
                PatchTarget {
                    symbol: "nv_cap_init".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                PatchTarget {
                    symbol: "nv_cap_drv_init".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                PatchTarget {
                    symbol: "nv_procfs_init".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "nv_cap_procfs_init".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "nvlink_core_init".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                PatchTarget {
                    symbol: "nvswitch_init".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                PatchTarget {
                    symbol: "nv_acpi_init".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                // Return non-NULL fake handle (1) so RM's NULL checks
                // pass during nvidia_frontend_open.
                PatchTarget {
                    symbol: "nv_cap_create_dir_entry".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                PatchTarget {
                    symbol: "nv_cap_create_file_entry".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                PatchTarget {
                    symbol: "nv_cap_destroy_entry".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                // Bypass cap validation for RM alloc ioctls.
                // nv_cap_validate_and_dup_fd normally dereferences the
                // cap pointer to check cap->minor, which crashes with
                // our fake cap pointers (value=1). Returning 1 (a valid
                // positive fd number) makes RM treat the client as having
                // full capabilities. RetAtEntry returned 0 (stdin) which
                // RM rejects as INSUFFICIENT_PERMISSIONS (0x1b).
                // nv_cap_close_fd is NOPed to prevent closing fd 1.
                PatchTarget {
                    symbol: "nv_cap_validate_and_dup_fd".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                PatchTarget {
                    symbol: "nv_cap_close_fd".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                // os_is_administrator calls capable(CAP_SYS_ADMIN) and returns
                // NvBool (1=admin, 0=not admin). RM checks this to decide
                // whether to enforce capability-based access control. Must
                // return 1 (admin) so RM sets pClient->bIsAdmin=true and
                // bypasses nv_cap_validate_and_dup_fd entirely. Previous
                // RetAtEntry returned 0 (not admin) which triggered the
                // cap validation path — that fails with our stubbed cap
                // system, causing INSUFFICIENT_PERMISSIONS (0x1b) on every
                // device_alloc with share=0.
                PatchTarget {
                    symbol: "os_is_administrator".into(),
                    strategy: PatchStrategy::Ret1AtEntry,
                },
                // Change the chardev major from 195 (0xc3) to 0 (dynamic
                // allocation) so nvsov gets its own chardev that doesn't
                // conflict with the host nvidia-580 module.
                // Layout: `mov $0xc3, %edi` at init_module+0x7a (5 bytes:
                // bf c3 00 00 00). We patch byte +0x7b (the immediate)
                // from 0xc3 to 0x00. The __register_chrdev call at +0x7f
                // remains intact and runs with major=0 → dynamic alloc.
                PatchTarget {
                    symbol: "init_module".into(),
                    strategy: PatchStrategy::PatchByteAt {
                        fn_offset: 0x7b,
                        expected: 0xc3,
                        replacement: 0x00,
                    },
                },
                // Force init_module to return 0 after chrdev registration.
                // __register_chrdev(0, ...) returns the assigned major (>0),
                // which init_module leaks as its return value. The kernel
                // rejects non-zero returns from init_module. Fix: change
                // `mov %ebx,%eax` (89 d8) at +0x8a to `xor %eax,%eax`
                // (31 c0), forcing return 0.
                PatchTarget {
                    symbol: "init_module".into(),
                    strategy: PatchStrategy::PatchByteAt {
                        fn_offset: 0x8a,
                        expected: 0x89,
                        replacement: 0x31,
                    },
                },
                PatchTarget {
                    symbol: "init_module".into(),
                    strategy: PatchStrategy::PatchByteAt {
                        fn_offset: 0x8b,
                        expected: 0xd8,
                        replacement: 0xc0,
                    },
                },
            ],
            min_applied: 1,
        }
    }

    /// Boot-services variant of the catalyst — uses nvidia RM as a UEFI-like
    /// boot service to initialize compute engines (ACR→FECS→GPCCS→TPC), then
    /// performs a clean unbind + PRI ring recovery post-swap.
    ///
    /// Key finding (Exp 221): PRI ring destruction happens in the kernel's PCI
    /// framework during unbind (PMC_ENABLE cleared), NOT in nv_pci_remove.
    /// RetAtEntry on nv_pci_remove was tried but only leaked iomem without
    /// preserving the PRI ring. Clean catalyst unbind + post-swap PRI ring
    /// re-enumeration is the correct approach.
    ///
    /// Same co-load isolation NOPs as catalyst, allowing full RM compute init.
    pub fn nvidia_boot_services() -> Self {
        // Identical to nvidia_catalyst_handoff — the "boot services" concept
        // is now about post-swap PRI ring recovery, not teardown NOP'ing.
        let mut ps = Self::nvidia_catalyst_handoff();
        ps.name = "nvidia_boot_services".into();
        ps
    }
}
