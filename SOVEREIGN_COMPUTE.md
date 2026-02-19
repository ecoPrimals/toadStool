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

| Phase | Description | Status | Target |
|-------|-------------|--------|--------|
| **0** | Fossil functions removed, NAK SM70 latency tables, capability probe | ✅ Done | Feb 2026 |
| **1** | Manual ILP in Jacobi kernel — 8-cycle DFMA gap filled at source | 🔄 Active | Feb 2026 |
| **2** | `LatencyModel` trait, `Sm70Model`, `Rdna2Model`, `MeasuredModel` | 📋 Planned | Q1 2026 |
| **3** | `WgslDependencyGraph` + `IlpReorderer` + `WgslLoopUnroller` in `ShaderTemplate` | 📋 Planned | Q2 2026 |
| **4** | Full naga-IR optimizer — SSA form, register pressure, loop pipelining | 📋 Planned | Q3 2026 |
| **5** | `math_f64.wgsl` completeness — sin/cos/atan2/asin/acos full range | 📋 Planned | Q3-Q4 2026 |

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

## Phase 1 — Active 🔄

**Target**: Jacobi eigensolve (`batched_eigh_single_dispatch_f64.wgsl`)

**What to do**:
1. Restructure the rotation kernel to expose ILP in the 8-cycle DFMA window
2. Interleave independent `cc = c*c`, `ss = s*s`, `cs = c*s` computations
   between the trig calls and the rotation updates
3. Add `// @unroll_hint 32` to the inner sweep loop
4. Validate on Titan V (SM70) with `bench_wgsize_nvk` — target ≥ 3× speedup
5. Validate neutral/positive on RTX 3090 (PTXAS already does this)

**Files**: `crates/barracuda/src/shaders/linalg/batched_eigh_single_dispatch_f64.wgsl`

---

## Phase 2 — Planned 📋

**Target**: `LatencyModel` trait in BarraCUDA

**What to build**:
```
crates/barracuda/src/device/latency.rs       ← new
  pub trait LatencyModel
  pub struct Sm70LatencyModel                ← DFMA=8cy, FFMA=4cy (arXiv:1804.06826)
  pub struct Rdna2LatencyModel               ← VFMA64=~4cy (AMD ISA docs)
  pub struct ConservativeModel               ← safe fallback
  pub struct MeasuredModel { dfma_cycles }   ← from bench_f64_builtins probe

crates/barracuda/src/device/capabilities.rs ← extend
  impl GpuDriverProfile {
    pub fn latency_model(&self) -> Box<dyn LatencyModel>
  }
```

**Feeds into**: Phase 3 reorderer needs a `LatencyModel` to know how many
independent ops to place between def and use.

---

## Phase 3 — Planned 📋

**Target**: `WgslOptimizer` module in `ShaderTemplate`

**What to build**:
```
crates/barracuda/src/shaders/optimizer/
  mod.rs
  dependency_graph.rs   ← let-binding DAG analysis
  ilp_reorderer.rs      ← topological sort guided by LatencyModel
  loop_unroller.rs      ← bounded loops (≤ 32 iterations)

Annotation format in WGSL:
  // @ilp_region begin
  let a = ...;
  let b = ...;   ← optimizer reorders this block
  // @ilp_region end
```

**Scope**: Opt-in per shader via `// @ilp_region` annotations.
Straight-line `let` sequences only. No branches. Jacobi inner kernel first.

**Integration**:
```rust
impl ShaderTemplate {
    pub fn for_driver_auto(shader: &str, needs_workaround: bool) -> String {
        let shader = Self::substitute_fossil_f64(shader);
        let shader = if let Some(profile) = current_profile() {
            WgslOptimizer::reorder(&shader, &profile.latency_model())  // NEW
        } else { shader };
        if needs_workaround { Self::apply_transcendental_workaround(&shader) }
        else { shader }
    }
}
```

---

## Phase 4 — Planned 📋

**Target**: Full naga-IR optimizer

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
Inter-iteration loop pipelining (iteration i+1 loads during iteration i's ops).

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
| Titan V (SM70) | NVIDIA | hotSpring | Primary NVK test | Phase 1 validation |
| RTX 4070 (SM89) | NVIDIA | Tower | Proprietary baseline | Phase 1 baseline |
| RTX 3090 (SM86) | NVIDIA | gate2 | Proprietary validation | All phases |
| RX 6950 XT (RDNA2) | AMD | gate2 | ACO/RADV test | All phases |

`bench_wgsize_nvk` + `bench_f64_builtins` are the measurement tools.
Results feed `MeasuredLatencyModel` (Phase 2).

---

## NAK Contribution Timeline

We contribute upstream as we validate, but we never *depend* on NAK merging.

| Phase | Our Work | NAK Contribution | Timing |
|-------|----------|-----------------|--------|
| 0 (done) | SM70 latency tables | Submit MR with `sm70_instr_latencies.rs` | Post Titan V validation |
| 1 | Manual ILP, benchmark results | Share before/after numbers as evidence | After Phase 1 bench |
| 2 | Latency model interface | Propose `LatencyModel` abstraction for NAK | When Phase 2 is stable |
| 3-4 | WGSL optimizer experience | Inform NAK Phase 2-4 (FMA selection, unrolling, dual-issue) | Ongoing |

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
