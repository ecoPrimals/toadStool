// Single-Dispatch Batched Symmetric Eigenvalue Decomposition (f64)
//
// CRITICAL EVOLUTION: Eliminates the per-rotation queue.submit() bottleneck.
// 
// Previous implementation: 4 dispatches × n(n-1)/2 rotations × max_sweeps = ~8000 submits
// This implementation: 1 dispatch total
//
// Architecture:
// - One WORKGROUP processes ONE matrix from the batch
// - Matrix A and eigenvectors V stored in workgroup shared memory
// - ALL Jacobi sweeps run in a loop INSIDE the shader
// - Only ONE queue.submit() needed for the entire batch
//
// Memory: For n=12, each matrix needs 12×12×8 = 1152 bytes
//         A + V = 2304 bytes, well within 16KB shared memory limit
//         Can support up to n≈40 with 16KB, n≈64 with 48KB
//
// Use case: hotSpring HFB (n=12, batch=40) completes in 1 dispatch vs 7920 dispatches
//
// Reference: hotSpring handoff Feb 12, 2026 - TIER 1.1

// Maximum matrix dimension supported (limited by shared memory)
// 32×32 matrices = 32×32×8×2 = 16KB for A+V
const MAX_N: u32 = 32u;

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

// Workgroup shared memory for one matrix
// Using MAX_N to allow static allocation
var<workgroup> A_shared: array<f64, 1024>;  // 32×32 = 1024 elements
var<workgroup> V_shared: array<f64, 1024>;  // 32×32 = 1024 elements

// Helper: 2D to 1D index for shared memory
fn idx2d(row: u32, col: u32, n: u32) -> u32 {
    return row * n + col;
}

// Helper: Global memory offset for batch
fn batch_offset(batch_idx: u32, n: u32) -> u32 {
    return batch_idx * n * n;
}

// Single-dispatch eigensolve: ONE workgroup processes ONE matrix
// Dispatch with (batch_size, 1, 1) workgroups
@compute @workgroup_size(1, 1, 1)
fn batched_eigh_single_dispatch(
    @builtin(workgroup_id) wg_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let batch_idx = wg_id.x;
    let n = params.n;
    
    if (batch_idx >= params.batch_size || n > MAX_N) {
        return;
    }
    
    let base = batch_offset(batch_idx, n);
    let tol = f64(params.tolerance);
    
    // Step 1: Load matrix A from global to shared memory
    for (var i = 0u; i < n; i = i + 1u) {
        for (var j = 0u; j < n; j = j + 1u) {
            A_shared[idx2d(i, j, n)] = A_batch[base + idx2d(i, j, n)];
        }
    }
    
    // Step 2: Initialize V = Identity in shared memory
    for (var i = 0u; i < n; i = i + 1u) {
        for (var j = 0u; j < n; j = j + 1u) {
            if (i == j) {
                V_shared[idx2d(i, j, n)] = f64(1.0);
            } else {
                V_shared[idx2d(i, j, n)] = f64(0.0);
            }
        }
    }
    
    // Step 3: Jacobi sweeps - ALL iterations run here
    for (var sweep = 0u; sweep < params.max_sweeps; sweep = sweep + 1u) {
        // Check convergence: max off-diagonal element
        var max_off = f64(0.0);
        for (var i = 0u; i < n; i = i + 1u) {
            for (var j = i + 1u; j < n; j = j + 1u) {
                let off = abs(A_shared[idx2d(i, j, n)]);
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
                let apq = A_shared[idx2d(p, q, n)];
                
                // Skip if already zero
                if (abs(apq) < 1e-14) {
                    continue;
                }
                
                let app = A_shared[idx2d(p, p, n)];
                let aqq = A_shared[idx2d(q, q, n)];
                
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
                
                // Apply rotation to A (rows and columns p, q)
                for (var k = 0u; k < n; k = k + 1u) {
                    if (k != p && k != q) {
                        let akp = A_shared[idx2d(k, p, n)];
                        let akq = A_shared[idx2d(k, q, n)];
                        
                        let new_akp = c * akp - s * akq;
                        let new_akq = s * akp + c * akq;
                        
                        A_shared[idx2d(k, p, n)] = new_akp;
                        A_shared[idx2d(k, q, n)] = new_akq;
                        A_shared[idx2d(p, k, n)] = new_akp;  // Symmetric
                        A_shared[idx2d(q, k, n)] = new_akq;
                    }
                }
                
                // Update 2×2 block
                let app_new = c * c * app - 2.0 * c * s * apq + s * s * aqq;
                let aqq_new = s * s * app + 2.0 * c * s * apq + c * c * aqq;
                
                A_shared[idx2d(p, p, n)] = app_new;
                A_shared[idx2d(q, q, n)] = aqq_new;
                A_shared[idx2d(p, q, n)] = f64(0.0);
                A_shared[idx2d(q, p, n)] = f64(0.0);
                
                // Apply rotation to V (columns p, q)
                for (var k = 0u; k < n; k = k + 1u) {
                    let vkp = V_shared[idx2d(k, p, n)];
                    let vkq = V_shared[idx2d(k, q, n)];
                    
                    V_shared[idx2d(k, p, n)] = c * vkp - s * vkq;
                    V_shared[idx2d(k, q, n)] = s * vkp + c * vkq;
                }
            }
        }
    }
    
    // Step 4: Extract eigenvalues (diagonal of A) to global memory
    let eig_base = batch_idx * n;
    for (var i = 0u; i < n; i = i + 1u) {
        eigenvalues[eig_base + i] = A_shared[idx2d(i, i, n)];
    }
    
    // Step 5: Write eigenvectors back to global memory
    for (var i = 0u; i < n; i = i + 1u) {
        for (var j = 0u; j < n; j = j + 1u) {
            V_batch[base + idx2d(i, j, n)] = V_shared[idx2d(i, j, n)];
        }
    }
}
