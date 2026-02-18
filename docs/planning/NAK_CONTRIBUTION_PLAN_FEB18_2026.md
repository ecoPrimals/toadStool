# NAK Compiler Contribution Plan — SM70 Sovereign FP64 Compute

**Date**: 2026-02-18
**Status**: Research complete, Phase 1 ready to implement
**Author**: ToadStool / BarraCUDA team
**Context**: W-003 in DEBT.md — absorbing hotSpring GPU sovereignty analysis

---

## The Goal in One Sentence

If we solve NAK's SM70 compiler deficiencies, BarraCUDA's WGSL shaders run
at hardware FP64 peak throughput on ANY GPU with open-source drivers — NVK
(NVIDIA), RADV (AMD), and future open hardware — with zero proprietary dependencies.

**Current state**: NVK/NAK runs our Jacobi eigensolve ~9x slower than
proprietary PTXAS (after warp-packing). All five root causes are in Rust code
we can read, build, test, and contribute upstream.

---

## The Compilation Stack

```
our WGSL shaders        ← we own this (480+ shaders in crates/barracuda/src/shaders/)
    ↓ naga (Rust)       ← gfx-rs, contribute upstream: f64 FMA folding
  SPIR-V
    ↓ spirv_to_nir (C)  ← Mesa, thin translation layer
  NIR
    ↓ nak_from_nir (Rust)     ← Mesa NAK, PRIMARY TARGET
  NAK IR (SSA)
    ↓ NAK opt passes (Rust)   ← SM70 scheduling, FMA selection, loop unroll
  SASS machine code (SM70 Volta binary)
```

Every layer is modifiable. Our primary target is NAK (Mesa, Rust, AGPL-3.0-compatible).

---

## SM70 (Volta) Hardware Reality

### Titan V (GV100) Specifications
| Property | Value |
|----------|-------|
| SM count | 80 |
| FP32 TFLOPS | ~14.9 |
| FP64 TFLOPS | ~7.45 (1:2 ratio) |
| Warp schedulers / SM | 4 |
| CUDA cores / SM (FP32) | 64 |
| FP64 units / SM | 32 (16 per pair of warp schedulers) |
| Shared memory / SM | Up to 96KB |
| L2 cache | 4.5MB |
| Register file / SM | 256KB |

### SM70 Instruction Latencies (Public Data)
Sources: Volta Tuning Guide, arXiv:1804.06826 (Dissecting Volta), arXiv:2503.20481 (Modern NVIDIA cores)

| Instruction class | Latency (cycles) | Notes |
|------------------|-----------------|-------|
| FP32 FFMA         | 4               | Improved from 6 in Pascal (Volta Tuning Guide) |
| FP32 FADD/FMUL   | 4               | Same as FFMA |
| FP64 DFMA         | ~8              | 2× FP32, aligns with 1:2 throughput ratio |
| FP64 DADD/DMUL   | ~8              | Same functional units as DFMA |
| INT IADD/IMAD    | 6               | Standard ALU |
| MUFU (RSQ, SIN)  | ~16-20          | Special function unit |
| Shared mem LDS   | ~23             | Volta measured (1804.06826) |
| Shared mem STS   | ~20             | Write-to-read via SM |
| L1 cache hit     | ~33             | Tag lookup + data |
| L2 cache hit     | ~100-200        | Volta measured |
| Global LDG       | ~300-600        | Highly variable (DRAM latency) |

**Key for NAK**: NAK currently applies a blanket fallback delay instead of
per-instruction per-architecture latencies for SM70. This is what the SM32
(Kepler) latency fix addressed — we replicate it for SM70.

### Volta Dual-Issue Model
SM70 warp schedulers support issuing 2 independent instructions per cycle to
different execution units. The NVIDIA SASS encoding encodes:
- **Stall count** (5 bits): minimum cycles to wait before issuing next instruction
- **Yield flag**: hint to scheduler to switch to another warp
- **Read barrier**: which instructions to wait for before reading
- **Write barrier**: dependency tracking for register writes

Proprietary PTXAS exploits this via latency hiding: it schedules independent
work between dependent FP64 operations. NAK currently does not implement
dual-issue scheduling for any architecture.

---

## The Five Deficiencies (Detailed)

### Deficiency 1: No SM70 Instruction Scheduling (~3-4x impact)

**What's missing**: NAK has per-instruction latency tables for SM32 (Kepler,
added July 2025 by Lorenzo Rossi, Mesa 25.2). SM70 uses a blanket fallback.

