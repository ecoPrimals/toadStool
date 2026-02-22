# hotSpring Math Primitives Absorption — February 15, 2026

**Date**: February 15, 2026  
**Status**: ✅ Complete  
**From**: hotSpring handoff document (Feb 12, 2026)  
**Absorbed by**: ToadStool/BarraCUDA team

---

## Summary

This document records the absorption of physics-agnostic GPU math primitives from
hotSpring's nuclear EOS validation study into ToadStool/BarraCUDA. These primitives
were validated by hotSpring's 169/169 acceptance checks on consumer GPU hardware
(RTX 4070, f64 via SHADER_F64).

---

## Absorbed Primitives

### 1. f64 Special Function Shaders

**Location**: `crates/barracuda/src/shaders/special/`

| New Shader | Source | Purpose |
|------------|--------|---------|
| `hermite_f64.wgsl` | hotSpring deformed HFB | Physicist's Hermite polynomials (f64) |
| `laguerre_f64.wgsl` | hotSpring deformed HFB | Generalized Laguerre polynomials (f64) |

**Features**:
- Three-term recurrence (numerically stable)
- Additional kernels: `hermite_function` (normalized Hermite × Gaussian), `radial_laguerre` (2D HO)
- Applications: quantum mechanics, Gaussian quadrature, nuclear wavefunctions

### 2. Broyden Mixing Module

**Location**: `crates/barracuda/src/ops/mixing/`

| File | Purpose |
|------|---------|
| `mod.rs` | Module documentation, presets |
| `broyden_f64.rs` | Rust implementation of LinearMixer and BroydenMixer |

**Shader**: `crates/barracuda/src/shaders/mixing/broyden_f64.wgsl`

**Kernels**:
- `mix_linear` — Simple damped iteration: x_new = (1-α)·x_old + α·x_computed
- `broyden_update` — Full Modified Broyden II with history
- `compute_residual` — Residual calculation F(x) = x_out - x_in
- `compute_diff_to_history` — History vector differences

**Applications**: DFT, HFB nuclear structure, Poisson-Boltzmann, coupled-cluster,
fixed-point iterations

### 3. Finite-Difference Gradient Module

**Location**: `crates/barracuda/src/ops/grid/`

| File | Purpose |
|------|---------|
| `mod.rs` | Module documentation |
| `fd_gradient_f64.rs` | Rust implementation of gradient operators |

**Shader**: `crates/barracuda/src/shaders/grid/fd_gradient_f64.wgsl`

**Kernels**:
- `gradient_1d` — 1D finite-difference gradient
- `gradient_2d` — 2D gradient (both components, row-major)
- `gradient_magnitude_2d` — |∇f|
- `laplacian_2d` — ∇²f = ∂²f/∂x² + ∂²f/∂y²
- `gradient_cylindrical` — (ρ, z) gradient for axial symmetry
- `laplacian_cylindrical` — ∇²f with 1/ρ term

**Applications**: Fluid dynamics, heat transfer, wave propagation, electrostatics,
nuclear physics, image processing

### 4. Weighted Inner Product Shader

**Location**: `crates/barracuda/src/shaders/reduce/weighted_dot_f64.wgsl`

**Kernels**:
- `weighted_dot_simple` — Sequential weighted dot product
- `weighted_dot_parallel` — Workgroup tree reduction (256-wide shared memory)
- `final_reduce` — Sum partial results
- `weighted_dot_batched` — Multiple dot products in parallel
- `dot_parallel` — Unweighted dot product
- `norm_squared_parallel` — Vector norm squared ||v||²

**Applications**: Galerkin methods, FEM assembly, spectral methods, correlation
computation, nuclear potential matrix elements

### 5. Science-Grade Buffer Limits

**Location**: `crates/barracuda/src/device/`

| Change | Before | After |
|--------|--------|-------|
| Default `max_storage_buffer_binding_size` | 128 MiB (wgpu default) | 512 MiB |
| Default `max_buffer_size` | 256 MiB (wgpu default) | 1 GiB |

