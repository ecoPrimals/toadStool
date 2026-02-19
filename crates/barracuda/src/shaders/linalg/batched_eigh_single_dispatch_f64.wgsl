// Single-Dispatch Batched Symmetric Eigenvalue Decomposition (f64)
//
// CRITICAL EVOLUTION: Eliminates the per-rotation queue.submit() bottleneck.
// 
// Previous implementation: 4 dispatches × n(n-1)/2 rotations × max_sweeps = ~8000 submits
// This implementation: 1 dispatch total
//
// Architecture (Warp-Packed):
// - 32 threads per workgroup, each thread processes ONE independent matrix
// - No barriers, no cooperation — each thread owns its own matrix in global memory
// - ALL Jacobi sweeps run in a loop INSIDE the shader
// - Only ONE queue.submit() needed for the entire batch
//
// Warp-packing rationale (hotSpring cross-GPU analysis Feb 18, 2026):
// - @workgroup_size(1,1,1) wastes 31/32 SIMD lanes on NVIDIA GPUs
// - @workgroup_size(32,1,1) fills the full warp with independent work
// - Measured 2.2x NVK speedup (Titan V), neutral on proprietary drivers
// - Dispatch: batch.div_ceil(32) workgroups instead of batch workgroups
//
// Memory: Each thread accesses its own n×n matrix in global memory.
//         No shared memory used — avoids shared memory size limits entirely.
//         Supports MAX_N=32 regardless of warp size.
//
// Use case: hotSpring HFB (n=12, batch=40) completes in 1 dispatch vs 7920 dispatches
//
// ILP Phase 1 (Feb 19, 2026) — Sovereign Compute Evolution:
// The rotation kernel has been restructured for instruction-level parallelism.
// On SM70 (Titan V / V100), DFMA has 8-cycle latency. Naïve:
//   c * akp - s * akq   -- stalls 8cy waiting for c*akp to resolve
// After ILP restructuring:
//   Independent values (cc, ss, two_cs, vkp, vkq) are computed during the
//   8-cycle latency window so the hardware scoreboard never stalls.
//   This is expressed at WGSL source level → works on NVIDIA, AMD, Intel, Apple.
//
// @ilp_region annotations mark sections for the Phase 3 WgslDependencyGraph
// reorderer (SOVEREIGN_COMPUTE_EVOLUTION.md Phase 3).
//
// Reference: hotSpring handoff Feb 12, 2026 - TIER 1.1
//            hotSpring GPU sovereignty analysis Feb 18, 2026
//            SOVEREIGN_COMPUTE.md Phase 1

// Maximum matrix dimension supported
// Each thread works directly in global memory (no shared memory needed)
const MAX_N: u32 = 32u;
const WARP_SIZE: u32 = 32u;

struct SingleDispatchParams {
    n: u32,           // Matrix dimension (must be <= MAX_N)
    batch_size: u32,  // Number of matrices
    max_sweeps: u32,  // Maximum Jacobi sweeps
    tolerance: f32,   // Convergence tolerance for off-diagonal
}

@group(0) @binding(0) var<uniform> params: SingleDispatchParams;
@group(0) @binding(1) var<storage, read_write> A_batch: array<f64>;  // [batch × n × n]
@group(0) @binding(2) var<storage, read_write> V_batch: array<f64>;  // [batch × n × n]
@group(0) @binding(3) var<storage, read_write> eigenvalues: array<f64>;  // [batch × n]

// Each thread works on its own matrix directly in global memory.
// No shared memory needed — each lane is fully independent.

// Helper: 2D to 1D index
fn idx2d(row: u32, col: u32, n: u32) -> u32 {
    return row * n + col;
}

// Helper: Global memory offset for batch
fn batch_offset(batch_idx: u32, n: u32) -> u32 {
    return batch_idx * n * n;
}

