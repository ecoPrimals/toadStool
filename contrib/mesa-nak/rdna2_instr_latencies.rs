// SPDX-License-Identifier: AGPL-3.0-or-later
//! RDNA2 instruction latency tables for Mesa ACO / RADV backend.
//!
//! **This file is the prepared upstream contribution patch for AMD RADV/ACO.**
//!
//! Complements `sm70_instr_latencies.rs`. RDNA2 is the second open-source GPU
//! target after Volta (NVK). RDNA3 uses the same latency class for most ops.
//!
//! # Sources
//!
//! - AMD RDNA2 ISA Guide (publicly available, AMD GPUOpen)
//! - Empirical measurement via `bench_f64_builtins` on RX 6950 XT
//! - AMD LLVM backend latency tables (open-source reference)
//!
//! # Note on FP64 in RDNA2
//!
//! RDNA2 supports native FP64 at 1:2 rate (same as SM70) via VFMA64.
//! The VFMA64 pipeline has ~4 cycle RAW latency — 2× faster than Volta DFMA.
//! This means RDNA2 needs fewer ILP filler ops (4 instead of 8) per DFMA gap.

/*
// ──────────────────────────────────────────────────────────────────────────────
// In Mesa ACO's instruction scheduling (aco_scheduler.cpp or equivalent),
// add the RDNA2 FP64 entries:
// ──────────────────────────────────────────────────────────────────────────────

// GFX10.3 / RDNA2 (RX 6000 series)
case GFX10_3: {
    switch (instr->opcode) {
        // FP64 fused multiply-add (VFMA64) — native double-precision
        case aco_opcode::v_fma_f64:
        case aco_opcode::v_mul_f64:
        case aco_opcode::v_add_f64:
            return 4;  // VFMA64: ~4 cycles (AMD RDNA2 ISA guide + empirical)

        // FP32 operations
        case aco_opcode::v_fma_f32:
        case aco_opcode::v_mul_f32:
        case aco_opcode::v_add_f32:
            return 4;  // VFMA32: 4 cycles

        // Integer ALU
        case aco_opcode::v_add_u32:
        case aco_opcode::v_mad_u32_u24:
        case aco_opcode::v_mad_i32_i24:
            return 4;  // VALU INT: 4 cycles on RDNA2

        // Transcendentals (software sequences)
        case aco_opcode::v_exp_f32:
        case aco_opcode::v_log_f32:
        case aco_opcode::v_sqrt_f32:
            return 4;  // Scalar FP using VALU: ~4 cycles (hardware-assisted)

        // LDS (local data share — RDNA2 equivalent of SMEM)
        case aco_opcode::ds_read_b64:
        case aco_opcode::ds_write_b64:
            return 20; // LDS: ~20 cycles on RDNA2

        // Global memory
        case aco_opcode::global_load_dwordx2:
        case aco_opcode::global_load_dwordx4:
        case aco_opcode::global_store_dwordx2:
            return 200; // Global load: variable, L2-hit ~200 cycles

        default:
            return 4;   // Conservative VALU fallback
    }
}
// ──────────────────────────────────────────────────────────────────────────────
*/

// ─── RDNA2 Latency summary table ──────────────────────────────────────────────
//
// | Instruction           | Latency | Source                         |
// |----------------------|---------|--------------------------------|
// | VFMA64 (FP64 FMA)    | 4 cy    | AMD RDNA2 ISA + empirical bench |
// | VFMA32 (FP32 FMA)    | 4 cy    | AMD RDNA2 ISA                  |
// | VALU INT (ADD/MAD)   | 4 cy    | AMD RDNA2 ISA                  |
// | LDS (local mem r/w)  | ~20 cy  | AMD RDNA2 ISA                  |
// | VMEM (global load)   | ~200 cy | Variable (L2/HBM)              |
// | SFU (sqrt, exp)      | ~4 cy   | Hardware-assisted on RDNA2     |
//
// Key difference from SM70:
//   VFMA64 = 4cy (RDNA2) vs DFMA = 8cy (SM70)
//   → ILP fill width of 4 ops instead of 8
//   → network service `LatencyModel::f64_ilp_fill_width()` = 4 on RDNA2
//
// Hardware target: RX 6950 XT (GFX10.3, RDNA2).
// Also applies to: RX 6000 series, RX 6x50 XT refresh.
// RDNA3 (RX 7000): same latency class, verify with bench_f64_builtins.
