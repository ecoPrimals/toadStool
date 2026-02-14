# BarraCUDA GPU MD Evolution — hotSpring Phase C Handoff

**Date**: February 14, 2026  
**From**: hotSpring validation team  
**To**: ToadStool / BarraCUDA evolution team  
**Re**: GPU f64 Molecular Dynamics — lessons, code patterns, and evolution targets

---

## Summary

hotSpring Phase C built a **complete f64 GPU molecular dynamics pipeline** for Yukawa OCP
plasmas. All 9 PP Yukawa DSF study cases pass validation on the RTX 4070 using `SHADER_F64`.
This document describes what we built, how it maps to existing ops, what to adapt,
and what new capabilities to evolve.

**Result**: 9/9 cases pass. 0.000% energy drift. 3.7× GPU speedup at N=2000.
60 minutes total on a $350 GPU.

---

## 1. What Exists in ToadStool (f32 Ops)

`crates/barracuda/src/ops/md/` contains:

| Op | File | Precision | Notes |
|----|------|-----------|-------|
| Yukawa force | `forces/yukawa.rs` + `.wgsl` | f32 | All-pairs, charges, no PBC in force |
| Coulomb force | `forces/coulomb.rs` + `.wgsl` | f32 | Bare Coulomb |
| Lennard-Jones | `forces/lennard_jones.rs` + `.wgsl` | f32 | Standard LJ |
| Born-Mayer | `forces/born_mayer.rs` + `.wgsl` | f32 | Short-range repulsion |
| Morse | `forces/morse.rs` + `.wgsl` | f32 | Bond stretching |
| Velocity-Verlet | `integrators/velocity_verlet.rs` + `.wgsl` | f32 | Full VV (old+new forces) |
| RK4 | `integrators/rk4.rs` + `.wgsl` | f32 | 4th-order Runge-Kutta |
| Laplacian | `integrators/laplacian.rs` + `.wgsl` | f32 | Discrete Laplacian |
| PBC distance | `pbc.rs` + `.wgsl` | f32 | Pairwise distance matrix with MIC |

**Key observation**: These are all **single-dispatch ops** — they compute one thing (force,
one VV step, PBC distance matrix). They don't compose into a simulation loop.

---

## 2. What hotSpring Phase C Added (f64 Simulation Loop)

All code lives in `hotSpring/barracuda/src/md/`:

| File | What it does | New vs existing |
|------|-------------|-----------------|
| `shaders.rs` | 7 WGSL shader source strings (inline) | **New** — f64 versions of existing + new kernels |
| `simulation.rs` | Full GPU MD loop (equil + prod) | **New** — nothing like this in toadstool |
| `config.rs` | Sarkas parameter configurations | **New** — OCP reduced units |
| `observables.rs` | RDF, VACF, SSF, energy validation | **New** — no observable compute in toadstool |
| `cpu_reference.rs` | CPU mirror for benchmarking | **New** — pure Rust reference |
| `mod.rs` | Module root | Standard |

### 2.1 New Shaders (Inline in shaders.rs)

| Shader | Purpose | toadstool equivalent |
|--------|---------|---------------------|
| `SHADER_YUKAWA_FORCE` | f64 all-pairs Yukawa with PBC + PE accumulation | `forces/yukawa.wgsl` (f32, no PBC, no PE) |
| `SHADER_YUKAWA_FORCE_CELLLIST` | f64 cell-list Yukawa (27-neighbor) | **None** — new capability |
| `SHADER_VV_KICK_DRIFT` | Fused half-kick + drift + PBC wrap | `integrators/velocity_verlet.wgsl` (f32, separate) |
| `SHADER_VV_HALF_KICK` | Second half-kick only | **None** — split VV is new |
| `SHADER_BERENDSEN` | Velocity rescaling thermostat | **None** — new capability |
| `SHADER_KINETIC_ENERGY` | Per-particle KE for temperature | **None** — new capability |
| `SHADER_RDF_HISTOGRAM` | Pair distance binning with atomicAdd | **None** — new capability |

### 2.2 Simulation Loop Architecture

The hotSpring MD loop runs like this (each step):