**Evidence**: The Kepler fix achieved 2.5x PixMark Piano, 80% Talos Principle.
SM70 has longer FP64 latencies (8 cycles vs 4 for FP32), making scheduling
even more critical for our FP64-heavy Jacobi kernels.

**Fix location**: `src/nouveau/compiler/nak/calc_instr_deps.rs`

**Template**: Lorenzo Rossi's SM32 merge request is the exact pattern.
The SM32 work ported latency data from the old `codegen/` emitter.
For SM70, we use: arXiv:1804.06826 + Red Hat NDA docs (contact karolherbst/airlied).

**Implementation sketch**:
```rust
// In calc_instr_deps.rs — after SM32 (Kepler) block
SmVersion::Sm70 | SmVersion::Sm72 | SmVersion::Sm75 => {
    // Volta/Turing latency class
    match instr.op {
        Op::FAdd { .. } | Op::FMul { .. } | Op::FFma { .. }
            if matches!(instr.srcs[0].src_type, SrcType::F32) => {
            LatClass::Fixed(4) // FP32 FMA: 4 cycles (Volta Tuning Guide)
        }
        Op::DAdd { .. } | Op::DMul { .. } | Op::DFma { .. } => {
            LatClass::Fixed(8) // FP64 FMA: 8 cycles (arXiv:1804.06826)
        }
        Op::IAdd { .. } | Op::IMad { .. } => {
            LatClass::Fixed(6) // INT: 6 cycles
        }
        Op::Lop { .. } | Op::Shf { .. } => {
            LatClass::Fixed(6) // Logical/shift: 6 cycles
        }
        Op::ALd { .. } | Op::ASt { .. } => {
            LatClass::Fixed(23) // Shared memory: ~23 cycles
        }
        Op::Ld { .. } | Op::St { .. }
            if is_global_mem(&instr) => {
            LatClass::Variable { min: 50, typical: 300 } // L2 hit min
        }
        Op::Mufu { .. } => {
            LatClass::Fixed(16) // Special function unit
        }
        _ => LatClass::Fixed(6), // Safe conservative fallback
    }
}
```

**Validation**: Run `bench_wgsize_nvk.rs` before and after on Titan V.
Expected: 3-4x improvement in the Jacobi kernel. Validate with `hw_tests.rs`.

### Deficiency 2: No Dual-Issue Exploitation (~2x impact)

**What's missing**: NAK does not implement dual-issue scheduling for any
architecture. For SM70, each warp scheduler issues up to 2 independent
instructions per clock. Proprietary PTXAS aggressively exploits this.

**Why it matters for Jacobi**: The inner rotation loop body:
```wgsl
let new_akp = c * akp - s * akq;  // Compute 1: independent
let new_akq = s * akp + c * akq;  // Compute 2: same reads, independent result
A[base + k_p] = new_akp;          // Store 1: after Compute 1
A[base + k_q] = new_akq;          // Store 2: after Compute 2
```
Compute 1 and Compute 2 are independent — dual-issue opportunity on every iteration.

**Fix location**: New pass in `src/nouveau/compiler/nak/` (instruction pairing)
The SM32 Kepler work explicitly deferred this: "dual-issue and functional-unit
resource tracking" are listed as remaining work in the Mesa MR.

**Complexity**: HIGH. Requires:
1. Per-SM execution unit model (integer, FP32, FP64, SFU, load/store)
2. Dependency analysis pass to identify independent instruction pairs
3. SASS encoding update to emit paired instructions
4. Volta-specific: inter-warp-scheduler coordination model

**Plan**: Tackle AFTER Phase 1 (latency tables). Phase 1 is a prerequisite
because dual-issue decisions depend on correct latency information.

### Deficiency 3: Limited Loop Unrolling (~1.5-2x impact)

**What's missing**: NAK MR 26626 (Dec 2023) added basic loop unrolling.
Status unknown — may not handle nested loops or loops with variable bounds
that are actually bounded (our Jacobi n-loop has a compile-time max of 32).

**Jacobi pattern (our WGSL)**:
```wgsl
// n is a uniform param, but MAX_N=32 is a constant
// The k-loop is bounded by runtime n ≤ 32
for (var k = 0u; k < n; k = k + 1u) {
    // 8 global memory ops + 4 FP64 FMAs per iteration
}
```

**Fix location**: `src/nouveau/compiler/nak/` unrolling pass + possible
`src/nouveau/nir/` lowering for constant-bounded loops.

**Approach**: Investigate whether SPIR-V `OpLoopMerge` with a `PartialCount`
hint or naga's WGSL loop lowering can expose bounds to NAK. If not, contribute
a bounded-loop unrolling pass that handles `for k in 0..n where n ≤ MAX_N`.

