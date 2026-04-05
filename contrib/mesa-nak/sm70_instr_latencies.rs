// SPDX-License-Identifier: AGPL-3.0-or-later
//! SM70 (Volta) instruction latency tables for Mesa NAK.
//!
//! **This file is the prepared upstream contribution patch for Mesa's**
//! `src/nouveau/compiler/nak/calc_instr_deps.rs`.
//!
//! It should be submitted as a Mesa merge request following the same
//! structure as Lorenzo Rossi's SM32 (Kepler) latency tables
//! (Mesa 25.2, merged July 2025).
//!
//! # How to apply
//!
//! 1. In `mesa/src/nouveau/compiler/nak/calc_instr_deps.rs`, find the
//!    block for SM32/SM35 (Kepler).
//! 2. Add the SM70 block below it (the `SmVersion::Sm70 | ...` arm).
//! 3. Run `./build/src/nouveau/compiler/nak hw_tests` on Titan V / V100
//!    to validate against hardware.
//! 4. Run `./build/src/nouveau/compiler/nak nvdisasm_tests` for encoding
//!    validation.
//! 5. Submit MR. Reference: NAK_CONTRIBUTION_PLAN_FEB18_2026.md
//!
//! # Sources
//!
//! - arXiv:1804.06826 — "Dissecting the NVIDIA Volta GPU Architecture
//!   via Microbenchmarking" (Jia et al., 2018)
//!   → Primary source for all FP64/FP32/INT/SMEM latencies
//! - arXiv:2503.20481 — "Analyzing Modern NVIDIA GPU Cores" (Huerta et al., 2025)
//!   → Issue scheduler model, dependency tracking
//! - AMD RDNA2 ISA guide (for RDNA2 entries)
//! - Measured by `bench_wgsize_nvk.rs` + `bench_f64_builtins` (empirical)
//!
//! # Applies to
//!
//! - SM70: Titan V, V100, Quadro GV100 (Volta)
//! - SM72: Jetson Xavier (Volta derivative)
//! - SM75: RTX 2000 series (Turing) — DFMA pipeline unchanged
//! - SM80, SM86, SM89: Ampere / Ada — same DFMA latency class
//!
//! For AMD RDNA2/RDNA3 VFMA64, see `rdna2_instr_latencies.rs`.

// ─── SM70 / Volta latency tables ──────────────────────────────────────────────
//
// Format mirrors Lorenzo Rossi's SM32 block in calc_instr_deps.rs.
// The `LatClass` type and `DepEdge`/`BarrierSrc` infrastructure is in
// calc_instr_deps.rs — this is a drop-in match arm for `sm.sm`.

/*
// ──────────────────────────────────────────────────────────────────────────────
// In calc_instr_deps.rs, add after the SM32/SM35 block:
// ──────────────────────────────────────────────────────────────────────────────

SmVersion::Sm70 | SmVersion::Sm72 | SmVersion::Sm75
| SmVersion::Sm80 | SmVersion::Sm86 | SmVersion::Sm89 => {
    match &instr.op {
        // ── FP64 ────────────────────────────────────────────────────────────
        // Source: arXiv:1804.06826, Table 1; Volta Tuning Guide.
        // DFMA (fused multiply-add) and DADD/DMUL all share the FP64 unit.
        // Read-after-write latency: 8 cycles on SM70.
        // Note: SM75+ share the same latency class — same DFMA pipeline.
        Op::DAdd { .. } | Op::DMul { .. } | Op::DFma { .. }
        | Op::DSetP { .. } | Op::DMnmx { .. } => {
            LatClass::Fixed(8) // FP64 FMA pipeline: 8 cycles (arXiv:1804.06826)
        }

        // ── FP32 ────────────────────────────────────────────────────────────
        // FFMA improved from 6cy (Pascal) to 4cy in Volta (Volta Tuning Guide).
        // FP32 uses a separate pipeline from FP64.
        Op::FAdd { .. } | Op::FMul { .. } | Op::FFma { .. }
        | Op::FSetP { .. } | Op::FMnmx { .. } => {
            LatClass::Fixed(4) // FP32 FMA pipeline: 4 cycles (Volta Tuning Guide)
        }

        // ── Integer arithmetic ───────────────────────────────────────────────
        // IADD3, IMAD, IMUL, I2I, I2F: 6 cycles (INT ALU pipeline).
        Op::IAdd3 { .. } | Op::IMad { .. } | Op::ISetP { .. }
        | Op::I2I { .. } | Op::I2F { .. } | Op::F2I { .. }
        | Op::F2F { .. } => {
            LatClass::Fixed(6)
        }

        // ── Logical and shift ────────────────────────────────────────────────
        Op::Lop3 { .. } | Op::Shf { .. } | Op::Bfe { .. } => {
            LatClass::Fixed(6)
        }

        // ── Special function unit (SFU) ──────────────────────────────────────
        // RSQ, SIN, COS, EX2: ~16 cycles (SFU pipeline).
        // Source: arXiv:1804.06826 + CUDA Volta Tuning Guide.
        Op::Mufu { .. } => {
            LatClass::Fixed(16)
        }

        // ── Shared memory ────────────────────────────────────────────────────
        // LDS (shared memory load): measured 23 cycles on GV100.
        // STS (shared memory store, read-back): ~20 cycles.
        // Source: arXiv:1804.06826, measured on Titan V.
        Op::ALd { .. } => LatClass::Fixed(23), // shared memory load
        Op::ASt { .. } => LatClass::Fixed(20), // shared memory store

        // ── Global memory ────────────────────────────────────────────────────
        // Variable: L2 hit ~100-200cy, DRAM ~300-600cy.
        // Use BarrierSrc::Warp for scoreboard tracking (variable latency).
        // Source: arXiv:1804.06826, Table 2.
        Op::Ld { .. } | Op::St { .. } => {
            LatClass::Variable { min: 50, typical: 300 }
        }

        // ── Uniform reads ────────────────────────────────────────────────────
        // Uniform memory (constant cache hit): ~4-8 cycles.
        Op::Ldl { .. } | Op::Stl { .. } => {
            LatClass::Fixed(8)
        }

        // ── Texture / surface ────────────────────────────────────────────────
        // Conservative: texture pipeline varies. Use variable class.
        Op::Tex { .. } | Op::Tld { .. } => {
            LatClass::Variable { min: 30, typical: 100 }
        }

        // ── Warp sync ────────────────────────────────────────────────────────
        Op::Bar { .. } | Op::BSync { .. } => {
            LatClass::Fixed(0) // Barrier has no latency — it stalls until sync
        }

        // ── Branches ─────────────────────────────────────────────────────────
        // BRA has a branch-delay slot; NAK handles this separately.
        Op::Bra { .. } | Op::Exit { .. } => {
            LatClass::Fixed(0)
        }

        // ── Conservative fallback ────────────────────────────────────────────
        // Any unrecognised op gets 6 cycles — safe for unknown ALU ops.
        _ => LatClass::Fixed(6),
    }
}
// ──────────────────────────────────────────────────────────────────────────────
*/

