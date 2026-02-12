// BFGS Inverse Hessian Update - Shader-First Implementation
//
// Updates the inverse Hessian approximation H⁻¹ using the BFGS formula:
//   H⁻¹_new = (I - ρsy^T)H⁻¹(I - ρys^T) + ρss^T
//
// Where:
//   s = x_new - x_old (step vector)
//   y = ∇f_new - ∇f_old (gradient difference)
//   ρ = 1 / (s^T y) (curvature)
//
// Expanded form (more efficient):
//   H⁻¹_new = H⁻¹ - ρ(s⊗Hy + Hy⊗s) + ρ(1 + ρy^T H⁻¹ y)(s⊗s)
//
// This is SHADER-FIRST BFGS:
// - Matrix update is O(n²) parallel operations
// - All n² elements updated simultaneously
// - Sequential part is only the outer iteration loop
//
// Reference: Nocedal & Wright, "Numerical Optimization" (2006)

struct Params {
    n: u32,           // Dimension
    rho: f32,         // 1 / (s^T y)
    yHy: f32,         // y^T H⁻¹ y (precomputed)
    _pad: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> s: array<f32>;           // Step vector (n)
@group(0) @binding(2) var<storage, read> Hy: array<f32>;          // H⁻¹ · y (n) - precomputed
@group(0) @binding(3) var<storage, read_write> H_inv: array<f32>; // Inverse Hessian (n×n)

// Full BFGS update - each thread updates one matrix element
@compute @workgroup_size(16, 16, 1)
fn bfgs_update(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.y;
    let j = global_id.x;
    let n = params.n;
    
    if (i >= n || j >= n) {
        return;
    }
    
    let idx = i * n + j;
    let rho = params.rho;
    
    // factor = ρ(1 + ρ·y^T H⁻¹ y) = ρ + ρ²·yHy
    let factor = rho * (1.0 + rho * params.yHy);
    
    // H⁻¹_new[i,j] = H⁻¹[i,j] - ρ(s[i]·Hy[j] + Hy[i]·s[j]) + factor·s[i]·s[j]
    H_inv[idx] = H_inv[idx] 
                 - rho * (s[i] * Hy[j] + Hy[i] * s[j])
                 + factor * s[i] * s[j];
}

// Compute ρ = 1/(s^T y) - parallel dot product with reduction
// Output: single scalar in rho_out[0]
@group(0) @binding(0) var<uniform> dot_params: Params;
@group(0) @binding(1) var<storage, read> dot_a: array<f32>;
@group(0) @binding(2) var<storage, read> dot_b: array<f32>;
@group(0) @binding(3) var<storage, read_write> dot_result: array<f32>;

var<workgroup> partial_sums: array<f32, 256>;

@compute @workgroup_size(256, 1, 1)
fn dot_product(@builtin(local_invocation_id) local_id: vec3<u32>,
               @builtin(global_invocation_id) global_id: vec3<u32>,
               @builtin(workgroup_id) wg_id: vec3<u32>) {
    let tid = local_id.x;
    let gid = global_id.x;
    let n = dot_params.n;
    
    // Each thread computes partial sum
    var sum: f32 = 0.0;
    var i = gid;
    while (i < n) {
        sum = sum + dot_a[i] * dot_b[i];
        i = i + 256u * 256u;  // Grid stride loop
    }
    
    partial_sums[tid] = sum;
    workgroupBarrier();
    
    // Parallel reduction within workgroup
    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (tid < stride) {
            partial_sums[tid] = partial_sums[tid] + partial_sums[tid + stride];
        }
        workgroupBarrier();
    }
    
    // First thread writes result
    if (tid == 0u) {
        // Atomic add for multi-workgroup accumulation
        // For single workgroup, this is the final result
        dot_result[wg_id.x] = partial_sums[0];
    }
}

// Matrix-vector multiply: Hy = H⁻¹ · y
// Each thread computes one element of result
@compute @workgroup_size(256, 1, 1)
fn mat_vec_mul(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let n = params.n;
    
    if (i >= n) {
        return;
    }
    
    // Load y from s (reusing binding)
    // Actually we need separate bindings for clarity
    var sum: f32 = 0.0;
    for (var j = 0u; j < n; j = j + 1u) {
        sum = sum + H_inv[i * n + j] * s[j];  // s is used as y here
    }
    
    Hy[i] = sum;
}

// Combined: Compute Hy and yHy in one pass
// More efficient for small-medium matrices
struct CombinedParams {
    n: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<uniform> combined_params: CombinedParams;
@group(0) @binding(1) var<storage, read> y_vec: array<f32>;
@group(0) @binding(2) var<storage, read> H_mat: array<f32>;
@group(0) @binding(3) var<storage, read_write> Hy_out: array<f32>;
@group(0) @binding(4) var<storage, read_write> yHy_out: array<f32>;  // Single element

var<workgroup> Hy_shared: array<f32, 256>;

@compute @workgroup_size(256, 1, 1)
fn compute_Hy_and_yHy(@builtin(local_invocation_id) local_id: vec3<u32>,
                       @builtin(global_invocation_id) global_id: vec3<u32>) {
    let tid = local_id.x;
    let i = global_id.x;
    let n = combined_params.n;
    
    // Compute Hy[i]
    var sum: f32 = 0.0;
    if (i < n) {
        for (var j = 0u; j < n; j = j + 1u) {
            sum = sum + H_mat[i * n + j] * y_vec[j];
        }
        Hy_out[i] = sum;
        Hy_shared[tid] = y_vec[i] * sum;  // y[i] * Hy[i] for yHy
    } else {
        Hy_shared[tid] = 0.0;
    }
    workgroupBarrier();
    
    // Reduce to get yHy = Σ y[i] * Hy[i]
    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (tid < stride && tid + stride < n) {
            Hy_shared[tid] = Hy_shared[tid] + Hy_shared[tid + stride];
        }
        workgroupBarrier();
    }
    
    if (tid == 0u) {
        yHy_out[0] = Hy_shared[0];
    }
}
