# Sovereign Compute — Evolution Tracker

**Vision**: ToadStool as ubiquitous as fungus. BarraCuda as universal math.
Pure Rust. Any hardware. No vendor lock.

**Full spec**: [`specs/SOVEREIGN_COMPUTE_EVOLUTION.md`](specs/SOVEREIGN_COMPUTE_EVOLUTION.md)
**Remaining work**: [`SOVEREIGN_COMPUTE_GAPS.md`](SOVEREIGN_COMPUTE_GAPS.md) — checklist of gaps to close

> **Note (S153):** Sovereign compiler Phases 0–4 and all shader/math infrastructure transferred to **barraCuda** (`ecoPrimals/barraCuda/`). **IPC-first architecture** (barraCuda v0.35): all inter-primal communication via JSON-RPC at runtime. coralReef Iteration 43 owns VFIO transport (PFIFO channel init, QMD, pushbuf, DMA). toadStool owns VFIO interface (device management, permissions, pooling, dispatch routing). See gaps doc for remaining work.

---

## The North Star

```
Today:    WGSL → naga → SPIR-V → vendor compiler → GPU
                                   ^^^^^^^^^^^^^^^^^^^
                                   We depend on this being good

Sovereign: WGSL → BarraCuda WgslOptimizer → pre-scheduled SPIR-V → vendor → GPU
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
| **2** | `LatencyModel` trait, Sm70/Rdna2/Conservative/Measured models + `AppleMLatencyModel` | ✅ Done | Feb 19–20, 2026 |
| **3** | `WgslDependencyGraph` + `IlpReorderer` + `WgslLoopUnroller` built + **wired into `compile_shader_f64()`** | ✅ Done | Feb 20, 2026 |
| **4** | naga-IR optimizer — FMA fusion, dead expr elimination, SPIR-V passthrough | ✅ Done | Feb 25, 2026 |
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
  pub struct Sm70LatencyModel         ← DFMA=8cy, FFMA=4cy (arXiv:1804.06826) — SM70–SM89
  pub struct Rdna2LatencyModel        ← VFMA64=~4cy (AMD ISA docs + empirical) — RDNA2/3/CDNA2
  pub struct AppleMLatencyModel       ← software-emulated f64 ~16cy, f32 ~4cy — M1/M2/M3/M4
  pub struct ConservativeModel        ← safe maximum fallback (unknown/Intel GPUs)
  pub struct MeasuredModel            ← populated from bench_f64_builtins probe
  pub fn model_for_arch(GpuArch)      ← dispatch helper (all known GPU families covered)

crates/barracuda/src/device/capabilities.rs
  pub enum GpuArch { Volta, Turing, Ampere, Ada, Rdna2, Rdna3, Cdna2, IntelArc, AppleM, Software, Unknown }
  impl GpuDriverProfile {
    pub fn latency_model(&self) -> Box<dyn LatencyModel>
  }
```

Cross-vendor latency table (complete as of Feb 20, 2026):

| GPU Family | Arch | DFMA/FMA64 | FFMA | Model |
|---|---|---|---|---|
| SM70–SM89 (Volta → Ada) | NVIDIA | 8 cy | 4 cy | `Sm70LatencyModel` |
| RDNA2/3, CDNA2 | AMD | ~4 cy | ~4 cy | `Rdna2LatencyModel` |
| M1/M2/M3/M4 (software f64) | Apple | ~16 cy (SW) | ~4 cy | `AppleMLatencyModel` |
| Intel Xe / Unknown | — | — | — | `ConservativeModel` |
| Measured at runtime | Any | empirical | empirical | `MeasuredModel` |

9 unit tests covering all five models and the arch dispatch function.

---

## Phase 3 — Done ✅ (built Feb 19, wired Feb 20)

**Target**: `WgslOptimizer` in `crates/barracuda/src/shaders/optimizer/` — **now live in the compilation hot path**

**What was built (Feb 19)**:
```
crates/barracuda/src/shaders/optimizer/
  mod.rs                ← WgslOptimizer::optimize(), for_arch(), Default (Conservative)
  dependency_graph.rs   ← WgslDependencyGraph: parse() let-binding DAG, classify_op()
  ilp_reorderer.rs      ← IlpReorderer: ASAP list scheduling (BinaryHeap, release_cycle)
  loop_unroller.rs      ← WgslLoopUnroller: @unroll_hint N, word-boundary substitution
```

