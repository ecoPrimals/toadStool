# Active Technical Debt Register

**Date**: February 28, 2026
**Philosophy**: Math is universal, precision is silicon. Workarounds are
short-term solutions that increase debt. We aim to solve deep debt over
iterations, evolving toward vendor-agnostic, capability-based solutions.

---

## Active Workarounds

### W-001: f64 Transcendental Polyfills — Architectural Solution

**Status**: ACTIVE — Architecturally solved; polyfill is the sovereign solution
**Impact**: Enables f64 transcendentals on ALL GPUs regardless of vendor math library support

**Root Cause**: SPIR-V has no mechanism to link vendor math libraries (NVIDIA libdevice, AMD ocml).
Every f64 transcendental fails through SPIR-V on NVK/NAK, NVIDIA proprietary (Ada), and RADV.

**Solution**: `math_f64.wgsl` — 28 pure-WGSL polyfill functions (Cody-Waite range reduction,
Lanczos gamma, Horner polynomials). Auto-injected by `compile_shader_f64()`. No vendor
dependencies, works on every GPU, ships with the crate, testable in CI without hardware.

**Files**:
- `crates/barracuda/src/shaders/math/math_f64.wgsl` — 28 polyfill functions
- `crates/barracuda/src/shaders/precision/mod.rs` — `inject_missing_math_f64()`, `patch_transcendentals_in_code()`
- `crates/barracuda/src/device/wgpu_device/capabilities.rs` — `needs_f64_exp_log_workaround()`
- `crates/barracuda/src/device/probe.rs` — runtime capability probing, global cache

**F64 Built-in Capability Matrix** (probed Feb 18, 2026):

| Function     | RTX 3090 (Ampere) | RX 6950 XT (RDNA2) | Titan V (NVK/NAK) |
|-------------|-------------------|---------------------|-------------------|
| exp, log    | NATIVE            | fallback            | fallback          |
| sin, cos    | NATIVE†           | fallback            | TBD               |
| sqrt, fma   | NATIVE            | **NATIVE**          | TBD               |
| abs/min/max | NATIVE            | **NATIVE**          | TBD               |

†NVIDIA PTXAS sin/cos on f64 uses MUFU — likely f32 precision in f64 register.

**Evolution Path**:
1. DONE: Capability probing (`probe_f64_builtins()`) + fossil substitution
2. DONE: Fossil f64 functions (abs, sqrt, min, max, etc.) marked and auto-substituted
3. Upstream ACO fix: Contribute `fexp2(f64)` to Mesa ACO for RDNA2/3
4. Upstream NAK fix: Contribute `exp(f64)` lowering to Mesa NAK

---

### W-003: NAK Compiler 149x Performance Gap (Sovereign FP64 Compute)

**Status**: ACTIVE — Phases 1 + 4 done, pending Titan V hw validation
**Impact**: NVK/NAK Jacobi eigensolve ~9x slower than NVIDIA proprietary after warp-packing

**Phases**:

| # | Phase | Status |
|---|-------|--------|
| 1 | SM70 instruction latency tables | **DONE** — `sm70_instr_latencies.rs`, DFMA=8cy |
| 2 | f64 FMA selection (mul+add → DFMA) | Pending |
| 3 | Loop unrolling for bounded nested loops | Pending |
| 4 | Sovereign naga-IR FMA fusion + DCE | **DONE** — Phase 4 compiler |

**First solution absorbed**: Warp-packed eigensolve (`@workgroup_size(32,1,1)`) — 2.2x NVK speedup.
`GpuDriverProfile::optimal_eigensolve_strategy()` — data-driven strategy selection.

**Tracking**: https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/nouveau/compiler

---

## Remaining Debt

### Architecture

| ID | Description | Priority | Notes |
|----|-------------|----------|-------|
| D-CD | ComputeDispatch migration | High | 34/250 ops migrated (~3,739 lines removed). ~216 legacy ops use manual BGL/BG boilerplate. Incremental — each op is ~80 lines → ~5 lines. |
| D-DF64 | DF64 as default precision path | Medium | `df64_rewrite` as default, not fallback (groundSpring V35). Architectural decision. |
| D-NPU | NpuDispatch trait | Medium | Generic NPU interface — airSpring/wetSpring/groundSpring converge |
| D-COV | Test coverage ~82% → 90% | Medium | Barracuda near target. Gap: async networking, server lifecycle, deep protocol handlers. |

