# Cross-Spring Absorption Tracker

**Date**: February 23, 2026 (Session 51)
**Sources**: hotSpring V067, neuralSpring V16/S48, wetSpring V16-V022, wateringHole standards

---

## HIGH PRIORITY

### H-001: GPU-Resident CG Shaders (hotSpring)

**Source**: `hotSpring/barracuda/src/lattice/shaders/`
**Impact**: 15,360x readback reduction, 30.7x HMC speedup

| Shader | Source Path | Target in barracuda |
|--------|-------------|---------------------|
| `sum_reduce_f64.wgsl` | `lattice/shaders/sum_reduce_f64.wgsl` | `ops/lattice/shaders/` |
| `cg_compute_alpha_f64.wgsl` | `lattice/shaders/cg_compute_alpha_f64.wgsl` | `ops/lattice/shaders/` |
| `cg_compute_beta_f64.wgsl` | `lattice/shaders/cg_compute_beta_f64.wgsl` | `ops/lattice/shaders/` |
| `cg_update_xr_f64.wgsl` | `lattice/shaders/cg_update_xr_f64.wgsl` | `ops/lattice/shaders/` |
| `cg_update_p_f64.wgsl` | `lattice/shaders/cg_update_p_f64.wgsl` | `ops/lattice/shaders/` |

**Status**: DONE (S51) -- 5 CG shaders + 7 SU3/fermion shaders copied to `shaders/lattice/`, wired via `include_str!()` in `cg.rs` + `absorbed_shaders.rs`

---

### H-002: GPU-Resident CG Rust Infrastructure (hotSpring)

**Source**: `hotSpring/barracuda/src/lattice/gpu_hmc.rs` (2595 lines)

| Component | Purpose |
|-----------|---------|
| `GpuResidentCgPipelines` | Pipeline cache for CG sub-operations |
| `GpuResidentCgBuffers` | GPU scalar buffers + bind groups |
| `build_reduce_chain()` | Multi-pass reduction construction |
| `AsyncCgReadback` | Double-buffered staging for batch overlap |
| `BidirectionalStream` | `std::sync::mpsc` for CPU+NPU routing |

**Target**: `crates/barracuda/src/ops/lattice/gpu_cg_resident.rs`
**Status**: PARTIAL (S51) -- Shaders absorbed and wired; Rust pipeline structs pending (need GPU for integration testing)

---

### H-003: neuralSpring gpu_ops Module (38 Promoted GPU Ops)

**Source**: `neuralSpring/src/gpu_ops/` (6 submodules) + `neuralSpring/src/gpu_dispatch.rs` (932 lines)

| Submodule | Ops |
|-----------|-----|
| `linalg` | matmul, transpose, frobenius_norm |
| `activation` | softmax, boltzmann |
| `reduction` | l2_distance, mean, variance, logsumexp |
| `bio` | hmm_forward/backward/viterbi, allele_frequencies, nucleotide_diversity |
| `population` | replicator_step, hill_activation_batch, geographic_distance_matrix |
| `eigensolver` | eigh_gpu, disorder_sweep_gpu |

**Target**: Wire into `barracuda::dispatch` or `barracuda::ops` as appropriate
**Status**: PENDING (S51 note: ops are neuralSpring-local dispatch wrappers around existing barracuda primitives; no new shaders needed, just dispatch routing)

---

### H-004: 7 Local WGSL Shaders (neuralSpring)

**Source**: `neuralSpring/metalForge/shaders/`

| Shader | Purpose | Validator |
|--------|---------|-----------|
| `head_split.wgsl` | MHA head splitting | `validate_mha_gpu` 10/10 |
| `head_concat.wgsl` | MHA head concatenation | `validate_mha_gpu` 10/10 |
| `xoshiro128ss.wgsl` | GPU PRNG | `validate_gpu_prng` 5/5 |
| `logsumexp_reduce.wgsl` | Log-domain reduction | `validate_gpu_logsumexp` 5/5 |
| `stencil_cooperation.wgsl` | Fermi imitation dynamics | `validate_gpu_stencil` 3/3 |
| `rk45_adaptive.wgsl` | Adaptive Dormand-Prince | `validate_gpu_rk45` 6/6 |
| `wright_fisher_step.wgsl` | Population drift+selection | `validate_gpu_wright_fisher` 4/4 |