**Files modified**:
- `tensor_context.rs` — Added `science_limits()` function
- `wgpu_device.rs` — Updated `new_with_filter()` and `from_adapter_index()` to use `science_limits()`
- `mod.rs` — Exported `science_limits`

**Rationale**: The default wgpu limits are too small for scientific computing.
hotSpring validated these higher limits on consumer GPU (RTX 4070) with full f64
precision across 2,042 nuclei.

---

## What Stayed in hotSpring (Physics-Specific)

The following shaders encode nuclear physics models and remain in hotSpring:

| Shader | Why Physics-Specific |
|--------|---------------------|
| `batched_hfb_potentials_f64.wgsl` | Skyrme EDF parameters (t0/t1/t2/t3/x0-x3/α) |
| `batched_hfb_hamiltonian_f64.wgsl` | Nuclear effective mass, l(l+1) angular momentum |
| `batched_hfb_density_f64.wgsl` | Nuclear pairing gaps, Fermi surface |
| `batched_hfb_energy_f64.wgsl` | Skyrme energy functional, Coulomb exchange |
| `deformed_potentials_f64.wgsl` | Skyrme + Coulomb on cylindrical grid |
| `deformed_density_energy_f64.wgsl` | Q20 quadrupole moment, nuclear β₂ |

These demonstrate that ToadStool's GPU primitives can support arbitrary
domain-specific physics. The same pattern works for fluid dynamics, protein
folding, ray tracing, and gaming physics.

---

## Validation Status

All absorbed primitives come from code that passed hotSpring's acceptance criteria:

| Capability | Validation |
|------------|------------|
| f64 GPU compute | 169/169 acceptance checks |
| Hermite polynomials | 2,042 nuclei × 200 SCF iterations |
| Laguerre polynomials | Deformed nuclear wavefunctions |
| Broyden mixing | HFB SCF convergence |
| Finite differences | Kinetic density τ, gradients |
| Workgroup reduction | Potential matrix elements |
| Buffer limits (512 MiB/1 GiB) | Overnight runs on RTX 4070 |

---

## Build Status

```
cargo check --workspace  # ✅ All crates compile
```

Warnings: Minor unused field warnings in grid operators (for future GPU implementations).

---

## Usage Examples

### Broyden Mixing

```rust
use barracuda::ops::mixing::{LinearMixer, MixingParams, presets};

// For warmup or simple problems
let params = presets::density_mixing();  // α=0.5, clamp_min=0
let mixer = LinearMixer::new(device, vec_dim, params)?;
let x_mixed = mixer.mix(&x_old, &x_computed).await?;
```

### Finite-Difference Gradient

```rust
use barracuda::ops::grid::Gradient1D;

let grad = Gradient1D::new(device, n, dx)?;
let df_dx = grad.compute(&f).await?;
```

### Science-Grade Device

```rust
// Now the default — no changes needed!
let device = WgpuDevice::new().await?;
// Gets 512 MiB storage buffer binding, 1 GiB max buffer

// For maximum capacity (1 GB / 2 GB)
let device = WgpuDevice::new_high_capacity().await?;
```

---

## Future Work (From Handoff)

Per the original handoff document, remaining items:

1. **ResourceQuota** — Per-task VRAM budget enforcement
2. **ComputePartition** — Fraction of GPU for a task (via dispatch limiting)
3. **WorkloadRouter** — Route tasks to best available device
4. **MultiDevicePool** — Manage heterogeneous GPU array

These enable the Strandgate vision: orchestrated multi-task GPU utilization.

---

## References

- `docs/planning/HOTSPRING_MD_HANDOFF_FEB14_2026.md` — GPU MD evolution handoff
- Original handoff document (in user message, Feb 12, 2026)
- hotSpring repository: `ecoPrimals/hotSpring/`
