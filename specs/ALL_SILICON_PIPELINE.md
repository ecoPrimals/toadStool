# All-Silicon Pipeline Specification

**Date**: March 18, 2026 — S159
**Status**: Phase B foundation landed; Phases C-D planned
**Depends on**: `wateringHole/GPU_FIXED_FUNCTION_SCIENCE_REPURPOSING.md`

---

## Summary

A modern GPU die has at least eight distinct hardware units beyond shader
cores. Each was designed for a specific graphics operation but actually
computes a general mathematical function. toadStool's all-silicon pipeline
discovers, profiles, and routes work to every unit — not just shader cores.

The DF64 discovery proved the pattern: fp32 ALUs "designed for pixel colors"
emulate fp64 at 8-16x the throughput of native fp64. The rasterizer is a
spatial query engine. The depth buffer is a min-reducer. The ROPs are
scatter-adders. Every unit is a hidden computer.

**Goal**: A single RTX 3090 delivering 50-100 effective TFLOPS (vs 0.33
native fp64 today) by using all silicon in parallel on different parts of
the problem. No existing framework (CUDA, Vulkan, OpenCL, Kokkos) supports
this.

---

## Silicon Unit Model

### `SiliconUnit` Enum

Every functional unit on the GPU die that can execute science work:

```rust
pub enum SiliconUnit {
    ShaderCore,    // FP arithmetic — compute shaders, DF64
    TensorCore,    // Matrix multiply-accumulate (MMA) — CG solver, convolution
    RtCore,        // BVH spatial query — neighbor search, Monte Carlo transport
    TextureUnit,   // 2D interpolated lookup — EOS tables, activation functions
    Rop,           // Per-pixel scatter-add / min / max — histograms, deposition
    Rasterizer,    // Point-in-polygon + barycentric interp — voxelization, binning
    DepthBuffer,   // Per-pixel min reduction — Voronoi, distance fields
    Tessellator,   // Adaptive mesh subdivision — AMR, FEM refinement
    VideoEncoder,  // Block transform coding — simulation compression
}
```

### `SiliconCapabilities` Struct

Discovered per GPU at runtime. Attached to `GpuAdapterInfo`:

```rust
pub struct SiliconCapabilities {
    pub has_tensor_cores: bool,
    pub has_rt_cores: bool,
    pub has_video_encoder: bool,
    pub tensor_core_gen: Option<TensorCoreGen>,
    pub rt_core_gen: Option<RtCoreGen>,
    pub estimated_tmu_count: u32,
    pub estimated_rop_count: u32,
    pub rasterizer_available: bool,
    pub tessellator_available: bool,
}
```

### `TensorCoreGen` and `RtCoreGen`

Generation-specific capability data for tensor and RT cores:

```rust
pub enum TensorCoreGen {
    Volta,      // SM 7.0 — FP16 MMA only
    Turing,     // SM 7.5 — FP16, INT8, INT4
    Ampere,     // SM 8.0+ — FP16, BF16, TF32, FP64, INT8
    Ada,        // SM 8.9 — FP8 added
    Hopper,     // SM 9.0 — FP8, transformer engine
}

pub enum RtCoreGen {
    Turing,     // 1st gen — ray-triangle intersection
    Ampere,     // 2nd gen — triangle + concurrent RT/shader
    Ada,        // 3rd gen — opacity micro-maps, displaced micro-meshes
}
```

---

## Performance Surface Database (Phase B)

The performance surface maps `(operation, silicon_unit, precision)` → measured
throughput. Built from spring experiment data reported via JSON-RPC.

### Data Model

```rust
pub struct PerformanceMeasurement {
    pub operation: Arc<str>,         // "math.pairwise.yukawa"
    pub silicon_unit: SiliconUnit,   // RtCore
    pub precision_mode: Arc<str>,    // "fp16", "fp32", "tf32", "df64"
    pub throughput_gflops: f64,      // measured
    pub tolerance_achieved: f64,     // 1e-7
    pub gpu_model: Arc<str>,         // "RTX 3090"
    pub measured_by: Arc<str>,       // "hotSpring exp076"
    pub timestamp: u64,              // epoch seconds
}

pub struct PerformanceSurfaceEntry {
    pub operation: Arc<str>,
    pub tolerance_required: f64,
    pub recommended_unit: SiliconUnit,
    pub recommended_precision: Arc<str>,
    pub estimated_throughput_gflops: f64,
    pub fallback_unit: SiliconUnit,
    pub fallback_throughput_gflops: f64,
}
```

### JSON-RPC Methods

```
compute.performance_surface.report {
  "operation": "math.pairwise.yukawa",
  "silicon_unit": "rt_core",
  "precision_mode": "fp32",
  "throughput_gflops": 5400.0,
  "tolerance_achieved": 1e-7,
  "gpu_model": "RTX 3090",
  "measured_by": "hotSpring exp076"
}

compute.performance_surface.query {
  "operation": "math.pairwise.yukawa",
  "tolerance_required": 1e-14,
  "available_units": ["shader_core", "tensor_core", "tmu"]
}
→ {
  "recommended_unit": "shader_core",
  "recommended_precision": "df64",
  "estimated_throughput_gflops": 3240,
  "fallback_unit": "shader_core",
  "fallback_precision": "fp64_native",
  "fallback_throughput_gflops": 330
}

compute.performance_surface.list {}
→ { "entries": [...], "gpu_models": [...], "operations": [...] }
```

---

## Multi-Unit Routing (Phase C)