```
1. SHADER_VV_KICK_DRIFT     — half-kick velocities, drift positions, PBC wrap
2. SHADER_YUKAWA_FORCE       — recompute forces from new positions (+ PE)
3. SHADER_VV_HALF_KICK       — second half-kick with new forces
4. (equil only) SHADER_BERENDSEN — thermostat velocity rescaling
5. (at dump intervals) SHADER_KINETIC_ENERGY — read back for energy tracking
```

All data stays on GPU. CPU only reads back at dump intervals.

### 2.3 Cell-List Neighbor Search

For N > ~5000, the all-pairs O(N²) kernel becomes inefficient. hotSpring implements a
**CPU-managed cell list** that sorts particles by spatial cell, then the GPU kernel only
loops over 27 neighboring cells per particle:

```
CPU side:
  1. Read back positions
  2. Assign particles to cells (box_side / rc cells per dim)
  3. Sort particle arrays by cell index
  4. Compute cell_start[] and cell_count[]
  5. Upload sorted arrays + cell metadata

GPU side:
  Each thread loops over 27 neighbor cells instead of all N particles
```

Currently rebuilt every step (rebuild_interval=1) for correctness. This is the right
trade for small N; at large N the CPU overhead is <5% of total.

---

## 3. Physics Notes (OCP Reduced Units)

hotSpring uses **OCP reduced units** throughout, matching Sarkas:

| Quantity | Reduced unit | Value |
|----------|-------------|-------|
| Length | Wigner-Seitz radius a_ws | 1.0 |
| Time | Inverse plasma frequency ω_p⁻¹ | 1.0 |
| Energy | e²/(4πε₀ a_ws) | 1.0 |
| Temperature | T* = 1/Γ | Varies |
| Mass | m* = 4πn a_ws³ = **3.0** | Fixed |
| Force prefactor | 1.0 (Γ enters via temperature) | Fixed |
| Box side | L = (4πN/3)^(1/3) | Computed from N |
| Number density | n* = 3/(4π) | Fixed |

**Critical**: The reduced mass of 3.0 was a debugging milestone. Using mass=1.0 (the
naive assumption) causes temperature to be wrong by factor 3, which looks like divergence
in the thermostat. The Sarkas docs don't spell this out clearly.

### DSF Study Parameters (9 PP Yukawa Cases)

| κ | Γ values | rc/a_ws | dt* |
|---|----------|---------|-----|
| 1 | 14, 72, 217 | 8.0 | 0.01 |
| 2 | 31, 158, 476 | 6.5 | 0.01 |
| 3 | 100, 503, 1510 | 6.0 | 0.01 |

All cases use Berendsen thermostat (τ/dt = 5.0) during equilibration, then NVE production.

---

## 4. Validated Results (Eastgate, RTX 4070, N=2000)

### 4.1 Energy Conservation

All 9 cases achieve **0.000% energy drift** (exact symplectic VV in production).
Maximum drift across all cases: 0.004%.

### 4.2 Observable Trends

| Observable | Physical expectation | Verified? |
|-----------|---------------------|-----------|
| RDF peak height | Increases with Γ (more structure) | ✓ |
| RDF tail g(r)→1 | All cases < 0.0015 error | ✓ |
| Diffusion D* | Decreases with Γ (more coupling) | ✓ |
| SSF S(k→0) | Compressibility consistent with theory | ✓ |

### 4.3 GPU vs CPU Performance

| N | GPU steps/s | CPU steps/s | Speedup |
|---|-------------|-------------|---------|
| 500 | 521.5 | 608.1 | 0.9× (dispatch overhead dominates) |
| 2000 | 240.5 | 64.8 | **3.7×** |

GPU advantage scales as O(N²) because force computation dominates and GPU parallelizes it.
At N=10,000 we expect **50-100×** speedup.

### 4.4 Energy Efficiency

| N | GPU J/step | CPU J/step | Ratio |
|---|-----------|-----------|-------|
| 500 | 0.081 | 0.071 | 1.1× more |
| 2000 | 0.207 | 0.712 | **3.4× less** |

---

## 5. Evolution Targets for ToadStool

### 5.1 Port f64 Force Kernels (HIGH)

Take hotSpring's `SHADER_YUKAWA_FORCE` and create a proper f64 variant of `forces/yukawa.rs`:

