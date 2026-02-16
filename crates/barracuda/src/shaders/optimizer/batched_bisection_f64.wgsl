// Batched Bisection Root-Finding (f64) — GPU-Parallel
//
// Solves many independent root-finding problems in parallel.
// Each thread handles one bisection problem.
//
// Use cases:
//   - BCS pairing: Find μ where Σ v²_k(μ) = N for each nucleus
//   - Multi-system parameter fitting
//   - Batch chemical equilibrium calculations
//
// Deep Debt Principles:
// - Pure WGSL (universal compute, hardware-agnostic)
// - Full f64 precision
// - Zero unsafe code
// - Self-contained
//
// Algorithm:
// - Each thread gets (lower, upper, params) for one problem
// - Runs bisection loop until convergence or max_iter
// - Writes root to output buffer
//
// BCS Pairing Formula:
//   Find μ such that: Σ_k v²_k(μ) = N
//   where v²_k = ½(1 - (ε_k - μ)/√((ε_k - μ)² + Δ²))

struct BisectionParams {
    batch_size: u32,        // Number of problems to solve
    max_iterations: u32,    // Max bisection iterations per problem
    n_levels: u32,          // Number of energy levels for BCS (params per problem)
    _pad: u32,
    tolerance: f64,         // Convergence tolerance |f(x)| < tol
}

// Input buffers
@group(0) @binding(0) var<storage, read> lower: array<f64>;     // Lower bounds [batch]
@group(0) @binding(1) var<storage, read> upper: array<f64>;     // Upper bounds [batch]
@group(0) @binding(2) var<storage, read> params: array<f64>;    // Problem parameters [batch, n_params]
@group(0) @binding(3) var<storage, read_write> roots: array<f64>; // Output roots [batch]
@group(0) @binding(4) var<storage, read_write> iterations: array<u32>; // Iterations used [batch]
@group(0) @binding(5) var<uniform> config: BisectionParams;

// Helper: absolute value for f64
fn abs_f64(x: f64) -> f64 {
    if (x < f64(0.0)) {
        return -x;
    }
    return x;
}

// Helper: sqrt for f64 (native builtin)
fn sqrt_f64(x: f64) -> f64 {
    return sqrt(x);
}

// BCS particle number function: Σ_k v²_k(μ) - N
// params layout: [ε_0, ε_1, ..., ε_{n-1}, Δ, N]
fn bcs_particle_number(mu: f64, problem_idx: u32) -> f64 {
    let n_levels = config.n_levels;
    let params_per_problem = n_levels + 2u; // eigenvalues + delta + target_N
    let base = problem_idx * params_per_problem;
    
    let delta = params[base + n_levels];
    let target_n = params[base + n_levels + 1u];
    
    var sum = f64(0.0);
    for (var k = 0u; k < n_levels; k = k + 1u) {
        let eps_k = params[base + k];
        let diff = eps_k - mu;
        let e_k = sqrt_f64(diff * diff + delta * delta);
        // v²_k = ½(1 - (ε_k - μ)/E_k)
        let v2_k = f64(0.5) * (f64(1.0) - diff / e_k);
        sum = sum + v2_k;
    }
    
    return sum - target_n;
}

// Generic bisection with custom function evaluation
// For now, hardcoded to BCS. Could be extended with function selector.
@compute @workgroup_size(64)
fn batched_bisection(
    @builtin(global_invocation_id) gid: vec3<u32>,
) {
    let problem_idx = gid.x;
    if (problem_idx >= config.batch_size) {
        return;
    }
    
    var lo = lower[problem_idx];
    var hi = upper[problem_idx];
    var iter_count = 0u;
    
    // Initial function values
    var f_lo = bcs_particle_number(lo, problem_idx);
    var f_hi = bcs_particle_number(hi, problem_idx);
    
    // Bisection loop
    for (var iter = 0u; iter < config.max_iterations; iter = iter + 1u) {
        let mid = f64(0.5) * (lo + hi);
        let f_mid = bcs_particle_number(mid, problem_idx);
        
        iter_count = iter + 1u;
        
        // Check convergence
        if (abs_f64(f_mid) < config.tolerance || (hi - lo) < config.tolerance) {
            roots[problem_idx] = mid;
            iterations[problem_idx] = iter_count;
            return;
        }
        
        // Narrow the interval
        if (f_lo * f_mid < f64(0.0)) {
            hi = mid;
            f_hi = f_mid;
        } else {
            lo = mid;
            f_lo = f_mid;
        }
    }
    
    // Max iterations reached - return best estimate
    roots[problem_idx] = f64(0.5) * (lo + hi);
    iterations[problem_idx] = config.max_iterations;
}

// Simple polynomial test function: f(x) = x² - target
// params layout: [target]
fn polynomial_test(x: f64, problem_idx: u32) -> f64 {
    let target = params[problem_idx];
    return x * x - target;
}

// Bisection for polynomial test (for validation)
@compute @workgroup_size(64)
fn batched_bisection_poly(
    @builtin(global_invocation_id) gid: vec3<u32>,
) {
    let problem_idx = gid.x;
    if (problem_idx >= config.batch_size) {
        return;
    }
    
    var lo = lower[problem_idx];
    var hi = upper[problem_idx];
    var iter_count = 0u;
    
    var f_lo = polynomial_test(lo, problem_idx);
    
    for (var iter = 0u; iter < config.max_iterations; iter = iter + 1u) {
        let mid = f64(0.5) * (lo + hi);
        let f_mid = polynomial_test(mid, problem_idx);
        
        iter_count = iter + 1u;
        
        if (abs_f64(f_mid) < config.tolerance || (hi - lo) < config.tolerance) {
            roots[problem_idx] = mid;
            iterations[problem_idx] = iter_count;
            return;
        }
        
        if (f_lo * f_mid < f64(0.0)) {
            hi = mid;
        } else {
            lo = mid;
            f_lo = f_mid;
        }
    }
    
    roots[problem_idx] = f64(0.5) * (lo + hi);
    iterations[problem_idx] = config.max_iterations;
}