**Wired into compilation (Feb 20)**:
`WgpuDevice::compile_shader_f64()` now runs the full two-stage pipeline:
1. `ShaderTemplate::for_driver_auto()` — exp/log patches for NVK/RADV drivers
2. `WgslOptimizer::optimize()` — `@ilp_region` reorder + `@unroll_hint` loop unroll

Fast-path guard: optimizer is a no-op when neither annotation is present (single
`contains()` call — zero overhead on the 480+ shaders without ILP regions). For the
Jacobi eigensolve (`batched_eigh_single_dispatch_f64.wgsl`), the reorderer fires
automatically, pre-scheduling DFMA pairs for the detected GPU's actual cycle count.

The latency model is taken from `GpuDriverProfile::latency_model()` — hardware-accurate
for SM70/RDNA2/AppleM; Conservative fallback for Intel/Unknown.

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

## Phase 4 — Done ✅ (Feb 25, 2026)

**Target**: naga-IR optimizer — `SovereignCompiler` in `crates/barracuda/src/shaders/sovereign/`

**Pipeline** (now live in `compile_shader_f64()`):
```
WGSL text (after ShaderTemplate + WgslOptimizer)
    → naga::front::wgsl::parse_str()   → naga::Module
    → fma_fusion::fuse_multiply_add()  → Mul+Add → Fma (1.3× NAK deficiency fixed)
    → dead_expr::eliminate()           → unused expressions removed
    → naga::valid::Validator           → validates optimized module
    → spv_emit::emit_spirv()           → Vec<u32> SPIR-V words
    → wgpu (SPIRV_SHADER_PASSTHROUGH)  → driver → GPU
```

**What was built**:
```
crates/barracuda/src/shaders/sovereign/
  mod.rs            ← SovereignCompiler: compile(), SovereignOutput::Spirv, CompileStats
  fma_fusion.rs     ← fuse_multiply_add(): Mul+Add → Fma (single-consumer only, safe)
  dead_expr.rs      ← eliminate(): mark-sweep DCE on naga expression arena
  spv_emit.rs       ← emit_spirv(): naga::back::spv::Writer, Vulkan 1.1, SPIR-V 1.3
```

**Integration**:
- `naga = "22.1"` added as direct dependency (same version as wgpu 22, zero type conflicts)
- `wgpu::Features::SPIRV_SHADER_PASSTHROUGH` requested in all device creation paths
- `WgpuDevice::compile_shader_spirv()` wraps `device.create_shader_module_spirv()`
- `compile_shader_f64()` attempts sovereign path first, falls back to WGSL text on error

**Fallback guarantee**: If `SPIRV_SHADER_PASSTHROUGH` is unavailable (WebGPU, some mobile
drivers), the existing WGSL text path continues working. Phase 4 is additive.

**FMA fusion safety**: Only fuses `Mul(a,b) + c` when the multiply has exactly one consumer.
No silent precision changes — `fma(a,b,c)` is strictly more precise than `a*b + c`.

10 unit tests covering FMA fusion, dead expression elimination, SPIR-V round-trip,
and full sovereign compilation pipeline.

**Remaining Phase 4 work** (future iterations):
- Register pressure estimation (live-range counting)
- Loop software pipelining at naga IR level
- Architecture-specific polynomial selection for transcendentals

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
5. **Computes** — receives jobs via JSON-RPC, runs BarraCuda shaders
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

*Last updated: March 13, 2026 — S153. Phases 0–4 complete and live in barraCuda (transferred S93–S94). ToadStool sovereign infrastructure complete (S151–S153): all toadStool-owned gaps 1–12 resolved. IPC-first architecture: barraCuda v0.35 has zero compile-time primal deps; toadStool is sole VFIO detection source via `compute.hardware.vfio_devices`. coralReef Iteration 43: PFIFO hardware channel creation via BAR0 MMIO closes the critical VFIO dispatch blocker. Pipeline: barraCuda (WGSL) →[JSON-RPC] coralReef (compile) →[JSON-RPC] toadStool (dispatch) → GPU. ecoprimals-mode CLI for science/gaming switching. 96+ JSON-RPC methods. 21,156+ tests.*