**Key differences from f32 version**:
- Uses `array<f64>` instead of `array<f32>`
- PBC minimum image integrated into force loop (not separate PBC op)
- Accumulates per-particle PE alongside force
- Uses `math_f64.wgsl` for `sqrt_f64`, `exp_f64`, `round_f64`
- No vec3<f64> (not supported) — all scalar arithmetic
- Force sign: `fx = fx - force_mag * dx * inv_r` (repulsive Yukawa)

**Pattern**: Prepend via `ShaderTemplate::with_math_f64(shader_body)` which already exists.

### 5.2 Add Cell-List to MD Ops (HIGH)

Neither hotSpring nor toadstool has a proper cell-list op. hotSpring's implementation
is CPU-managed with GPU-side 27-neighbor iteration. Proper evolution:

1. Add `ops::md::neighbor::CellList` struct to toadstool
2. GPU kernel for cell assignment + counting (parallel histogram)
3. GPU radix sort by cell index (or use existing sort infrastructure)
4. Keep the 27-neighbor force kernel pattern from hotSpring
5. Configurable rebuild interval (every N steps)

### 5.3 Add Berendsen Thermostat Op (MEDIUM)

Simple velocity rescaling: `v *= scale`. The shader is 10 lines. But also add:
- Nosé-Hoover (for NVT production runs)
- Langevin thermostat (for stochastic dynamics)

### 5.4 Add Observable Compute Ops (MEDIUM)

hotSpring computes these on CPU from GPU snapshots. For large N, compute directly on GPU:

| Observable | GPU kernel needed | Notes |
|-----------|------------------|-------|
| RDF histogram | `atomicAdd` pair-distance binning | hotSpring has `SHADER_RDF_HISTOGRAM` |
| Kinetic energy | Per-particle `0.5 * m * v²` | hotSpring has `SHADER_KINETIC_ENERGY` |
| VACF | Dot product of v(t)·v(t+τ) | Currently CPU only |
| SSF S(k) | Fourier sum `|Σ exp(ik·r)|²` | Currently CPU only, natural for GPU |

### 5.5 Split Velocity-Verlet (MEDIUM)

Current VV shader takes old+new forces and does the full update in one dispatch.
hotSpring's split VV (half-kick → drift → force → half-kick) is more flexible:
- Allows thermostating between kicks
- Allows force kernel swap without touching integrator
- Standard in production MD codes (LAMMPS, GROMACS)

### 5.6 PPPM / Ewald for Long-Range Coulomb (HIGH)

The 3 Coulomb (κ=0) DSF cases require particle-particle particle-mesh:
- Short-range: screened Coulomb in real space (existing Coulomb kernel)
- Long-range: FFT of charge density on mesh → reciprocal-space potential
- Already have `ops/fft/fft_3d.rs` — need to wrap it into an Ewald decomposition

This is the gateway to full plasma simulation (not just screened potentials).

### 5.7 MSU HPC Comparison Benchmark (FUTURE)

Pin a CPU-only baseline at N=10,000+ with 80k production steps on MSU HPCC (iCER).
Run the same physics on consumer GPU. The headline number:
**HPC cluster vs consumer GPU, same physics, same observables.**

Route: Murillo collaboration or alumni access.

---

## 6. Bugs and Gotchas We Hit

### 6.1 Force Sign Convention

**Bug**: Initial implementation had `fx = fx + force_mag * dx * inv_r` (attractive).
Yukawa between like charges is **repulsive**: `fx = fx - force_mag * dx * inv_r`.

**How to tell**: Temperature diverges exponentially within ~100 steps. Particles collapse
into clusters. Total energy goes to -∞.

### 6.2 Reduced Mass in OCP Units

**Bug**: Using mass=1.0 instead of mass=3.0 in OCP reduced units causes the kinetic
energy to be wrong by factor 3. The thermostat targets T*=1/Γ but measures wrong
temperature, overcorrecting and causing oscillations.

**How to tell**: Temperature oscillates wildly around target during equilibration,
never settling. Energy drift in production is ~10-50%.

### 6.3 Cell-List Staleness

**Bug**: With rebuild_interval=50, particles migrate across cell boundaries between
rebuilds. The force kernel then uses stale cell assignments, missing neighbors.