// Warp-packed eigensolve: 32 threads per workgroup, each owns one matrix
// Dispatch with (batch_size.div_ceil(32), 1, 1) workgroups
@compute @workgroup_size(32, 1, 1)
fn batched_eigh_single_dispatch(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let batch_idx = wg_id.x * WARP_SIZE + local_id.x;
    let n = params.n;
    
    if (batch_idx >= params.batch_size || n > MAX_N) {
        return;
    }
    
    let base = batch_offset(batch_idx, n);
    let tol = f64(params.tolerance);
    
    // Step 1: Initialize V = Identity directly in global memory
    // (A_batch already contains input matrices)
    for (var i = 0u; i < n; i = i + 1u) {
        for (var j = 0u; j < n; j = j + 1u) {
            if (i == j) {
                V_batch[base + idx2d(i, j, n)] = f64(1.0);
            } else {
                V_batch[base + idx2d(i, j, n)] = f64(0.0);
            }
        }
    }
    
    // Step 2: Jacobi sweeps - ALL iterations run here
    // Each thread operates on its own matrix in global memory (no shared memory needed)
    for (var sweep = 0u; sweep < params.max_sweeps; sweep = sweep + 1u) {
        // Check convergence: max off-diagonal element
        var max_off = f64(0.0);
        for (var i = 0u; i < n; i = i + 1u) {
            for (var j = i + 1u; j < n; j = j + 1u) {
                let off = abs(A_batch[base + idx2d(i, j, n)]);
                if (off > max_off) {
                    max_off = off;
                }
            }
        }
        
        // Early exit if converged
        if (max_off < tol) {
            break;
        }
        
        // Cyclic Jacobi: iterate through all (p, q) pairs
        for (var p = 0u; p < n - 1u; p = p + 1u) {
            for (var q = p + 1u; q < n; q = q + 1u) {
                let apq = A_batch[base + idx2d(p, q, n)];
                
                // Skip if already zero
                if (abs(apq) < 1e-14) {
                    continue;
                }
                
                let app = A_batch[base + idx2d(p, p, n)];
                let aqq = A_batch[base + idx2d(q, q, n)];
                
                // Compute rotation angle
                let diff = aqq - app;
                var t: f64;
                
                if (abs(diff) < 1e-14) {
                    // app ≈ aqq
                    if (apq >= 0.0) { t = f64(1.0); } else { t = f64(-1.0); }
                } else {
                    // tan(2θ) = 2*apq / (aqq - app)
                    let phi = diff / (2.0 * apq);
                    let abs_phi = abs(phi);
                    if (phi >= 0.0) {
                        t = f64(1.0) / (abs_phi + sqrt(f64(1.0) + phi * phi));
                    } else {
                        t = f64(-1.0) / (abs_phi + sqrt(f64(1.0) + phi * phi));
                    }
                }
                
                let c = f64(1.0) / sqrt(f64(1.0) + t * t);
                let s = t * c;

                // @ilp_region begin
                // Hoist scalar products outside the inner loops. On SM70 each DFMA
                // takes 8 cycles; these are computed ONCE per (p,q) pair and reused
                // n times inside the loops — filling the latency window for free.
                let cc      = c * c;
                let ss      = s * s;
                let two_cs  = 2.0 * c * s;
                // Independent of cc/ss/two_cs — start during their latency windows:
                let app_new = cc * app - two_cs * apq + ss * aqq;
                let aqq_new = ss * app + two_cs * apq + cc * aqq;
                // @ilp_region end

                // Apply rotation to A (rows and columns p, q).
                // @ilp_region begin
                // Load both akp and akq before computing — independent loads issued
                // in parallel, hardware can coalesce or dual-issue them.
                // The DFMA for new_akp is computed while new_akq loads resolve,
                // eliminating back-to-back DFMA stalls on SM70.
                for (var k = 0u; k < n; k = k + 1u) {
                    if (k != p && k != q) {
                        let akp     = A_batch[base + idx2d(k, p, n)];
                        let akq     = A_batch[base + idx2d(k, q, n)];
                        // Load V simultaneously — issued during A-load latency:
                        let vkp     = V_batch[base + idx2d(k, p, n)];
                        let vkq     = V_batch[base + idx2d(k, q, n)];

                        // A rotation: independent of V loads, starts during V-load gap
                        let new_akp = c * akp - s * akq;
                        let new_akq = s * akp + c * akq;

                        // V rotation: independent of A results, compiler/HW can dual-issue
                        let new_vkp = c * vkp - s * vkq;
                        let new_vkq = s * vkp + c * vkq;

                        // Write back A (symmetric):
                        A_batch[base + idx2d(k, p, n)] = new_akp;
                        A_batch[base + idx2d(k, q, n)] = new_akq;
                        A_batch[base + idx2d(p, k, n)] = new_akp;
                        A_batch[base + idx2d(q, k, n)] = new_akq;

                        // Write back V:
                        V_batch[base + idx2d(k, p, n)] = new_vkp;
                        V_batch[base + idx2d(k, q, n)] = new_vkq;
                    }
                }
                // @ilp_region end

                // Update 2×2 diagonal block with pre-hoisted products:
                A_batch[base + idx2d(p, p, n)] = app_new;
                A_batch[base + idx2d(q, q, n)] = aqq_new;
                A_batch[base + idx2d(p, q, n)] = f64(0.0);
                A_batch[base + idx2d(q, p, n)] = f64(0.0);
            }
        }
    }
    
    // Step 3: Extract eigenvalues (diagonal of A) to eigenvalue buffer
    let eig_base = batch_idx * n;
    for (var i = 0u; i < n; i = i + 1u) {
        eigenvalues[eig_base + i] = A_batch[base + idx2d(i, i, n)];
    }
}
