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
    #[must_use]
    pub fn volta_warm_handoff() -> Self {
        Self {
            name: "volta_warm_handoff".into(),
            module_name: "nouveau".into(),
            targets: vec![
                PatchTarget {
                    symbol: "gf100_gr_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
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
    #[must_use]
    pub fn kepler_warm_handoff() -> Self {
        Self {
            name: "kepler_warm_handoff".into(),
            module_name: "nouveau".into(),
            targets: vec![
                PatchTarget {
                    symbol: "gf100_gr_fini".into(),
                    strategy: PatchStrategy::RetAfterFtrace,
                },
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
