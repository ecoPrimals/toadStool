# Sovereign Compute — Evolution Tracker

**Vision**: ToadStool as ubiquitous as fungus. BarraCUDA as universal math.
Pure Rust. Any hardware. No vendor lock.

**Full spec**: [`specs/SOVEREIGN_COMPUTE_EVOLUTION.md`](specs/SOVEREIGN_COMPUTE_EVOLUTION.md)

---

## The North Star

```
Today:    WGSL → naga → SPIR-V → vendor compiler → GPU
                                   ^^^^^^^^^^^^^^^^^^^
                                   We depend on this being good

Sovereign: WGSL → BarraCUDA WgslOptimizer → pre-scheduled SPIR-V → vendor → GPU
                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                             We own this. Any GPU. Any vendor. Forever.
```

When the optimizer is complete:
- NAK improving makes us faster, but NAK **not** improving doesn't hurt us
- PTXAS staying proprietary doesn't matter — we pre-schedule at source level
- A new GPU vendor appears — add a `LatencyModel` + `bench_f64_builtins` run → done
- ToadStool node spawns on unknown hardware → probes → adapts → computes

---

## Phase Status

| Phase | Description | Status | Completed |
|-------|-------------|--------|-----------|
| **0** | Fossil functions removed, NAK SM70 latency tables, capability probe | ✅ Done | Feb 18, 2026 |
| **1** | Manual ILP in Jacobi kernel — `@ilp_region` restructure, warp-packing | ✅ Done | Feb 18, 2026 |
| **2** | `LatencyModel` trait, `Sm70Model`, `Rdna2Model`, `ConservativeModel`, `MeasuredModel` | ✅ Done | Feb 19, 2026 |
| **3** | `WgslDependencyGraph` + `IlpReorderer` + `WgslLoopUnroller` wired into `ShaderTemplate` | ✅ Done | Feb 19, 2026 |
| **4** | Full naga-IR optimizer — SSA form, register pressure, loop pipelining | 📋 Planned | Q3 2026 |
| **5** | `math_f64.wgsl` completeness — sin/cos/atan2/asin/acos full range, libm fuzz | 📋 Planned | Q3–Q4 2026 |

---

## Phase 0 — Done ✅

**What we learned**: The 149× NAK/PTXAS gap is a scheduling gap, not a silicon gap.
DFMA was assumed 4cy; reality is 8cy. The fix is to fill those 8 cycles with
independent operations — and we can do that **at the WGSL source level**, across
every GPU vendor, without waiting for any compiler to improve.

**What was built**:
- `sm70_instr_latencies.rs` — Volta latency table (8cy DFMA, 4cy FFMA, WAR/WAW per-category)
- f64 fossil functions removed from `math_f64.wgsl` (abs, sqrt, min, max, floor, ceil, round, fract, sign, clamp)
- `F64BuiltinCapabilities` probe — per-GPU capability matrix at runtime
- `substitute_fossil_f64()` — auto-upgrades legacy shader calls to native WGSL
- `for_driver_auto()` comment-aware — exp/log workaround doesn't corrupt shader source

---

## Phase 1 — Done ✅

**Target**: Jacobi eigensolve (`batched_eigh_single_dispatch_f64.wgsl`)

**What was built**:
- Rotation kernel restructured for ILP: `cc = c*c`, `ss = s*s`, `two_cs = 2*c*s`
  hoisted before the per-element loop, filling the 8-cycle DFMA window
- A and V rotations interleaved inside the inner loop — independent ops fill stalls
- `@ilp_region begin/end` annotations added — mark regions for Phase 3 reorderer
- `@workgroup_size(32, 1, 1)` warp-packing (measured 2.2× NVK speedup)
- `// @unroll_hint 32` annotation on the inner sweep loop

**Files**: `crates/barracuda/src/shaders/linalg/batched_eigh_single_dispatch_f64.wgsl`

---

## Phase 2 — Done ✅

**Target**: `LatencyModel` trait in `crates/barracuda/src/device/latency.rs`

**What was built**:
```
crates/barracuda/src/device/latency.rs
  pub trait LatencyModel              ← raw_latency(), war_latency(), needs_scoreboard()
  pub struct Sm70LatencyModel         ← DFMA=8cy, FFMA=4cy (arXiv:1804.06826)
  pub struct Rdna2LatencyModel        ← VFMA64=~4cy (AMD ISA docs + empirical)
  pub struct ConservativeModel        ← safe maximum fallback (unknown GPUs)
  pub struct MeasuredModel            ← populated from bench_f64_builtins probe
  pub fn model_for_arch(GpuArch)      ← dispatch helper

crates/barracuda/src/device/capabilities.rs
  impl GpuDriverProfile {
    pub fn latency_model(&self) -> Box<dyn LatencyModel>
  }
```

7 unit tests covering all four models and the arch dispatch function.

---

## Phase 3 — Done ✅

**Target**: `WgslOptimizer` in `crates/barracuda/src/shaders/optimizer/`

**What was built**:
```
crates/barracuda/src/shaders/optimizer/
  mod.rs                ← WgslOptimizer::optimize(), for_arch(), Default (Conservative)
  dependency_graph.rs   ← WgslDependencyGraph: parse() let-binding DAG, classify_op()
  ilp_reorderer.rs      ← IlpReorderer: ASAP list scheduling (BinaryHeap, release_cycle)
  loop_unroller.rs      ← WgslLoopUnroller: @unroll_hint N, word-boundary substitution
```

