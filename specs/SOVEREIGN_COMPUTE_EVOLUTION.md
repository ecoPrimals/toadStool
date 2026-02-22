# Sovereign Compute Evolution — BarraCuda WGSL Optimizer & Mycelial ToadStool

**Date**: February 18, 2026
**Status**: Vision formalised — Phase 0 absorbed (fossils, NAK latency tables), Phase 1 active
**Classification**: Core architecture evolution

---

## The Vision

ToadStool should be as **ubiquitous as fungus in nature** — spreading through every
computational substrate (GPU, NPU, CPU, embedded, edge, cloud, WASM) just as mycelium
threads through soil. Not a single monolith that requires careful installation, but a
spore that germinates wherever Rust can run.

BarraCuda is the **unified math language** of that organism — one set of WGSL shaders,
one f64-first numeric stack, running identically on NVIDIA, AMD, Intel, Apple, WebGPU.

The sovereign endpoint: **zero dependency on any vendor SDK for correctness or
performance**. Rust builds it. WGSL describes it. Vulkan/wgpu runs it. The compiler
optimises it — but we don't *need* the compiler to be smart. We write smart.

---

## What the NAK Investigation Taught Us

The 149× gap between NVK/NAK and PTXAS for the Jacobi eigensolve was not a silicon
gap. The RTX 3090 and the Titan V have the same FP64 execution units whether you use
the open-source or proprietary driver. The gap was **scheduling information** — PTXAS
knew that `DFMA` takes 8 cycles; NAK assumed 4.

That revealed three sovereign principles:

### Principle 1 — We are upstream of every compiler

NAK operates at the SPIR-V → machine-code boundary.
BarraCuda operates at the WGSL → naga → SPIR-V boundary.
**Anything NAK's scheduler can do to SPIR-V, we can express directly in WGSL
and it will work on every backend: NVK, RADV/ACO, PTXAS, Metal, DX12, WebGPU.**

We do not need NAK to improve. We can write latency-hiding WGSL today.

### Principle 2 — Scoreboards are universal

A GPU scoreboard tracks "this register is in-flight, stall until it resolves."
Every vendor has one (physical or virtual). The universal counter-measure is
**instruction-level parallelism (ILP)** — place independent instructions in the
latency window so the hardware never needs to stall.

This is not NVIDIA-specific. It is physics.

| Architecture | Physical mechanism | Response to WGSL ILP |
|---|---|---|
| SM70 Volta (DFMA 8cy) | Per-register scoreboard | Fills 8-cycle gap with independent ops |
| RDNA2 ACO (VFMA64 ~4cy) | VALU dependency tracking | Same — 4-cycle gap |
| Apple M2 GPU | Internal dep tracking | Same |
| Intel Xe | EU scoreboard | Same |

### Principle 3 — Latency tables are public knowledge

arXiv:1804.06826 (Volta), AMD RDNA3 ISA docs, Intel Xe ISA — all publicly available.
We can build accurate `LatencyModel` implementations for every major GPU family without
waiting for vendor disclosure or compiler work.

---

## Architecture: BarraCuda WGSL Optimizer

```
User WGSL shader
        │
        ▼
┌─────────────────────────────────────────────┐
│  ShaderTemplate (existing)                  │
│   · fossil substitution                     │
│   · missing function injection              │
│   · driver exp/log workaround               │
│         │                                   │
│         ▼  NEW ──────────────────────────── │
│  WgslOptimizer                              │
│   · dependency graph (let-binding analysis) │
│   · ILP reordering (LatencyModel-guided)    │
│   · loop unrolling (bounded loops ≤ 32)     │
│   · register pressure estimation            │
│         │                                   │
└─────────┼───────────────────────────────────┘
          │
          ▼
   Optimised WGSL  (already scheduled — compiler is irrelevant)
          │
          ▼
   naga → SPIR-V → Vulkan driver (NVK/ACO/PTXAS/Metal)
          │
          ▼
   GPU hardware  (no scoreboard stalls)
```

### Component: `LatencyModel` trait