### DF64 Transcendental Coverage

Extend `df64_transcendentals.wgsl` to cover remaining functions:
- `asin_df64`, `acos_df64`, `atan_df64`, `atan2_df64`
- `sinh_df64`, `cosh_df64`
- `gamma_df64`, `erf_df64` (Lanczos/Abramowitz at DF64 precision)

### Sovereign Compiler Phase 4+

Phase 4 core is DONE (FMA fusion, DCE, SPIR-V passthrough). Remaining iterations:
- Register pressure estimation (live-range counting on naga expression arena)
- Loop software pipelining at naga IR level
- Architecture-specific peephole optimization per `GpuArch`
- naga → NAK IR direct bridge (research)

### Cross-Repo Debt

| ID | Description | Status |
|----|-------------|--------|
| D-S20-003 | neuralSpring `evolved/` migration (~2075 lines) | Awaiting neuralSpring team |
| D-S18-002 | cubecl transitive `dirs-sys` | Needs upstream PR |

### Lower Priority (Carried)

| ID | Description | Status |
|----|-------------|--------|
| D-S46-001 | Conv2D/Pool WGSL shader evolution (stride/padding/channels/batch) | GPU shaders exist, lack full parameter support |
| D-S18-003 | e2e, fhe, comprehensive pending integration tests | Require future APIs |

---

## Recently Resolved (S69++)

| Item | Resolution |
|------|-----------|
| metalForge streaming pipeline | `PipelineBuilder` → `StreamingPipeline` (staging/pipeline.rs) |
| manual_jsonrpc → pure_jsonrpc | Full migration — all handlers, Unix/TCP, unibin migrated |
| 4 production stubs | biome.rs (real validation), container benchmark (runtime detection), gRPC (deprecated), OpenCL (capability-based) |
| 16 large files | Smart-refactored to domain modules (all < 1000 lines) |
| 34 ComputeDispatch ops | 5 linalg + 15 special functions + 14 MD/bio/reduce (~3,739 lines removed) |
| NAK workgroup tuning | `workgroup_size_for_arch()` — Volta 64, Ada 256, RDNA 64, Intel Arc 128 |
| Hardcoded IPs | 6 production files → named constants |
| anyhow elimination | Fully eliminated from all ~30 workspace crates |
| rust-version 1.75→1.80 | `std::sync::LazyLock` stable |
| Dead code documented | All 18 unjustified `#[allow(dead_code)]` instances annotated |
| +100 new tests | naga validation, untested modules, staging, pure_jsonrpc, distributed, monitoring |
| Unsafe evolution | GPU memory bounds checks, SAFETY docs, `alloc_and_lock()` helper |
| chrono elimination | 28 crates, 200+ files → `std::time` |
| Unsafe 47→45 | `BorrowedFd` → safe `AsFd` in akida-driver |

## Previously Resolved

Full session-by-session resolution history is in [CHANGELOG.md](CHANGELOG.md).

Key milestones:
- **S68**: Dual-layer universal precision (`op_preamble` + `df64_rewrite`), 122 shader tests
- **S66**: Cross-spring absorption wave (airSpring V009 + groundSpring V7), 707 shaders classified
- **S61**: Sovereign Compiler Phase 4 (naga-IR FMA fusion, DCE, SPIR-V passthrough)
- **S60**: DF64 FMA optimization (`two_prod` Dekker→`fma`), DF64 transcendentals, 4 force shaders all-DF64
- **S50**: Coverage push 73%→84%, hardcoded ports/URLs eliminated, mock evolution, cargo-deny
- **S25**: GPU FFT f64 validation, error system deep debt
- **S21**: wetSpring bio GPU primitives (Smith-Waterman, Gillespie SSA, decision tree, Felsenstein)
- **S14-20**: neuralSpring 11-shortcoming absorption, TensorSession ML ops, chrono/futures/dashmap eliminated
- **S5-13**: Coverage sprints, sleep elimination, sovereign compiler phases 1-3

---

*Debt is tracked, not ignored. Each workaround has an evolution path.*
*The goal is zero workarounds — vendor-agnostic, capability-based code.*
