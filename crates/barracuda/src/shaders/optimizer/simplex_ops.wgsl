// Nelder-Mead Simplex Operations - Shader-First Implementation
//
// Parallel primitives for the Nelder-Mead simplex algorithm:
// - Centroid computation (parallel reduction)
// - Reflect/Expand/Contract (parallel vector ops)
// - Multi-start parallel evaluation
//
// The outer decision loop (reflect vs expand vs contract) remains on CPU,
// but ALL compute-heavy operations are GPU shaders.
//
// This is SHADER-FIRST optimization:
// - Same math as CPU Nelder-Mead
// - Parallel execution of vector operations
// - Foundation for massive multi-start parallelism

struct SimplexParams {
    n: u32,           // Dimension
    n_points: u32,    // Number of simplex points (n+1)
    alpha: f32,       // Reflection coefficient (1.0)
    gamma: f32,       // Expansion coefficient (2.0)
}

@group(0) @binding(0) var<uniform> params: SimplexParams;
@group(0) @binding(1) var<storage, read> simplex: array<f32>;     // [n_points × n]
@group(0) @binding(2) var<storage, read> f_vals: array<f32>;      // [n_points]
@group(0) @binding(3) var<storage, read_write> centroid: array<f32>;  // [n]
@group(0) @binding(4) var<storage, read_write> output: array<f32>;    // [n]

// Compute centroid of simplex excluding worst point
// centroid[j] = (1/n) Σ_{i≠worst} simplex[i,j]
var<workgroup> shared_sum: array<f32, 256>;

@compute @workgroup_size(256, 1, 1)
fn compute_centroid(@builtin(local_invocation_id) local_id: vec3<u32>,
                    @builtin(global_invocation_id) global_id: vec3<u32>) {
    let j = global_id.x;  // Dimension index
    let n = params.n;
    let n_points = params.n_points;
    
    if (j >= n) {
        return;
    }
    
    // Find worst point index (highest f value)
    // For simplicity, assume f_vals is already sorted or worst_idx passed
    // Here we compute it
    var worst_idx = 0u;
    var worst_val = f_vals[0];
    for (var i = 1u; i < n_points; i = i + 1u) {
        if (f_vals[i] > worst_val) {
            worst_val = f_vals[i];
            worst_idx = i;
        }
    }
    
    // Sum over all points except worst
    var sum: f32 = 0.0;
    var count: u32 = 0u;
    for (var i = 0u; i < n_points; i = i + 1u) {
        if (i != worst_idx) {
            sum = sum + simplex[i * n + j];
            count = count + 1u;
        }
    }
    
    centroid[j] = sum / f32(count);
}

// Reflect point through centroid: x_r = centroid + α(centroid - x_worst)
@compute @workgroup_size(256, 1, 1)
fn reflect(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let j = global_id.x;
    let n = params.n;
    
    if (j >= n) {
        return;
    }
    
    // Find worst point (could be passed as parameter)
    var worst_idx = 0u;
    var worst_val = f_vals[0];
    for (var i = 1u; i < params.n_points; i = i + 1u) {
        if (f_vals[i] > worst_val) {
            worst_val = f_vals[i];
            worst_idx = i;
        }
    }
    
    let x_worst = simplex[worst_idx * n + j];
    let c = centroid[j];
    
    // x_reflect = c + α(c - x_worst)
    output[j] = c + params.alpha * (c - x_worst);
}

// Expand: x_e = centroid + γ(x_reflect - centroid)
// Requires x_reflect to be in 'output' buffer already
@compute @workgroup_size(256, 1, 1)
fn expand(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let j = global_id.x;
    let n = params.n;
    
    if (j >= n) {
        return;
    }
    
    let c = centroid[j];
    let x_r = output[j];  // x_reflect from previous step
    
    // x_expand = c + γ(x_r - c)
    output[j] = c + params.gamma * (x_r - c);
}

// Contract: x_c = centroid + ρ(x_worst - centroid)  [inside]
//       or: x_c = centroid + ρ(x_reflect - centroid) [outside]
struct ContractParams {
    n: u32,
    rho: f32,         // Contraction coefficient (0.5)
    inside: u32,      // 1 = inside (toward worst), 0 = outside (toward reflect)
    _pad: u32,
}