```rust
pub trait LatencyModel: Send + Sync {
    /// Cycles between write and first valid read (read-after-write).
    fn raw_latency(&self, op: WgslOpClass) -> u32;
    /// Cycles between read and next write of same register (write-after-read).
    fn war_latency(&self, op: WgslOpClass) -> u32;
    /// Whether this op uses a scoreboard vs fixed pipeline.
    fn needs_scoreboard(&self, op: WgslOpClass) -> bool;
}

pub enum WgslOpClass {
    F64Fma,        // DFMA on SM70: 8cy; VFMA64 on RDNA2: ~4cy
    F64MulAdd,     // sequential mul+add: 2× raw latency
    F64Transcend,  // exp_f64, log_f64: ~20cy software
    F32Fma,        // FFMA on SM70: 4cy
    I32Arith,      // IADD/IMAD: 2-6cy (useful for dual-issue pairing)
    SharedMem,     // SMEM load: 20-30cy
    GlobalMem,     // GMEM load: 200-800cy
}

// Concrete models (pure data — no unsafe, no C, no vendor SDK)
pub struct Sm70LatencyModel;    // Volta: DFMA=8cy, FFMA=4cy (arXiv:1804.06826)
pub struct Rdna2LatencyModel;   // RDNA2: VFMA64=~4cy (AMD ISA docs)
pub struct ConservativeModel;   // Safe fallback: use maximum observed latency
pub struct MeasuredModel {      // From bench_f64_builtins results
    pub dfma_cycles: u32,
    pub ffma_cycles: u32,
}
```

### Component: `WgslDependencyGraph`

A lightweight, single-pass analysis of a WGSL function body:
1. Scan `let` bindings in order; assign each a node
2. For each node, record which earlier bindings it references → edges
3. Build a topological sort that maximises distance between a def and its
   first use, subject to the `LatencyModel` for the op that produces it
4. Emit reordered WGSL

This is **not** a full compiler IR. It handles the 80% case: sequences of
`let` bindings in straight-line code inside inner loops. Branches and loops
are left as-is (conservative). The Jacobi rotation kernel is exactly this
pattern.

### Component: `WgslLoopUnroller`