### Deficiency 4: Missing f64 FMA Selection (~1.3-1.5x impact)

**What's missing**: SM70 has a native `DFMA` instruction (same latency as
DMUL+DADD in sequence, but one instruction → lower register pressure, better
scheduling). Whether naga or NAK currently emits DFMA vs separate DMUL+DADD
is unconfirmed and needs investigation.

**Jacobi FMA opportunities** (from our WGSL, every iteration):
```wgsl
// These patterns are fma(a, b, c) = a*b + c → should be DFMA
let new_akp = c * akp - s * akq;      // DFMA(c, akp, -s*akq)  or DFNMA(s, akq, c*akp)
let new_akq = s * akp + c * akq;      // DFMA(s, akp, c*akq)
let app_new = c*c*app - 2.0*c*s*apq + s*s*aqq;  // 2× DFMA chain
let aqq_new = s*s*app + 2.0*c*s*apq + c*c*aqq;  // 2× DFMA chain
```
In a batch=512, n=30, 200-sweep run, these patterns execute ~500M times.
Each DFMA saved vs DMUL+DADD = 1 less instruction, 8 cycles saved.

**Investigation method** (do this first):
```bash
# Compile our Jacobi pattern to SM70 SASS via CUDA on Godbolt
# Use: https://godbolt.org (NVCC 12.x, --gpu-architecture=sm_70)
# Compare: manual DMUL+DADD vs __fma_rn() — check if SASS shows DFMA or DMUL+DADD

# Then examine NAK output:
# Build Mesa with -D build-tests=true
# Run a WGSL shader through naga→NAK and dump SASS
# grep for DFMA vs DMUL in output
```

**Fix location**: Likely naga (`src/` in naga crate) for WGSL-level folding,
or NAK's `from_nir.rs` for NIR-level mul-add fusion, or a NAK optimization
pass for register-allocated IR.

**Note**: WGSL's `fma(a, b, c)` function maps to PTX `mad.rn.f64` → should
produce DFMA. But implicit patterns `a*b + c` may not be fused. This is the
key thing to verify.

### Deficiency 5: Generic Shared-Memory Scheduling (~1.5-2x impact)

**Assessment**: Our Jacobi shader intentionally avoids shared memory
(each thread reads its own matrix from global memory). This deficiency has
minimal impact on our specific workload. It matters more for GEMM-style ops
(our `gemm_f64.wgsl`) which do use shared memory for tiling.

**Priority**: LOW for Jacobi. Revisit when optimizing GEMM shaders.

---

## Phase 1 Implementation Plan: SM70 Latency Tables

This is our first concrete Mesa contribution. Low risk, well-defined scope,
follows an existing successful pattern.

### Prerequisites
1. **Build Mesa from source**:
   ```bash
   git clone https://gitlab.freedesktop.org/mesa/mesa.git
   cd mesa
   pip3 install meson
   meson setup build \
       -D gallium-drivers=nouveau \
       -D vulkan-drivers=nouveau \
       -D build-tests=true \
       -D buildtype=debug
   ninja -C build
   ```

2. **Get Red Hat latency docs** (authoritative for recent arches):
   Contact `karolherbst` or `airlied` on Mesa IRC/Matrix:
   `irc.oftc.net #nouveau` or Matrix `#nouveau:matrix.org`
   Ask for SM70 latency class data for `calc_instr_deps.rs`

3. **Validate with public sources**:
   - arXiv:1804.06826 "Dissecting the NVIDIA Volta GPU Architecture" (PDF)
     → Tables of FP64/FP32/INT latencies, measured on GV100 (Titan V)
   - arXiv:2503.20481 "Analyzing Modern NVIDIA GPU cores" (PDF)
     → Extended analysis, 2025, covers issue scheduler in detail
   - `nvdisasm --binary SM70` output from Godbolt (stall counts in control bytes)

### Finding the SM32 Template in `calc_instr_deps.rs`

The Kepler (SM32) latency work by Lorenzo Rossi is in the file.
Search for `SmVersion::Sm32` or `Sm35` in `calc_instr_deps.rs`.
The SM70 contribution mirrors this structure exactly.

Key NAK instruction opcodes relevant to our WGSL shaders:
```
FP64:  DAdd, DMul, DFma, DSetP, DMnmx
FP32:  FAdd, FMul, FFma (already handled for newer arches)
INT:   IAdd3, IMad, ISetP, IShf
MUFU:  (special functions — sqrt, rsq used in our Jacobi)
LDG/STG: Global memory loads/stores (no shared mem in Jacobi)
```

