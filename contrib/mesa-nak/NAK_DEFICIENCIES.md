# NAK Compiler Deficiencies for f64 Loop-Heavy Kernels

**Date:** 2026-02-19  
**Source:** hotSpring / ToadStool joint analysis  
**Context:** `NVK_EIGENSOLVE_PERF_ANALYSIS_FEB18_2026.md` + Jacobi eigensolve benchmarks  
**Applies to:** Mesa NAK (NVIDIA Turing/Volta backend, SM70/SM75)  
**License:** AGPL-3.0-only

---

## Background

On NVK (Mesa nouveau + NAK compiler), a batched f64 Jacobi eigensolve measured
**149× slower** than the proprietary NVIDIA driver on the same hardware (Titan V,
SM70).  Decomposition of the gap:

| Factor | Slowdown | Recoverable? |
|--------|----------|--------------|
| Loop unrolling absent | ~4× | Yes — NAK patch |
| Register spills to local memory | ~2× | Yes — NAK patch |
| Source-order scheduling | ~1.5× | Yes — NAK patch |
| Missing FMA fusion | ~1.3× | Yes — NAK patch |
| Shared memory bank conflicts | ~1.1× | Yes — NAK patch |
| **Compound** | **~9×** | **Yes** |

The remaining ~16× is attributed to other overhead (driver setup, SPIR-V
translation layers) not specific to NAK.  The 9× NAK-recoverable gap is the
target of this document.

**Universal benefit**: every Spring (MD, HFB, PPPM, eigensolve) running on any
consumer GPU with NVK benefits from each fix.  No proprietary driver needed.

---

## Deficiency 1 — Loop Unrolling

### Symptom
NAK emits one SPIR-V iteration per loop body.  The Jacobi k-loop (iterating
over n ≤ 32 matrix rows) runs 32 DFMA sequences serially, each stalling 8 cycles
waiting for the previous DFMA result on SM70.

**Measured impact: ~4× of the 9× gap.**

### Proprietary Behaviour
NVIDIA proprietary driver aggressively unrolls the k-loop, making all 32 row
iterations visible to the hardware scheduler simultaneously.  Inter-iteration ILP
fills the 8-cycle DFMA latency window with independent loads from iterations i+1
through i+3.

### Mesa NAK Location
- `src/compiler/nak/opt_instr.rs` — instruction-level optimisations  
- `src/compiler/nak/lower_vec.rs` — loop lowering path  
- LLVM reference: `lib/Transforms/Scalar/LoopUnrollPass.cpp`

### Proposed Patch
Add a loop unroll pass that:
1. Detects loops with statically known trip count (from uniform `params.n`, max 32).
2. For trip count ≤ UNROLL_THRESHOLD (suggest 32), fully unroll and insert
   range guards `if (k < n)` for each copy.
3. Prioritise loops containing DFMA chains (detect via register dependency graph).

```rust
// Pseudocode for NAK loop unroller heuristic
fn should_unroll(loop_info: &LoopInfo) -> bool {
    loop_info.trip_count_known()
        && loop_info.trip_count() <= UNROLL_THRESHOLD
        && loop_info.body_contains_dfma()
}
```

### WGSL Workaround (interim — already applied in barracuda)
`batched_eigh_nak_optimized_f64.wgsl` manually unrolls 4 iterations per step,
grouping all loads before all FMAs.  This gives the scheduler 4 independent DFMA
chains and yields a **2–4× speedup on NVK** without any compiler changes.

---

## Deficiency 2 — Register Allocation / Local Memory Spills

### Symptom
NAK spills f64 values to local (per-thread) memory when a value is used more
than once across instruction boundaries.  On SM70, each f64 occupies 2 registers;
a 6-value set (c, s, cc, ss, two_cs, neg_s) for the Jacobi rotation uses 12
registers, which NAK may not keep live simultaneously.

**Measured impact: ~2× of the 9× gap.**

### Proprietary Behaviour
Proprietary backend keeps all rotation scalars in registers for the full inner
loop.  No local memory traffic during the hot path.

### Mesa NAK Location
- `src/compiler/nak/ra.rs` — register allocator  
- Priority: increase the live-range window for values used in DFMA chains.

### Proposed Patch
Extend the RA heuristic to recognise values computed outside a loop that are
used inside it (loop-invariant scalars).  Pin these to registers for the loop
duration rather than spilling between iterations.

### WGSL Workaround
Explicit `let` bindings for every reused scalar (`c`, `s`, `neg_s`, `cc`, `ss`,
`two_cs`, `app_new`, `aqq_new`) prevent NAK from seeing the computation as an
inline expression that it might rematerialise or spill.

---

## Deficiency 3 — Source-Order Instruction Scheduling

### Symptom
NAK emits SPIR-V OpLoad / OpFMul / OpFAdd instructions in the order they appear
in the source.  A load followed immediately by a dependent FMA stalls 8 cycles
on SM70 (DFMA latency).

**Measured impact: ~1.5× of the 9× gap.**

### Proprietary Behaviour
Proprietary scheduler reorders instructions for latency hiding: memory loads
for iteration i+1 are interleaved with the FMA for iteration i, keeping the
execution units busy.

### Mesa NAK Location
- `src/compiler/nak/sched.rs` — instruction scheduler  
- Reference: NVIDIA's SASS scheduler (observed via `nvdisasm` output).