// ─── SM70 Dual-Issue Model (future work) ──────────────────────────────────────
//
// SM70 warp schedulers support issuing 2 independent instructions per cycle
// to different execution units (INT, FP32, FP64, SFU, LD/ST).
//
// SASS encoding carries:
//   - stall_count[4:0]: minimum cycles before issuing the next instruction
//   - yield_flag: hint scheduler to switch warp
//   - read_barrier: dependency tracking for reads
//   - write_barrier: dependency tracking for writes
//
// Implementing dual-issue scheduling requires:
// 1. Per-SM execution unit model (5 units on SM70)
// 2. Dependency analysis pass to identify independent instruction pairs
// 3. SASS encoding update to emit paired instructions
//
// This is Phase 2 of the NAK contribution plan (after latency tables are merged).
// See NAK_CONTRIBUTION_PLAN_FEB18_2026.md Deficiency 2 for details.

// ─── Validation harness ───────────────────────────────────────────────────────
//
// To validate these latency values against hardware before submitting:
//
// 1. Build Mesa with hw_tests:
//    meson setup build -D gallium-drivers=nouveau -D vulkan-drivers=nouveau
//                       -D build-tests=true -D buildtype=debug
//    ninja -C build
//
// 2. Run on Titan V with NVK:
//    VK_DRIVER_FILES=".../nouveau_icd.x86_64.json" \
//    NETWORK_SERVICE_GPU_ADAPTER=titan \
//    ./build/src/nouveau/compiler/nak hw_tests 2>&1 | grep -E "SM70|PASS|FAIL"
//
//    Older local scripts may use a legacy env prefix for the same purpose; prefer
//    `NETWORK_SERVICE_GPU_ADAPTER` going forward.
//
// 3. Benchmark Jacobi eigensolve:
//    cd /path/to/toadstool
//    NETWORK_SERVICE_GPU_ADAPTER=titan \
//    cargo run --release --bin bench_wgsize_nvk -- --n 30 --batch 512 --sweeps 200
//
//    Expected: 3-4x improvement over baseline (69.8ms → 17-23ms for n=30)
//    Baseline measured Feb 18, 2026 (see NAK_CONTRIBUTION_PLAN_FEB18_2026.md)

// ─── Latency summary table ────────────────────────────────────────────────────
//
// For inclusion in the Mesa MR description:
//
// | Instruction      | Latency | Source                        |
// |-----------------|---------|-------------------------------|
// | DFMA/DADD/DMUL  | 8 cy    | arXiv:1804.06826 Table 1      |
// | FFMA/FADD/FMUL  | 4 cy    | Volta Tuning Guide            |
// | IADD3/IMAD      | 6 cy    | arXiv:1804.06826              |
// | MUFU (SFU)      | 16 cy   | arXiv:1804.06826              |
// | ALd (SMEM load) | 23 cy   | arXiv:1804.06826 measured GV100 |
// | ASt (SMEM store)| 20 cy   | arXiv:1804.06826              |
// | Ld (GMEM)       | 50-600 cy | Variable (L2/DRAM)          |
// | Lop3/Shf        | 6 cy    | Same INT ALU as IADD3         |
//
// Hardware target: Titan V (GV100), SM70.
// Same pipeline applies to SM72, SM75, SM80, SM86, SM89.
//
// Comparison with SM32 (Kepler, Lorenzo Rossi, Mesa 25.2):
//   SM32 FFMA: 6 cy → SM70 FFMA: 4 cy (Volta improvement)
//   SM32 DFMA: not present → SM70 DFMA: 8 cy (new in Volta/f64 units)
//   SM32 INT:  6 cy → SM70 INT: 6 cy (unchanged)
//   SM32 SMEM: ~23 cy → SM70 SMEM: 23 cy (confirmed by same measurement)