**Target**: `crates/barracuda/src/shaders/` (categorized by domain)
**Status**: DONE (S51) -- 2 new shaders absorbed (`xoshiro128ss.wgsl`, `logsumexp_reduce.wgsl`); 5 already present (`head_split`, `head_concat`, `stencil_cooperation`, `rk45_adaptive`, `wright_fisher_step`)

---

### H-005: ESN NPU Weight Export (wetSpring)

**Source**: wetSpring V17/V18, `barracuda/src/bio/esn.rs`

| Item | Action |
|------|--------|
| `ESN::to_npu_weights()` | Add NPU weight export to `esn_v2::ESN` |
| `quantize_affine_i8` for `Vec<f64>` | Extend quantizer to accept f64 vectors directly |
| `NpuReadoutWeights` struct | Add to `esn_v2` module |

**Target**: `crates/barracuda/src/esn_v2/npu.rs`
**Status**: DONE (S51) -- `NpuReadoutWeights`, `quantize_affine_i8_f64()`, `dequantize_affine_i8_f64()`, `ESN::to_npu_weights()` added with 8 tests

---

### H-006: BatchedOdeRK4Generic (wetSpring)

**Source**: wetSpring V15/V18

Generic ODE solver replacing 5 local wetSpring WGSL shaders with a single reusable
integrator parameterized by `<N_VARS, N_PARAMS>`. Each spring supplies only the
derivative function; barracuda provides RK4 integration, dispatch, and f64 polyfill.

| Shader replaced | Variables | Parameters |
|-----------------|-----------|------------|
| `phage_defense_ode_rk4_f64.wgsl` | 4 | 11 |
| `bistable_ode_rk4_f64.wgsl` | 5 | 21 |
| `multi_signal_ode_rk4_f64.wgsl` | 7 | 24 |
| `cooperation_ode_rk4_f64.wgsl` | 4 | 13 |
| `capacitor_ode_rk4_f64.wgsl` | 6 | 16 |

**Target**: `crates/barracuda/src/numerical/ode_generic.rs`
**Status**: DONE (S51) -- `OdeSystem` trait + `BatchedOdeRK4<S>` with WGSL template generation + CPU integration + 6 tests

---

### H-007: solve_f64 CPU Fallback (hotSpring)

**Source**: hotSpring `md/reservoir.rs` (~465)

ESN matrices (50-200 dim) don't justify GPU overhead. Add `Option<Arc<WgpuDevice>>`
to `solve_f64` so callers can use CPU path when device is `None`.

**Target**: `crates/barracuda/src/linalg/solve.rs`
**Status**: DONE (S51) -- `solve_f64_cpu()` added with Gaussian elimination + partial pivoting, 5 tests

---

## MEDIUM PRIORITY

### M-001: Tensor API Extensions (neuralSpring)

| Method | Purpose | Source |
|--------|---------|--------|
| `argmax_dim(axis)` | Index-of-max for Viterbi | TOADSTOOL_HANDOFF L364 |
| `softmax_dim(axis)` | Row-wise softmax for attention | TOADSTOOL_HANDOFF L368 |

**Status**: PENDING

---

### M-002: Pseudofermion Gauge-Link Verification (hotSpring)

Verify `pseudofermion_force_f64.wgsl` uses `F = TA(U * M)` (link multiplication
before TA projection), not the incorrect `F = TA(M)`.

**Status**: VERIFIED (S51) -- Shader correctly uses `su3_mul(u, m_mat)` at line 127 before TA projection

---

### M-003: Conv2D/Pool WGSL Executor Wiring (D-S46-001)

Wire `Conv2D`, `MaxPool2D`, `AvgPool2D` shaders to `GpuExecutor` with stride/padding/channels/batch.
Enables full LeNet-5 GPU validation for neuralSpring.

**Status**: PENDING

---

### M-004: evolved/ MHA Retirement (neuralSpring)

Replace `mha_projection.wgsl` with `matmul` + `head_split.wgsl`.
Replace `mha_output.wgsl` with `head_concat.wgsl` + `matmul`.
Retire `evolved::mha` after full native MHA validation.

