// Cyclic Reduction (Odd-Even Reduction) for Tridiagonal Systems
//
// Solves tridiagonal system: a[i]*x[i-1] + b[i]*x[i] + c[i]*x[i+1] = d[i]
//
// PARALLEL ALGORITHM (O(log n) steps vs O(n) for Thomas):
// 1. Reduction phase: Eliminate odd-indexed equations
// 2. After log2(n) reductions, solve 1-element system
// 3. Substitution phase: Back-substitute to recover all x[i]
//
// This is the SHADER-FIRST approach to tridiagonal solve:
// - Same math as Thomas algorithm
// - Parallel execution on GPU
// - When future sequential hardware available, same code runs there
//
// Applications:
// - Crank-Nicolson PDE (TTM, heat equation)
// - Cubic spline interpolation
// - Implicit finite difference
//
// Reference: Hockney & Jesshope, "Parallel Computers" (1988)

struct Params {
    n: u32,           // System size (must be power of 2 for simplicity)
    step: u32,        // Current reduction step (0 = first reduction)
    phase: u32,       // 0 = reduction, 1 = substitution
    _pad: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read_write> a: array<f32>;  // Sub-diagonal
@group(0) @binding(2) var<storage, read_write> b: array<f32>;  // Main diagonal
@group(0) @binding(3) var<storage, read_write> c: array<f32>;  // Super-diagonal
@group(0) @binding(4) var<storage, read_write> d: array<f32>;  // RHS / solution

// Reduction step: Eliminate odd-indexed unknowns
// After this, even-indexed equations form a smaller tridiagonal system
@compute @workgroup_size(256, 1, 1)
fn reduction(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let stride = 1u << (params.step + 1u);  // 2^(step+1)
    let half_stride = stride >> 1u;          // 2^step
    
    let i = global_id.x * stride + half_stride;  // Odd indices at this level
    
    if (i >= params.n) {
        return;
    }
    
    let i_prev = i - half_stride;  // Previous even index
    let i_next = i + half_stride;  // Next even index
    
    // Skip boundary cases
    if (i_prev >= params.n || i_next >= params.n) {
        return;
    }
    
    // Compute elimination factors
    let alpha = -a[i] / b[i_prev];
    let gamma = -c[i] / b[i_next];
    
    // Update coefficients for the reduced system
    // New equation for index i combines neighbors
    b[i] = b[i] + alpha * c[i_prev] + gamma * a[i_next];
    d[i] = d[i] + alpha * d[i_prev] + gamma * d[i_next];
    
    // Update connections to farther neighbors
    a[i] = alpha * a[i_prev];
    c[i] = gamma * c[i_next];
}

// Substitution step: Back-substitute to recover solution
@compute @workgroup_size(256, 1, 1)
fn substitution(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let stride = 1u << (params.step + 1u);
    let half_stride = stride >> 1u;
    
    let i = global_id.x * stride + half_stride;
    
    if (i >= params.n) {
        return;
    }
    
    let i_prev = i - half_stride;
    let i_next = i + half_stride;
    
    // x[i] = (d[i] - a[i]*x[i_prev] - c[i]*x[i_next]) / b[i]
    var x_prev: f32 = 0.0;
    var x_next: f32 = 0.0;
    
    if (i_prev < params.n) {
        x_prev = d[i_prev];  // d now holds solution
    }
    if (i_next < params.n) {
        x_next = d[i_next];
    }
    
    d[i] = (d[i] - a[i] * x_prev - c[i] * x_next) / b[i];
}

// Single-pass for small systems (n <= 256)
// Uses shared memory for efficiency
var<workgroup> shared_a: array<f32, 256>;
var<workgroup> shared_b: array<f32, 256>;
var<workgroup> shared_c: array<f32, 256>;
var<workgroup> shared_d: array<f32, 256>;

@compute @workgroup_size(256, 1, 1)
fn solve_small(@builtin(local_invocation_id) local_id: vec3<u32>,
               @builtin(workgroup_id) wg_id: vec3<u32>) {
    let tid = local_id.x;
    let n = params.n;
    
    // Load into shared memory
    if (tid < n) {
        shared_a[tid] = a[tid];
        shared_b[tid] = b[tid];
        shared_c[tid] = c[tid];
        shared_d[tid] = d[tid];
    }
    workgroupBarrier();
    
    // Reduction phase
    var stride = 1u;
    for (var step = 0u; step < 8u; step = step + 1u) {  // log2(256) = 8
        if (stride >= n) { break; }
        
        let half_stride = stride;
        stride = stride << 1u;
        
        let i = tid * stride + half_stride;
        if (i < n) {
            let i_prev = i - half_stride;
            let i_next = i + half_stride;
            
            if (i_prev < n && i_next < n && shared_b[i_prev] != 0.0 && shared_b[i_next] != 0.0) {
                let alpha = -shared_a[i] / shared_b[i_prev];
                let gamma = -shared_c[i] / shared_b[i_next];
                
                shared_b[i] = shared_b[i] + alpha * shared_c[i_prev] + gamma * shared_a[i_next];
                shared_d[i] = shared_d[i] + alpha * shared_d[i_prev] + gamma * shared_d[i_next];
                shared_a[i] = alpha * shared_a[i_prev];
                shared_c[i] = gamma * shared_c[i_next];
            }
        }
        workgroupBarrier();
    }
    
    // Solve the 1-element system at the center
    if (tid == n / 2u - 1u || (n == 1u && tid == 0u)) {
        if (shared_b[tid] != 0.0) {
            shared_d[tid] = shared_d[tid] / shared_b[tid];
        }
    }
    workgroupBarrier();
    
    // Substitution phase (reverse order)
    stride = n;
    for (var step = 0u; step < 8u; step = step + 1u) {
        if (stride <= 1u) { break; }
        
        stride = stride >> 1u;
        let half_stride = stride >> 1u;
        if (half_stride == 0u) { break; }
        
        let i = tid * stride + half_stride;
        if (i < n && i > 0u) {
            let i_prev = i - half_stride;
            let i_next = i + half_stride;
            
            var x_prev: f32 = 0.0;
            var x_next: f32 = 0.0;
            if (i_prev < n) { x_prev = shared_d[i_prev]; }
            if (i_next < n) { x_next = shared_d[i_next]; }
            
            if (shared_b[i] != 0.0) {
                shared_d[i] = (shared_d[i] - shared_a[i] * x_prev - shared_c[i] * x_next) / shared_b[i];
            }
        }
        workgroupBarrier();
    }
    
    // Write back solution
    if (tid < n) {
        d[tid] = shared_d[tid];
    }
}