### Running NAK Hardware Tests on Titan V

```bash
cd build
VK_DRIVER_FILES="/usr/share/vulkan/icd.d/nvidia_icd.json:\
/opt/mesa-nvk/share/vulkan/icd.d/nouveau_icd.x86_64.json" \
BARRACUDA_GPU_ADAPTER=titan \
./src/nouveau/compiler/nak hw_tests
```

This runs `hw_tests.rs` which measures actual instruction latencies on the
hardware — the ground truth for our latency tables.

### Contribution Workflow

```bash
# 1. Create feature branch
git checkout -b nak-sm70-latency-tables

# 2. Edit calc_instr_deps.rs — add SM70 match arm
#    (see implementation sketch in Deficiency 1 above)

# 3. Run hardware tests
./build/src/nouveau/compiler/nak hw_tests 2>&1 | grep -E "SM70|PASS|FAIL"

# 4. Run nvdisasm tests (validates encoding against real nvdisasm)
./build/src/nouveau/compiler/nak nvdisasm_tests

# 5. Benchmark with our shader
cd /path/to/toadstool
BARRACUDA_GPU_ADAPTER=titan cargo run --release --bin bench_wgsize_nvk

# 6. Submit MR to Mesa
# Reference: Lorenzo Rossi's SM32 MR as precedent
# Link: https://gitlab.freedesktop.org/mesa/mesa/-/merge_requests/...
```

---

## Phase 2 Investigation: f64 FMA Selection

### Step 1: Godbolt Experiment