**Status**: PENDING (blocked on H-004 shader absorption)

---

### M-005: Root Re-exports (wetSpring)

Re-export at crate root to simplify deep import paths:
- `barracuda::ops::bio::quality_filter::QualityConfig`
- `barracuda::ops::bio::unifrac_propagate::UniFracConfig`

**Status**: DONE (S51) -- Both re-exported in `lib.rs`

---

### M-006: FlatTree Constructors (wetSpring)

Add `FlatTree::from_newick()` and `FlatTree::from_edges()` with automatic level computation.

**Status**: PENDING

---

### M-007: FusedMapReduceF64::dot(a, b) (wetSpring)

Add convenience dot-product method.

**Status**: DONE (S51) -- `dot(&self, a, b) -> Result<f64>` added using existing sum-of-products pattern

---

### M-008: ESN v2 Matrix Ridge Regression (wetSpring)

`W_out = Y * X^T * (X * X^T + lambda * I)^{-1}` for proper readout training.

**Status**: PENDING

---

### M-009: Mixed-Hardware Infrastructure (neuralSpring)

| Component | Source | Target |
|-----------|--------|--------|
| `MixedSubstrate` enum | `metalForge/mixed.rs` | `barracuda::unified_hardware` |
| `PcieBridge` + P2P | `metalForge/pcie_bridge.rs` | `barracuda::unified_hardware::transfer` |
| Dispatch heuristics | `metalForge/` | `barracuda::dispatch` |

**Status**: PENDING

---

### M-010: Tolerance Registry Pattern (neuralSpring)

24+ GPU validation tolerances in neuralSpring's `tolerances/registry.rs`.
Consider adding `barracuda::tolerances` module with centralized physical-justification constants.

**Status**: PENDING

---

## LOW PRIORITY

| ID | Item | Source |
|----|------|--------|
| L-001 | Screened Coulomb eigensolve (Sturm) | hotSpring `physics/screened_coulomb.rs` |
| L-002 | GPU ESN reservoir update shaders | wetSpring V17 |
| L-003 | `chi_squared_f64` test primitive | wetSpring V18 |
| L-004 | `GpuSession` builder API (pre-warmed) | wetSpring V16 |
| L-005 | Cross-spring provenance tags (`@origin`/`@absorbed`) | wetSpring V18 |
| L-006 | FST variance decomposition shader | neuralSpring PURE_GPU_ROADMAP |
| L-007 | Anderson transport/conductance primitives | wetSpring V20 |
| L-008 | NCBI data cache module | wetSpring V19 |
| L-009 | `swarm_nn_scores.wgsl` | neuralSpring |

---

## ALREADY RESOLVED (verified)

| Item | Resolved In |
|------|-------------|
| `Tensor::mean()` entry point fix (`mean_reduce`) | S49 -- confirmed in codebase |
| VACF batch GPU (`vacf_batch_f64.wgsl` + `VacfBatchGpu`) | S46 |
| Stress virial GPU | S46 |
| Heat current GPU | S49 |
| Pseudofermion CPU + GPU | S46-S48 |
| Dirac + CG vector ops (`complex_dot_re`, `axpy`, `xpay`) | S47-S48 |
| 5 ODE RK4 WGSL shaders (individual) | S46 |
| HMM forward f64 | S39 |
| 13 f32->f64 shader evolutions | S49 |
| WebSocket deprecation | S43 |
| gRPC stub -> Unix socket JSON-RPC | S43 |

---

## wateringHole Standards Compliance

| Standard | Status |
|----------|--------|
| Shader-first mandate | COMPLIANT -- 645+ WGSL f64, zero CPU-only math |
| ecoBin v2.0 | COMPLIANT -- pure Rust, cross-platform, XDG paths |
| Universal IPC v3.0 | COMPLIANT -- JSON-RPC 2.0 + tarpc, Unix/TCP transport |
| Capability discovery | COMPLIANT -- `compute.discover_capabilities` implemented |
| Semantic method naming | COMPLIANT -- `{domain}.{operation}` pattern |
| Zero hardcoded primal names | COMPLIANT -- capability-based discovery |

---

*Updated: February 23, 2026 -- Session 51*
*Next review: After HIGH items absorbed*