Extends dispatch routing from "which GPU" to "which unit ON the GPU."

### Routing Decision Model

```rust
pub struct MultiUnitRoutingPlan {
    pub operations: Vec<RoutedOperation>,
    pub total_estimated_throughput_gflops: f64,
    pub gpu_target: Arc<str>,
}

pub struct RoutedOperation {
    pub operation: Arc<str>,
    pub silicon_unit: SiliconUnit,
    pub precision_mode: Arc<str>,
    pub estimated_throughput_gflops: f64,
    pub fallback: Option<Box<RoutedOperation>>,
    pub reason: Arc<str>,
}
```

### JSON-RPC Method

```
compute.route.multi_unit {
  "workload": [
    { "op": "neighbor_search", "tolerance": 1e-2, "data_size": 1000000 },
    { "op": "force_eval", "tolerance": 1e-14, "data_size": 1000000 },
    { "op": "accumulation", "tolerance": 1e-7, "data_size": 1000000 }
  ]
}
→ {
  "plan": [
    { "op": "neighbor_search", "unit": "rt_core", "precision": "fp32",
      "reason": "spatial query, 10x over compute",
      "fallback": { "unit": "shader_core", "precision": "fp32" } },
    { "op": "force_eval", "unit": "shader_core", "precision": "df64",
      "reason": "14-digit tolerance requires DF64" },
    { "op": "accumulation", "unit": "rop", "precision": "fp32",
      "reason": "additive scatter, 5x over atomics",
      "fallback": { "unit": "shader_core", "precision": "fp32" } }
  ],
  "total_estimated_tflops": 12.4,
  "gpu": "RTX 3090"
}
```

### Graceful Degradation

Every routing decision has a fallback. The math is the same; the throughput
changes:

| Primary | Fallback | Reason |
|---------|----------|--------|
| RT core BVH | Compute BVH (shader) | GPU lacks RT cores (MI50, Titan V) |
| Tensor core MMA | Shader core matmul | GPU lacks tensor cores |
| TMU table lookup | Compute evaluation | Graphics pipeline unavailable |
| ROP scatter-add | Atomic add (shader) | Blend mode unavailable |
| Rasterizer binning | Compute loop | Draw commands unavailable |

---

## Mixed Command Streams (Phase D)

The sovereign VFIO path submits compute dispatches via PBDMA. For fixed-function
units, toadStool extends this with additional command types:

| Command Type | GPU Mechanism | toadStool Role |
|---|---|---|
| Compute dispatch | PBDMA `DISPATCH(groups)` | Current path (Phase A) |
| Draw (rasterizer) | PBDMA `DRAW(vertices)` | Vertex buffer + pipeline state |
| RT trace | PBDMA `TRACE_RAY(...)` | BVH + ray generation |
| Framebuffer ops | ROP config registers | Blend mode, depth func |
| Texture binding | TMU descriptor | Texture handle + sampler |
| Tensor MMA | SM ISA (`HMMA`/`IMMA`) | coralReef emits MMA instructions |

This is NOT a rewrite — it extends the existing PBDMA submission with
additional command types. The submission mechanism is identical; the payloads
differ per hardware unit.

---

## Phase Dependencies

```
Phase A: Sovereign compute dispatch (VFIO shader cores)
  CURRENT — blocked on coralReef FECS firmware loading
  Unlocks: sovereign shader compute for all springs

Phase B: Silicon discovery + performance surface database
  S159 FOUNDATION — types and JSON-RPC methods landed
  Unlocks: toadStool knows every unit and measured throughput
  Does NOT require Phase A — uses wgpu feature queries + sysfs probing

Phase C: Tolerance-based multi-unit routing
  Requires: Phase B (performance surface data from spring experiments)
  Unlocks: single workload splits across multiple units

Phase D: Mixed command streams
  Requires: Phase A (VFIO) + Phase C (routing decisions)
  Unlocks: all silicon active in single dispatch — 50-100 TFLOPS
```

**Key insight**: Phase B does NOT depend on Phase A. We can discover silicon
capabilities and build the performance surface database using wgpu feature
queries, sysfs probing, and spring experiment data — all without VFIO.
Springs can begin their hardware experiments (per ludoSpring V24 assignments)
and report measured data to toadStool now.

---

## Cross-Ecosystem Flow

1. **Spring discovers**: hotSpring validates RT core neighbor finding (exp076)
2. **Spring reports**: `compute.performance_surface.report` to toadStool
3. **toadStool routes**: Future `math.pairwise.yukawa` with tolerance 1e-2 →
   RT cores for neighbor search (10x over compute)
4. **barraCuda absorbs**: New `math.spatial.neighbor_rt` dispatch op
5. **coralReef compiles**: RT pipeline state + BVH build instructions
6. **All springs benefit**: `math.spatial.neighbor_rt` works for wetSpring SPH,
   groundSpring seismic, healthSpring dosimetry, ludoSpring fog of war

Every hardware discovery in one spring creates a reusable primitive for all.
Read across a row: one discovery, eight beneficiaries.

---

**References**:
- `wateringHole/GPU_FIXED_FUNCTION_SCIENCE_REPURPOSING.md` — Full analysis
- `wateringHole/TOADSTOOL_LEVERAGE_GUIDE.md` Section 11 — Silicon map
- `crates/toadstool-core/src/silicon.rs` — `SiliconUnit`, `SiliconCapabilities`
- `crates/runtime/universal/src/backends/wgpu_backend/types.rs` — `GpuAdapterInfo`