Compile this CUDA kernel to SM70 SASS via Godbolt (https://godbolt.org):

```cuda
// Compile with: nvcc --gpu-architecture=sm_70
__global__ void jacobi_rotation(double* a, double c, double s, int n) {
    int k = threadIdx.x;
    double akp = a[k];
    double akq = a[k + n];
    // Pattern 1: explicit fma
    a[k]     = fma(c, akp, -s * akq);
    a[k + n] = fma(s, akp,  c * akq);
    // Pattern 2: implicit a*b+c
    a[k]     = c * akp - s * akq;
    a[k + n] = s * akp + c * akq;
}
```

Look for `DFMA` vs `DMUL` + `DADD` in the SASS output for each pattern.
If Pattern 2 produces DFMA: PTXAS does the fusion. Then check if naga's
SPIR-V output for `wgsl: a*b+c` carries an `FP_FAST_FMAD` hint.

### Step 2: naga Investigation

Check if naga emits `OpFMul` + `OpFAdd` or `OpFma` for `a * b + c` in WGSL.
If it emits separate ops, add FMA fusion either in naga or in NAK's IR passes.

---

## Our WGSL Shader Targets

The following shaders are highest priority for NAK optimization (most FP64 ops):

| Shader | Key pattern | FP64 ops / call | NAK deficiency |
|--------|------------|-----------------|----------------|
| `batched_eigh_single_dispatch_f64.wgsl` | Jacobi rotation | ~500M (batch=512, n=30, 200sw) | 1,2,3,4 |
| `gemm_f64.wgsl` | Matrix multiply | ~batch×n³ | 1,2,4,5 |
| `bicgstab_gpu.wgsl` | Krylov iteration | ~iters×n | 1,2,4 |
| `broyden_f64.wgsl` | Quasi-Newton update | ~iters×n² | 1,4 |
| `cg_gpu.wgsl` | Conjugate gradient | ~iters×n | 1,4 |
| `cholesky.wgsl` | Triangular factor | ~n³/3 | 1,4 |
| `lu_gpu.wgsl` | LU decomposition | ~n³/3 | 1,4 |

---

## Measurement Plan

### Baseline (already done via `bench_wgsize_nvk.rs`)
```
Titan V (NVK/NAK), warp-packed wp32:
  n=30, batch=512, 200 sweeps: ~69.8ms
  n=20, batch=512, 200 sweeps: ~31.5ms
RTX 4070 (proprietary PTXAS):
  n=30, batch=512, 200 sweeps: ~7.4ms   (9.4x faster)
  n=20, batch=512, 200 sweeps: ~3.5ms   (9.0x faster)
```

### After Phase 1 (SM70 latency tables)
Expected: 3-4x improvement on Titan V
Target: ~17-23ms for n=30 (from 69.8ms)
Gap to proprietary: reduced from 9.4x → ~2.5-3x

### After Phase 2 (f64 FMA)
Expected: additional 1.3-1.5x
Target: ~12-18ms for n=30

### After Phase 3 (loop unrolling)
Expected: additional 1.5-2x
Target: ~6-12ms for n=30

### After Phase 4 (dual-issue)
Expected: additional 2x
Target: ~3-6ms for n=30 → approaching ~7.4ms proprietary baseline

---

## Key Contacts and Resources

### Mesa Community
- **karolherbst** (Red Hat, Mesa/NVK lead): has NDA SM70 latency docs
  - IRC: `irc.oftc.net #nouveau`
  - Matrix: `#nouveau:matrix.org`
- **Lorenzo Rossi**: authored SM32 latency MR — template author, reference for contribution style

### Primary Papers
1. **arXiv:1804.06826** — "Dissecting the NVIDIA Volta GPU Architecture via Microbenchmarking"
   Jia, Maggioni, Staiger, Scarpazza (2018). Tables of SM70 instruction latencies.
   PDF: https://arxiv.org/pdf/1804.06826
   Use for: FP64 DADD/DMUL/DFMA latency values, memory hierarchy data.

2. **arXiv:2503.20481** — "Analyzing Modern NVIDIA GPU cores"
   Huerta, Shoushtary, Cruz, González (March 2025). Modern NVIDIA microarchitecture.
   PDF: https://arxiv.org/pdf/2503.20481
   Use for: Issue scheduler model, register file structure, dependence management.

3. **Volta Architecture Whitepaper**
   https://images.nvidia.com/content/volta-architecture/pdf/volta-architecture-whitepaper.pdf
   Use for: Independent Thread Scheduling section, FP64 unit layout.

4. **CUDA Binary Utilities (11.8.0 archive)**
   https://docs.nvidia.com/cuda/archive/11.8.0/cuda-binary-utilities/
   Use for: nvdisasm SM70 output, stall count encoding, ISA reference.

### Tools
- **Godbolt** (https://godbolt.org) — CUDA → SM70 SASS, reference PTXAS output
- **nvdisasm** — disassemble SM70 cubins, inspect control bytes and stall counts
- **hw_tests.rs** — Mesa NAK hardware test infrastructure (micro-benchmark latencies)
- **NMSU-PEARL/GPUs-ISA-Latencies** — microbenchmark suite (reference, HPEC '19)

### Mesa Repository
- `src/nouveau/compiler/nak/calc_instr_deps.rs` — latency tables (PRIMARY TARGET)
- `src/nouveau/compiler/nak/from_nir.rs` — NIR→NAK-IR lowering (FMA selection)
- `src/nouveau/compiler/nak/hw_tests.rs` — hardware test validation
- `src/nouveau/compiler/nak/nvdisasm_tests.rs` — encoding validation
- `src/nouveau/compiler/nak/ir.rs` — NAK IR definition, `Foldable` impls

---

## Strategic Context

NAK is a pure Rust compiler in a repo we can fork, build, and contribute to.
Every SM70 improvement we contribute:
1. Benefits all NVK users worldwide (open-source multiplier)
2. Validates ecoPrimals' "sovereignty in practice" principle
3. Makes hotSpring's Titan V (and equivalent Volta GPUs) first-class compute targets
4. Creates a pattern to then apply to AMD RADV/ACO (SM70 → RDNA3 second target)

The SM32 Kepler latency fix took one contributor, resulted in 2.5x speedup,
and was merged into Mesa 25.2. The SM70 fix is the same scope, same file,
same structure — and our team has hardware (Titan V) and benchmarks in place.

**This is not a research project. It is a well-defined engineering task:**
port the SM32 latency tables to SM70, validate on hardware, submit MR.

---

## Next Actions (Ordered)

1. **Contact karolherbst/airlied** on Mesa IRC for NDA SM70 latency doc access
2. **Read arXiv:1804.06826 PDF** — extract SM70 FP64 latency table
3. **Clone Mesa**, build with `-D build-tests=true -D gallium-drivers=nouveau`
4. **Find SM32 block in `calc_instr_deps.rs`** — study structure, copy to SM70
5. **Fill in SM70 latencies** from Red Hat docs / arxiv paper
6. **Run `hw_tests.rs` on Titan V** — validate latency values against hardware
7. **Run `bench_wgsize_nvk.rs`** — measure impact (target: 3-4x improvement)
8. **Godbolt experiment** for f64 FMA investigation (Phase 2 prep)
9. **Submit MR to Mesa** with SM70 latency tables
10. **Update W-003 in DEBT.md** — Phase 1 complete, begin Phase 2

---

*Cross-reference: DEBT.md W-003, docs/planning/SOFTWARE_UNIDIRECTIONAL_SIMULATION_FEB17_2026.md*
*Hardware: hotSpring Titan V (NVK) + RTX 4070 (proprietary) test setup*