### Proposed Patch
Implement a list scheduler (already partially present in `sched.rs`) that:
1. Builds a data-dependency DAG.
2. Uses ASAP scheduling: issue loads as early as possible relative to their
   consumer FMAs.
3. Use the latency table from `sm70_instr_latencies.rs` (DFMA = 8 cy,
   global load = 200–800 cy, shared = 23 cy).

### WGSL Workaround
Load groups (akp0..akp3, akq0..akq3, vkp0..vkp3, vkq0..vkq3) are all issued
before the FMA group.  Even with source-order NAK, this separates loads from
their consuming FMAs by approximately the load latency.

---

## Deficiency 4 — Missing FMA Fusion

### Symptom
For the pattern `a * b + c`, NAK emits:
```
DMUL  r0, ra, rb    // a * b
DADD  r0, r0, rc   // + c
```
Proprietary emits a single `DFMA r0, ra, rb, rc`.  This halves instruction
count, avoids an intermediate rounding, and is ~1.3× faster on SM70.

**Measured impact: ~1.3× of the 9× gap.**

### Mesa NAK Location
- `src/compiler/nak/lower_fma.rs` (if it exists) or `opt_algebraic.rs`
- SPIR-V source: `OpFMulAdd` / GLSL `fma()` → must map to DFMA, not DMUL+DADD.

### Proposed Patch
Ensure the NAK f64 FMA lowering path emits SM70 `DFMA` for:
- SPIR-V `OpFMulAdd` (from WGSL `fma()`)
- Algebraically equivalent `OpFMul` + `OpFAdd` patterns (peephole)

### WGSL Workaround
Explicit `fma(a, b, c)` calls throughout `batched_eigh_nak_optimized_f64.wgsl`
guarantee `OpFMulAdd` in SPIR-V.  Correctly implemented NAK will lower this to
`DFMA`.

---

## Deficiency 5 — Shared Memory Bank Conflicts / Branch Penalties

### Symptom
For sign-selection patterns (`if (x >= 0) { t = 1.0 } else { t = -1.0 }`),
NAK emits conditional branches.  On SIMT hardware, divergent branches within a
warp are serialised.

Additionally, NAK does not insert shared-memory padding or access-pattern
swizzling for f64 (2-register) values.  On SM70, 32-bank shared memory with
2-register f64 values results in 2-way bank conflicts for naive linear access.

**Measured impact: ~1.1× of the 9× gap (branch penalty; bank conflicts are
  not triggered in the global-memory-only Jacobi path).**

### Mesa NAK Location
- Branch predicates: `src/compiler/nak/opt_pred.rs`  
- Shared memory: `src/compiler/nak/lower_smem.rs`

### Proposed Patch
**Branches:** add a peephole pass that converts single-assignment sign-select
branches to PRMT / SEL instructions (equivalent to WGSL `select()`).

**Shared memory:** for f64 allocations, insert 1-word padding per 16 values
to avoid 2-way bank conflicts (same technique as CUDA `__shared__ double x[N+1]`).

### WGSL Workaround
`select(false_val, true_val, cond)` throughout `batched_eigh_nak_optimized_f64.wgsl`
(identity init, convergence max, rotation-angle sign).  No shared memory is used
in the Jacobi path — the workaround fully eliminates this deficiency for the
eigensolve kernel.

---

## Contribution Priority

| Priority | Deficiency | Expected Gain | Estimated Effort |
|----------|-----------|---------------|-----------------|
| **1** | Loop unrolling | ~4× | Medium (2–4 weeks) |
| **2** | Register allocation | ~2× | High (4–8 weeks) |
| **3** | Instruction scheduling | ~1.5× | Medium (2–4 weeks) |
| **4** | FMA fusion | ~1.3× | Low (1 week) |
| **5** | Branch predicates | ~1.1× | Low (1 week) |

Fixing deficiency 1 alone brings NVK within **~2.25× of proprietary** on the
Jacobi eigensolve.  Full NAK parity makes every consumer GPU a sovereign compute
node — no proprietary driver required, ever.

---

## Validation Strategy

Each patch should be validated with the `validate_nak_eigensolve` binary
(barracuda / hotSpring):

1. **Correctness**: eigenvalues match CPU reference within 1e-3 relative.
2. **Parity**: NAK-optimized ≡ baseline shader to 1e-15 (bitwise math equivalence).
3. **Performance**: measure steps/s on NVK before and after each patch.
4. **No regression**: proprietary driver must be neutral (± 2%).

---

## Reference Files

| File | Description |
|------|-------------|
| `sm70_instr_latencies.rs` | SM70 instruction latency table for scheduler |
| `rdna2_instr_latencies.rs` | RDNA2 latency table (AMD RADV/ACO reference) |
| `ecoPrimals/barraCuda/crates/barracuda/src/shaders/linalg/batched_eigh_nak_optimized_f64.wgsl` | WGSL workarounds for all 5 deficiencies (lives in barraCuda repo) |
| `ecoPrimals/barraCuda/crates/barracuda/src/shaders/optimizer/loop_unroller.rs` | barraCuda WGSL loop unroller (reference implementation; lives in barraCuda repo) |

---

*License: AGPL-3.0-only. Sovereign community property.*
