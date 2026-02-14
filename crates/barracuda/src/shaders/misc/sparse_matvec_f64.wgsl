// Sparse Matrix-Vector Product (CSR format) - f64 Precision
// y = A * x where A is in Compressed Sparse Row format
//
// Full f64 precision via WGSL native f64 support (SPIR-V/Vulkan)
// FP64 performance: 1:2-3 (not 1:32 like CUDA consumer cards)
//
// Uses atomic-free design: one thread per row, full precision

struct Params {
    num_rows: u32,
}

@group(0) @binding(0) var<storage, read> values: array<f64>;
@group(0) @binding(1) var<storage, read> col_indices: array<u32>;
@group(0) @binding(2) var<storage, read> row_ptrs: array<u32>;
@group(0) @binding(3) var<storage, read> vector: array<f64>;
@group(0) @binding(4) var<storage, read_write> output: array<f64>;
@group(0) @binding(5) var<uniform> params: Params;

@compute @workgroup_size(256)
fn spmv_f64(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let row = global_id.x;
    if (row >= params.num_rows) {
        return;
    }

    let start = row_ptrs[row];
    let end = row_ptrs[row + 1u];

    var sum: f64 = 0.0;
    for (var j = start; j < end; j = j + 1u) {
        let col = col_indices[j];
        let val = values[j];
        sum = sum + val * vector[col];
    }
    output[row] = sum;
}

// Vector operations for iterative solvers

// y = alpha * x + y (axpy)
@group(0) @binding(0) var<storage, read> x: array<f64>;
@group(0) @binding(1) var<storage, read_write> y_axpy: array<f64>;
@group(0) @binding(2) var<uniform> axpy_params: AxpyParams;

struct AxpyParams {
    n: u32,
    alpha: f64,
}

@compute @workgroup_size(256)
fn axpy_f64(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= axpy_params.n) {
        return;
    }
    y_axpy[idx] = axpy_params.alpha * x[idx] + y_axpy[idx];
}

// Dot product reduction (per-workgroup partial sum)
@group(0) @binding(0) var<storage, read> dot_a: array<f64>;
@group(0) @binding(1) var<storage, read> dot_b: array<f64>;
@group(0) @binding(2) var<storage, read_write> partial_sums: array<f64>;
@group(0) @binding(3) var<uniform> dot_params: DotParams;

struct DotParams {
    n: u32,
}

var<workgroup> shared_sum: array<f64, 256>;

@compute @workgroup_size(256)
fn dot_f64(@builtin(local_invocation_id) local_id: vec3<u32>,
           @builtin(global_invocation_id) global_id: vec3<u32>,
           @builtin(workgroup_id) wg_id: vec3<u32>) {
    let tid = local_id.x;
    let gid = global_id.x;
    let n = dot_params.n;

    // Each thread sums its elements
    var sum: f64 = 0.0;
    var i = gid;
    while (i < n) {
        sum = sum + dot_a[i] * dot_b[i];
        i = i + 256u * 256u;  // Stride by total threads
    }

    shared_sum[tid] = sum;
    workgroupBarrier();

    // Tree reduction in shared memory
    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (tid < stride) {
            shared_sum[tid] = shared_sum[tid] + shared_sum[tid + stride];
        }
        workgroupBarrier();
    }

    if (tid == 0u) {
        partial_sums[wg_id.x] = shared_sum[0];
    }
}

// Scale vector: y = alpha * x
@group(0) @binding(0) var<storage, read> scale_x: array<f64>;
@group(0) @binding(1) var<storage, read_write> scale_y: array<f64>;
@group(0) @binding(2) var<uniform> scale_params: ScaleParams;

struct ScaleParams {
    n: u32,
    alpha: f64,
}

@compute @workgroup_size(256)
fn scale_f64(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= scale_params.n) {
        return;
    }
    scale_y[idx] = scale_params.alpha * scale_x[idx];
}

// Copy vector: y = x
@group(0) @binding(0) var<storage, read> copy_src: array<f64>;
@group(0) @binding(1) var<storage, read_write> copy_dst: array<f64>;
@group(0) @binding(2) var<uniform> copy_params: CopyParams;

struct CopyParams {
    n: u32,
}

@compute @workgroup_size(256)
fn copy_f64(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= copy_params.n) {
        return;
    }
    copy_dst[idx] = copy_src[idx];
}

// Apply diagonal preconditioner: z[i] = r[i] / diag[i]
@group(0) @binding(0) var<storage, read> r_precond: array<f64>;
@group(0) @binding(1) var<storage, read> diag_precond: array<f64>;
@group(0) @binding(2) var<storage, read_write> z_precond: array<f64>;
@group(0) @binding(3) var<uniform> precond_params: PrecondParams;

struct PrecondParams {
    n: u32,
}

@compute @workgroup_size(256)
fn precond_f64(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= precond_params.n) {
        return;
    }
    let d = diag_precond[idx];
    // Safe division - avoid very small divisors
    if (abs(d) > 1e-12) {
        z_precond[idx] = r_precond[idx] / d;
    } else {
        z_precond[idx] = r_precond[idx];
    }
}

// Linear combination: z = alpha * x + beta * y
@group(0) @binding(0) var<storage, read> lc_x: array<f64>;
@group(0) @binding(1) var<storage, read> lc_y: array<f64>;
@group(0) @binding(2) var<storage, read_write> lc_z: array<f64>;
@group(0) @binding(3) var<uniform> lc_params: LCParams;

struct LCParams {
    n: u32,
    alpha: f64,
    beta: f64,
}

@compute @workgroup_size(256)
fn linear_comb_f64(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= lc_params.n) {
        return;
    }
    lc_z[idx] = lc_params.alpha * lc_x[idx] + lc_params.beta * lc_y[idx];
}