For loops with a statically bounded trip count (≤ 32, discoverable from a
`// @unroll_hint N` annotation or a `for i in 0u32..N` where `N` is a
shader constant), emit the loop body `N` times. This:
- Eliminates the loop counter dependency chain
- Exposes all iterations to the dependency reorderer simultaneously
- Enables inter-iteration ILP (iteration i+1's independent ops fill
  iteration i's latency gaps)

The Jacobi sweep has `for k in 0u32..n` where `n ≤ 32` for typical science
matrices. Unrolling this is the single highest-impact change possible.

---

## Evolution Phases

### Phase 0 — Absorbed ✅ (Feb 18, 2026)

- [x] f64 fossil functions removed — `abs`, `sqrt`, `min`, `max`, `floor`, `ceil`,
      `round`, `fract`, `sign`, `clamp` → native WGSL builtins
- [x] `F64BuiltinCapabilities` probe — per-GPU capability matrix at runtime
- [x] `for_driver_auto()` comment-aware — exp/log replacement doesn't corrupt comments
- [x] SM70 latency tables contributed to NAK (`sm70_instr_latencies.rs`) —
      DFMA=8cy corrected, WAR/WAW per-category, `needs_scoreboards()` accurate

**Outcome**: Foundation clean. We know the latency numbers. We know which
ops are native on which GPUs. The shader injection pipeline is correct.

---

### Phase 1 — Manual ILP in Critical Shaders (Active, Feb 2026)

Target: Jacobi eigensolve (`batched_eigh_single_dispatch_f64.wgsl`)
This is the primary hotSpring bottleneck and the subject of the 149× gap.

**Changes**:
1. Restructure the rotation kernel to interleave independent computations
   in the 8-cycle DFMA latency window on SM70:

```wgsl
// Before — stalls every 8cy waiting for DFMA result:
let c = cos_val;
let s = sin_val;
let new_p = c * a_kp - s * a_kq;   // BLOCKED: waits 8cy for c, s
let new_q = s * a_kp + c * a_kq;   // BLOCKED: waits for s

// After — independent ops fill the gap, zero stalls:
let c    = cos_val;
let s    = sin_val;
// Independent quantities computed during DFMA latency window:
let cc   = c * c;
let ss   = s * s;
let cs   = c * s;
let diff = cc - ss;    // will be used later, computed now
let sum2 = 2.0 * cs;   // same
// NOW use c, s — scoreboard already cleared:
let new_p = c * a_kp - s * a_kq;
let new_q = s * a_kp + c * a_kq;
```

2. Add `// @unroll_hint 32` annotation to the inner sweep loop
3. Validate on Titan V (SM70) + RX 6950 XT (RDNA2) with `bench_wgsize_nvk`

**Success criteria**: ≥ 3× speedup on Titan V, neutral or positive on NVIDIA
proprietary (PTXAS already does this — we're matching it at source level).

---

### Phase 2 — `LatencyModel` Trait + Measured Models (Q1 2026)

**What to build**:
- `pub trait LatencyModel` in `crates/barracuda/src/device/capabilities.rs`
- `Sm70LatencyModel`, `Rdna2LatencyModel`, `ConservativeModel` implementations
- `GpuDriverProfile::latency_model() -> Box<dyn LatencyModel>` method
- `MeasuredModel`: constructed from `bench_f64_builtins` output, stored
  in `F64BuiltinCapabilities` (already probed at runtime)

**Files**:
- `crates/barracuda/src/device/latency.rs` (new)
- `crates/barracuda/src/device/capabilities.rs` (extend)

**Outcome**: Every `GpuDriverProfile` can answer "how many cycles does an
f64 FMA take on this specific GPU?" — grounded in measurement, not assumption.

---

### Phase 3 — `WgslDependencyGraph` + Reorderer (Q2 2026)

**What to build**:
- `crates/barracuda/src/shaders/optimizer/mod.rs` (new crate module)
- `WgslDependencyGraph` — parses `let` bindings, builds DAG
- `IlpReorderer` — topological sort guided by `LatencyModel`
- `WgslLoopUnroller` — unrolls annotated bounded loops
- Integration into `ShaderTemplate::for_driver_auto()`

**Approach** — do NOT build a full WGSL parser. Instead:
- Operate on the `// @ilp_region begin ... // @ilp_region end` annotated
  sections of shaders (opt-in per shader, starting with Jacobi)
- Parse only `let name = expr;` forms within annotated regions
- Everything outside annotations is passed through unchanged

This limits scope to a few hundred lines of Rust while covering the
critical paths.

**Success criteria**: Jacobi eigensolve achieves within 2× of PTXAS on
Titan V without any manual ILP annotation — the reorderer does it
automatically from the `@ilp_region` annotation alone.

---

### Phase 4 — Full WGSL Source Optimizer (Q3 2026)

**What to build**:
- Full SSA-form WGSL function analysis (using `naga`'s AST as input —
  it already parses WGSL for us, no need to reinvent)
- Per-function dependency graph over naga's typed IR
- Global register pressure estimation
- Loop software pipelining (preload iteration `i+1` data during `i`'s ops)
- Emit naga IR directly (bypass WGSL text → SPIR-V, reduce round-trips)

**Why naga** — naga is already a dependency (it's wgpu's shader compiler).
We can drive it as a library: parse WGSL → get typed IR → transform IR →
re-emit IR → hand to naga's SPIR-V backend. No new parser to write.

**Outcome**: The sovereign compiler layer is complete. Any WGSL shader
written for BarraCuda is automatically optimised for the target GPU before
the vendor driver ever sees it. NAK/ACO/PTXAS receive pre-scheduled SPIR-V
and simply translate it 1-to-1 to machine code.

---

### Phase 5 — Vendor-Agnostic Math Completeness (Q3-Q4 2026)

Close the remaining gap in `math_f64.wgsl`:
- `sin_f64` / `cos_f64`: add Cody-Waite range reduction + minimax polynomial
  (currently software; native sin/cos on f64 are MUFU on NVIDIA — f32 precision)
- `atan2_f64`, `asin_f64`, `acos_f64`: missing transcendentals
- `lgamma_f64` full range: current implementation is asymptotic only
- Verify bit-exact results vs `libm` reference on every new function

**Test framework**: fuzz-test every new math function against `libm` via
`cargo test --release -p barracuda` — 10M random inputs, require ULP ≤ 1.

---

## The Mycelial Model

In nature, a fungal network (mycelium) has no centre. Each node is capable
of independent operation, shares resources with adjacent nodes, and routes
signals through the most available path. When one thread dies, the network
routes around it. When new substrate becomes available, it colonises it.

ToadStool's sovereign endpoint mirrors this:

```
                    ┌──────────────────────────────────────┐
                    │          biomeOS / NUCLEUS           │
                    │  BearDog  Songbird  NestGate  Tower  │
                    └──────────────┬───────────────────────┘
                                   │ discovers at runtime
                    ┌──────────────┼───────────────────────┐
                    │              ▼                        │
     ┌──────────┐   │   ┌──────────────────┐               │
     │  GPU     │◄──┼──►│   ToadStool      │◄──────────────┼── any compute node
     │ (wgpu)   │   │   │   Node Primal    │               │   (any OS, any arch)
     └──────────┘   │   │   · JSON-RPC 2.0 │               │
     ┌──────────┐   │   │   · tarpc        │               │
     │  NPU     │◄──┼──►│   · Unix sockets │               │
     │ (Akida)  │   │   │   · BarraCuda    │               │
     └──────────┘   │   └──────────────────┘               │
     ┌──────────┐   │            │                          │
     │  CPU     │◄──┘            │ self-describes           │
     │ (WASM)   │                ▼                          │
     └──────────┘   ┌──────────────────────┐               │
                    │  BarraCuda           │               │
                    │  WGSL Math Engine    │               │
                    │  · 480+ shaders      │               │
                    │  · WgslOptimizer     │               │
                    │  · LatencyModel      │               │
                    │  · Any GPU via wgpu  │               │
                    └──────────────────────┘               │
                                                           │
                    (identical binary on ARM/x86/WASM) ────┘
```

**What "fungal" means technically**:
- Single Rust binary, `#[no_std]`-compatible core, cross-compiled to any target
- ToadStool spawns on a new node, discovers its hardware, announces itself to
  the ecosystem via mDNS or Songbird — no config file needed
- BarraCuda shaders compile at first-run on the target GPU via wgpu's Vulkan
  backend — no pre-compiled binaries needed
- The `WgslOptimizer` adapts to the GPU it finds — `bench_f64_builtins` runs
  once, populates `MeasuredLatencyModel`, subsequent shaders are pre-scheduled
- Jobs arrive via JSON-RPC; results leave the same way; no shared state

---

## Cross-Vendor Latency Models (Target State)

| GPU Family | Architecture | DFMA/FMA64 | FFMA | ILP Fill Strategy | Status |
|---|---|---|---|---|---|
| SM70 (Titan V, V100) | Volta | 8cy | 4cy | 8 independent ops | ✅ Phase 0 done |
| SM75 (RTX 2080) | Turing | 8cy | 4cy | same | Reference exists in NAK |
| SM80 (A100) | Ampere | 8cy | 4cy | same | NAK sm80 tables exist |
| SM86 (RTX 3090) | Ampere | 8cy | 4cy | same | Proprietary — validated |
| SM89 (RTX 4070, 4090) | Ada | 8cy | 4cy | same | Proprietary — to measure |
| RDNA2 (RX 6950 XT) | GCN6 | ~4cy | ~4cy | 4 ops | Empirical via bench |
| RDNA3 (RX 7000) | GCN7 | ~4cy | ~4cy | 4 ops | To measure |
| Apple M2 GPU | Apple | ~4cy | ~4cy | TBD | To measure |
| Intel Xe | Xe | ~4cy | ~4cy | TBD | To measure |

`bench_f64_builtins` is the measurement tool for all unknown entries.

---

## What We Contribute Back to NAK

We are not in competition with NAK — we are parallel contributors working
at different abstraction levels. What we share back:

1. **SM70 latency tables** (Phase 0, done) — `sm70_instr_latencies.rs`
   with corrected DFMA=8cy and per-category WAR/WAW
2. **Empirical validation** — `bench_wgsize_nvk` results on Titan V showing
   the scheduling gap before and after Phase 1
3. **Phase 2-4 roadmap** — f64 FMA selection, loop unrolling, dual-issue;
   provided as Mesa MR with benchmarks

The timeline: BarraCuda's WGSL optimizer makes our applications fast
regardless of NAK's state. As NAK improves, our applications get faster
without changes. Both paths converge on the same hardware target.

---

## Relationship to Existing Specs

| Spec | Relationship |
|------|-------------|
| `FP64_GPU_EVOLUTION.md` | Phase 0 complete — fossil functions, capability matrix |
| `BARRACUDA_PARITY_ROADMAP.md` | Performance parity via WGSL ILP is the next tier |
| `CROSS_VENDOR_BENCHMARK_SPEC.md` | `bench_f64_builtins` feeds `MeasuredLatencyModel` |
| `PRIMAL_CAPABILITY_SYSTEM.md` | ToadStool node discovery — the mycelial network foundation |
| `NAK_CONTRIBUTION_PLAN_FEB18_2026.md` | Upstream NAK work (parallel, not dependent) |

---

*Sovereign compute means: we write the math, any hardware runs it, no vendor
can take it away. That is BarraCuda's contract with ToadStool.*