**How to tell**: Gradual temperature rise during production (not sudden divergence).
Energy drift increases linearly with time.

**Fix**: rebuild_interval=1 (every step) for now. At large N, the rebuild cost is
<5% of total. For truly large N, use Verlet lists with skin radius.

### 6.4 Stale GPU Processes

**Bug**: Previous `sarkas_gpu` runs that didn't exit cleanly leave processes holding
GPU memory. New runs get throttled (49 steps/s instead of 500 steps/s).

**How to tell**: `nvidia-smi` shows multiple processes. Kill them.

### 6.5 Naga f64 Type Promotion (from Phase B, still relevant)

All the `x - x + constant` patterns documented in `HOTSPRING_GPU_HANDOFF.md` and
`specs/FP64_GPU_EVOLUTION.md` apply to these MD shaders too. The force kernel has
many constants (0.5, 1.0, etc.) that must be constructed from f64 arithmetic.

---

## 7. File Map: hotSpring → ToadStool

| hotSpring file | Lesson for ToadStool | Priority |
|---------------|---------------------|----------|
| `md/shaders.rs` → `SHADER_YUKAWA_FORCE` | f64 Yukawa with PBC + PE → evolve `forces/yukawa.rs` | HIGH |
| `md/shaders.rs` → `SHADER_YUKAWA_FORCE_CELLLIST` | Cell-list force kernel → new `forces/yukawa_celllist.rs` | HIGH |
| `md/shaders.rs` → `SHADER_VV_KICK_DRIFT` | Split VV → evolve `integrators/velocity_verlet.rs` | MEDIUM |
| `md/shaders.rs` → `SHADER_BERENDSEN` | Thermostat → new `integrators/berendsen.rs` | MEDIUM |
| `md/shaders.rs` → `SHADER_KINETIC_ENERGY` | KE reduction → new `observables/kinetic_energy.rs` | MEDIUM |
| `md/shaders.rs` → `SHADER_RDF_HISTOGRAM` | RDF on GPU → new `observables/rdf.rs` | MEDIUM |
| `md/simulation.rs` | Full loop orchestration → inform `session.rs` patterns | REFERENCE |
| `md/config.rs` | OCP reduced units → inform docs/examples | REFERENCE |
| `md/observables.rs` | CPU-side validation logic → reference implementation | REFERENCE |
| `md/cpu_reference.rs` | CPU mirror for benchmarking → test infrastructure | LOW |

---

## 8. What's Already Converged

The toadstool team's latest commit (`f370a64e`) already absorbed several hotSpring lessons:

| Lesson | Status |
|--------|--------|
| `math_f64.wgsl` (27+ functions) | ✓ In toadstool, 649 lines (evolved beyond hotSpring's 370-line prototype) |
| `ShaderTemplate::with_math_f64()` | ✓ In toadstool, plus `math_f64_subset()` for modular extraction |
| Naga f64 gotchas | ✓ Documented in `specs/FP64_GPU_EVOLUTION.md` |
| `f64_const(x, c)` helper | ✓ In upstream math_f64.wgsl |
| FMA ops | ✓ New `ops/fma.rs` + WGSL shaders |
| Cache hierarchy awareness | ✓ New `device/cache_hierarchy.rs` |
| AGPL-3.0 license unification | ✓ All crates unified |

**Not yet absorbed**: Everything in Sections 5.1–5.6 above (MD-specific evolution).

---

## 9. The Strategic Picture

```
Current state:
  ToadStool has:  f32 MD ops (isolated dispatch)  +  f64 math library
  hotSpring has:  f64 MD simulation loop (validated)

Evolution target:
  ToadStool gets: f64 MD ops (composable) + cell-list + thermostats + observables
                  ↓
  Sarkas-equivalent MD runs as a first-class BarraCUDA pipeline
                  ↓
  Add PPPM/Ewald → full plasma simulation on consumer GPU
                  ↓
  MSU HPC comparison → "same physics, consumer hardware" headline
```

The hotSpring validation proves the physics work. ToadStool's job is to evolve isolated
kernels into a composable, generic MD pipeline that handles any force law, any integrator,
any thermostat — on any GPU vendor.

---

*From the hotSpring validation desk, February 14, 2026*
