// SPDX-License-Identifier: AGPL-3.0-or-later
use super::super::types::{PatchSet, PatchStrategy, PatchTarget};

impl PatchSet {
    /// Patch set for Volta (GV100) warm handoff via nouveau.
    ///
    /// NOPs teardown functions that power-gate GPCs and clock-gate engines
    /// on unbind. With these patched, `rmmod nouveau` preserves PMC_ENABLE,
    /// GPC broadcast routing fabric, FECS microcode, and TPC power state.
    ///
    /// Exp 215 identified that the original 5-target set preserved GPC fabric
    /// but TPCs remained power-gated (0xBADF5040 at per-TPC registers).
    /// Added clock gate teardown functions that control BLCG/SLCG/ELPG
    /// power domains within GPCs.
    ///
    /// # `gf100_gr_fini` is deliberately NOT patched
    ///
    /// It was NOPed until biomeGate/kernel 7.0 (nouveau 1.4.2), where it
    /// caused a kernel oops rather than the teardown hang it was meant to
    /// avoid. On this nouveau, `gf100_gr_fini` is what releases the FECS
    /// falcon. NOPed, the falcon is never released and GR's next init cycle
    /// fails:
    ///
    /// ```text
    /// gr: fini failed, -1028627904
    /// gr: fecs falcon already acquired by gr!
    /// gr: init failed, -16                      <- EBUSY
    /// ```
    ///
    /// Teardown then moves buffer objects with a copy engine that never came
    /// up, and page-faults in `nve0_bo_move_copy` under `nouveau_ttm_fini`.
    /// The oops kills the session and leaks nouveau's refcount, leaving a
    /// zombie module that not even a forced `delete_module` can remove.
    ///
    /// Losing GR state on unbind is an acceptable trade: it degrades the
    /// achievable tier, which classification reports honestly, instead of
    /// crashing the kernel.
    #[must_use]
    pub fn volta_warm_handoff() -> Self {
        Self {
            name: "volta_warm_handoff".into(),
            module_name: "nouveau".into(),
            targets: vec![
                PatchTarget {
                    symbol: "nvkm_pmu_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "nvkm_mc_disable".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "nvkm_mc_reset".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "gk104_fifo_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                // Exp 215: clock gate teardown — preserve TPC power domains.
                // Uses RetAtEntry because RetAfterFtrace hits kernel
                // relocation checks on these functions (byte+5 has an
                // R_X86_64_PLT32 relocation entry).
                PatchTarget {
                    symbol: "gk104_clkgate_fini".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "nvkm_therm_clkgate_fini".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
                PatchTarget {
                    symbol: "g84_therm_fini".into(),
                    strategy: PatchStrategy::RetAtEntry,
                },
            ],
            min_applied: 1,
        }
    }

    /// Patch set for Kepler (GK210 / K80) warm handoff via nouveau.
    ///
    /// Kepler has unsigned falcons so nouveau can fully initialize FECS.
    /// These patches preserve the initialized state across unbind.
    ///
    /// `gf100_gr_fini` is omitted for the same reason as the Volta set: it
    /// owns the FECS falcon release, and NOPing it turns a teardown into a
    /// kernel page fault. Kepler shares this `gf100_*` code path.
    #[must_use]
    pub fn kepler_warm_handoff() -> Self {
        Self {
            name: "kepler_warm_handoff".into(),
            module_name: "nouveau".into(),
            targets: vec![
                PatchTarget {
                    symbol: "nvkm_pmu_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "nvkm_mc_disable".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "nvkm_mc_reset".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
                PatchTarget {
                    symbol: "gk104_fifo_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
            ],
            min_applied: 1,
        }
    }
}