24 unit tests. `ShaderTemplate::for_driver_auto()` wired — every compiled shader
passes through the optimizer automatically. `for_driver_profile()` added for
hardware-accurate scheduling via `GpuDriverProfile::latency_model()`.

**Annotation syntax in WGSL**:
```wgsl
// @ilp_region begin
let c  = cos_val;                    // FP64 FMA — 8cy latency on SM70
let s  = sin_val;                    // independent: scheduler may reorder
let cc = c * c;                      // dep on c only
let ss = s * s;                      // dep on s only — independent of cc
let new_p = c * a_kp - s * a_kq;    // dep on c, s — scheduled after gap fills
// @ilp_region end

// @unroll_hint 8
for (var k = 0u; k < 8u; k = k + 1u) {
    // unrolled 8× with literal k=0..7
}
```

**Mesa contribution patches prepared**:
- `contrib/mesa-nak/sm70_instr_latencies.rs` — SM70/Turing/Ampere/Ada match arm
- `contrib/mesa-nak/rdna2_instr_latencies.rs` — RDNA2/RDNA3 ACO entries

---

## Phase 4 — Planned 📋

**Target**: Full naga-IR optimizer (Q3 2026)

**Key insight**: naga (wgpu's shader compiler) already parses WGSL for us.
Drive it as a library:
```
WGSL text → naga::parse() → naga::Module (typed IR)
         → BarraCUDA IR passes (reorder, unroll, pipeline)
         → modified naga::Module
         → naga::back::spv::write() → SPIR-V bytes
         → wgpu device (bypasses WGSL text entirely)
```

No new parser. Full SSA form. Register pressure estimation.
Inter-iteration loop pipelining (preload iteration i+1 data during iteration i's ops).

---

## Phase 5 — Planned 📋

**Target**: Complete `math_f64.wgsl` — every standard math function, full IEEE 754 range

| Function | Current State | Target |
|----------|--------------|--------|
| `exp_f64`, `log_f64` | Software (hardware crashes on NVK/RADV) | Conditionally native via probe |
| `sin_f64`, `cos_f64` | Software | Cody-Waite range reduction + minimax |
| `atan2_f64` | Missing | Implement + fuzz-test vs libm |
| `asin_f64`, `acos_f64` | Missing | Implement + fuzz-test vs libm |
| `lgamma_f64` | Asymptotic only | Full Lanczos approximation |
| All | Not fuzz-tested | 10M random inputs, ULP ≤ 1 vs libm |

---

## Validation Hardware

| GPU | Vendor | Machine | Role | Phase Priority |
|-----|--------|---------|------|----------------|
| Titan V (SM70) | NVIDIA | hotSpring | Primary NVK test | Phase 1–3 validation |
| RTX 4070 (SM89) | NVIDIA | Tower | Proprietary baseline | All phases |
| RTX 3090 (SM86) | NVIDIA | gate2 | Proprietary validation | All phases |
| RX 6950 XT (RDNA2) | AMD | gate2 | ACO/RADV test | All phases |

`bench_wgsize_nvk` + `bench_f64_builtins` are the measurement tools.
Results feed `MeasuredLatencyModel` (Phase 2, ready to use).

---

## NAK Contribution Timeline

We contribute upstream as we validate, but we never *depend* on NAK merging.

| Phase | Our Work | NAK Contribution | Status |
|-------|----------|-----------------|--------|
| 0 | SM70 latency tables | MR patch in `contrib/mesa-nak/sm70_instr_latencies.rs` | Ready to submit — awaiting Titan V hw validation |
| 1 | Manual ILP, `@ilp_region` annotations | Before/after benchmark evidence for MR description | Pending bench run |
| 2 | `LatencyModel` abstraction | Propose `LatencyModel` interface for NAK | Ready to share |
| 3 | `WgslOptimizer` experience | Inform NAK Phase 2–4 (FMA selection, unrolling, dual-issue) | Ongoing |

---

## Mycelial Deployment Target

When all phases complete, a ToadStool node:

1. **Spawns** — single `toadstool` binary, Rust, no runtime dependencies
2. **Probes** — `bench_f64_builtins` runs once, builds `MeasuredLatencyModel`
3. **Optimises** — `WgslOptimizer` pre-schedules all shaders for this specific GPU
4. **Announces** — mDNS-SD, joins the ecosystem via Songbird
5. **Computes** — receives jobs via JSON-RPC, runs BarraCUDA shaders
6. **Reports** — performance metrics back via Songbird feedback channel

Zero config files required for the math to be optimal.
Zero vendor SDK required for correctness.
Zero central coordinator required for network formation.

**Substrate independence**: The same binary runs on:
- A Titan V in a data centre (SM70, full FP64)
- A Raspberry Pi 5 with VideoCore VII GPU (WGSL via Vulkan/DX12)
- A browser tab via WebGPU (WASM + WebGPU backend)
- An AMD workstation (RDNA2, ACO/RADV)
- A future GPU family we've never seen (probe → adapt → compute)

---

*"The mycelium is the internet of the forest. ToadStool is the mycelium of compute."*

*Last updated: February 19, 2026 — Phases 0–3 complete.*
