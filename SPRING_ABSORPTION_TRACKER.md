# Spring Absorption Tracker

**Session**: S88 (March 2, 2026)
**ToadStool**: master, BarraCUDA 766+ WGSL shaders, 2,866+ tests

## Spring Pin Status

| Spring | Version | ToadStool Pin | Tests | Delegations |
|--------|---------|---------------|-------|-------------|
| hotSpring | v0.6.17 | S80→S87 | 660 | Lattice QCD, MD reservoir, NPU |
| groundSpring | V68 | S86→S87 | 776 | 76 (44 CPU + 32 GPU) |
| neuralSpring | V75/S113 | S86→S87 | 861 | 68 validate_modern |
| wetSpring | V92F | S86→S87 | 1,089 | 144 primitives |
| airSpring | V063 | S79→S87 | 813 | 25 Tier A GPU |

## Absorption Status

### P0 — Correctness (S88)

| Item | Source | Status |
|------|--------|--------|
| `anderson_4d` + `wegner_block_4d` re-export | groundSpring V68 | **DONE** |
| `SeasonalGpuParams::new()` constructor | groundSpring V68 | **DONE** |
| `BREAKING_CHANGES.md` | groundSpring, wetSpring | **DONE** |
| Feature-gate CI (`cargo check` without features) | wetSpring, groundSpring | **DONE** |

### P1 — API Gaps (S88)

| Item | Source | Status |
|------|--------|--------|
| `MultiHeadEsn::from_exported_weights()` | hotSpring | **DONE** |
| Cross-spring named tolerances expansion | airSpring, wetSpring | **DONE** |
| `NeighborMode` 4D index convention docs | hotSpring | **DONE** |
| SU(3) DF64 shader sync verification | hotSpring | **DONE** (already absorbed) |

### P2 — Shader Evolution (S88)

| Item | Source | Status |
|------|--------|--------|
| L-BFGS GPU (batched numerical gradient) | groundSpring | **DONE** |
| Tridiag QL eigenvector solver | groundSpring | **DONE** |

### P3 — Future Evolution (tracked)

| Item | Source | Status |
|------|--------|--------|
| Flash attention shader | neuralSpring | Tracked |
| Fused LayerNorm+GELU | neuralSpring | Tracked |
| Deformed HFB full wiring (5 WGSL) | hotSpring | Tracked |
| Abelian Higgs U(1)+Higgs (3 WGSL) | hotSpring | Tracked |
| Richards PDE GPU full wiring | airSpring, groundSpring | Tracked |
| Multi-GPU interconnect | neuralSpring | Tracked |
| BatchedEncoder for SpectralNautilusBridge | neuralSpring | Tracked |
| Batched Brent GPU with custom closures | groundSpring | Tracked |

## Cross-Spring Patterns

| Pattern | Springs | Resolution |
|---------|---------|------------|
| GPU-resident state (no readback) | airSpring, groundSpring, hotSpring | `BatchedStatefulF64` exists; needs docs |
| Breaking changes tracking | groundSpring, wetSpring | `BREAKING_CHANGES.md` created |
| Feature-gate discipline | wetSpring, groundSpring | CI check added |
| Fused pipeline chains | airSpring, wetSpring | `UnidirectionalPipeline` exists |
| Shared named tolerances | airSpring, wetSpring | Expanded S88 |
| Provenance tags | hotSpring, groundSpring, neuralSpring | Future (P3) |

## Handoff Cross-Reference

### Incoming (to ToadStool)

| Handoff | From | Key Items |
|---------|------|-----------|
| `HOTSPRING_V0617_TOADSTOOL_S80_ABSORPTION` | hotSpring | Pseudofermion HMC, MultiHeadEsn, NeighborMode |
| `GROUNDSPRING_TOADSTOOL_V68_COMPREHENSIVE` | groundSpring | L-BFGS GPU, tridiag QL, SeasonalGpuParams, anderson_4d |
| `NEURALSPRING_TOADSTOOL_V75_S113_CROSS_SPRING` | neuralSpring | Flash attention, BatchedEncoder |
| `WETSPRING_TOADSTOOL_V92F_BARRACUDA` | wetSpring | BatchIprGpu n~2000, feature-gate |
| `AIRSPRING_TOADSTOOL_ABSORPTION` | airSpring | BatchedStatefulF64 wiring, seasonal chain |
| `AIRSPRING_V047_GPU_PIPELINE_EVOLUTION` | airSpring | Tier B shaders |

### Outgoing (from ToadStool)

| Handoff | To | Session |
|---------|----|---------|
| `TOADSTOOL_BARRACUDA_S87_DEEP_DEBT_EVOLUTION` | Ecosystem | S87 |
| `TOADSTOOL_BARRACUDA_S88_SPRING_ABSORPTION` | Ecosystem | S88 |