@group(0) @binding(0) var<uniform> contract_params: ContractParams;
@group(0) @binding(1) var<storage, read> simplex_c: array<f32>;
@group(0) @binding(2) var<storage, read> f_vals_c: array<f32>;
@group(0) @binding(3) var<storage, read> centroid_c: array<f32>;
@group(0) @binding(4) var<storage, read> x_reflect: array<f32>;
@group(0) @binding(5) var<storage, read_write> output_c: array<f32>;

@compute @workgroup_size(256, 1, 1)
fn contract(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let j = global_id.x;
    let n = contract_params.n;
    
    if (j >= n) {
        return;
    }
    
    let c = centroid_c[j];
    
    if (contract_params.inside == 1u) {
        // Inside contraction: toward worst point
        // First find worst (would be better to pass as param)
        var worst_idx = 0u;
        var worst_val = f_vals_c[0];
        for (var i = 1u; i < n + 1u; i = i + 1u) {
            if (f_vals_c[i] > worst_val) {
                worst_val = f_vals_c[i];
                worst_idx = i;
            }
        }
        let x_worst = simplex_c[worst_idx * n + j];
        output_c[j] = c + contract_params.rho * (x_worst - c);
    } else {
        // Outside contraction: toward reflected point
        output_c[j] = c + contract_params.rho * (x_reflect[j] - c);
    }
}

// Shrink: Move all points except best toward best
// x_i = x_best + σ(x_i - x_best)
struct ShrinkParams {
    n: u32,
    n_points: u32,
    sigma: f32,       // Shrinkage coefficient (0.5)
    best_idx: u32,
}

@group(0) @binding(0) var<uniform> shrink_params: ShrinkParams;
@group(0) @binding(1) var<storage, read_write> simplex_s: array<f32>;  // Modified in place

@compute @workgroup_size(16, 16, 1)
fn shrink(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.y;  // Point index
    let j = global_id.x;  // Dimension index
    let n = shrink_params.n;
    let n_points = shrink_params.n_points;
    
    if (i >= n_points || j >= n || i == shrink_params.best_idx) {
        return;
    }
    
    let x_best_j = simplex_s[shrink_params.best_idx * n + j];
    let x_i_j = simplex_s[i * n + j];
    
    simplex_s[i * n + j] = x_best_j + shrink_params.sigma * (x_i_j - x_best_j);
}

// Project onto bounds: clamp each dimension
struct BoundsParams {
    n: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<uniform> bounds_params: BoundsParams;
@group(0) @binding(1) var<storage, read> bounds_lo: array<f32>;
@group(0) @binding(2) var<storage, read> bounds_hi: array<f32>;
@group(0) @binding(3) var<storage, read_write> point: array<f32>;

@compute @workgroup_size(256, 1, 1)
fn project_bounds(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let j = global_id.x;
    
    if (j >= bounds_params.n) {
        return;
    }
    
    point[j] = clamp(point[j], bounds_lo[j], bounds_hi[j]);
}

// Sort simplex by function values (parallel bitonic sort)
// This enables efficient parallel finding of best/worst
struct SortParams {
    n_points: u32,
    stage: u32,
    step: u32,
    _pad: u32,
}

@group(0) @binding(0) var<uniform> sort_params: SortParams;
@group(0) @binding(1) var<storage, read_write> indices: array<u32>;
@group(0) @binding(2) var<storage, read> f_vals_sort: array<f32>;

@compute @workgroup_size(256, 1, 1)
fn bitonic_sort_step(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let n = sort_params.n_points;
    
    if (i >= n / 2u) {
        return;
    }
    
    let stage = sort_params.stage;
    let step = sort_params.step;
    
    // Bitonic merge network
    let pair_distance = 1u << step;
    let block_size = 1u << (stage + 1u);
    
    let left_idx = (i / pair_distance) * (pair_distance * 2u) + (i % pair_distance);
    let right_idx = left_idx + pair_distance;
    
    if (right_idx >= n) {
        return;
    }
    
    let left_val = f_vals_sort[indices[left_idx]];
    let right_val = f_vals_sort[indices[right_idx]];
    
    // Determine sort direction (ascending in first half of stage)
    let ascending = ((i / (block_size / 2u)) % 2u) == 0u;
    
    let should_swap = (ascending && left_val > right_val) || (!ascending && left_val < right_val);
    
    if (should_swap) {
        let temp = indices[left_idx];
        indices[left_idx] = indices[right_idx];
        indices[right_idx] = temp;
    }
}
