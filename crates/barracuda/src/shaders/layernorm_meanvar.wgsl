// LayerNorm Mean+Variance Computation (Single Pass)
//
// **2-DISPATCH LAYERNORM - DISPATCH 1: COMPUTE MEAN AND VARIANCE TOGETHER**
//
// Original 3-pass approach:
//   Pass 1: Compute mean
//   Pass 2: Compute variance (using mean)
//   Pass 3: Normalize
//
// Optimized 2-dispatch approach:
//   Dispatch 1: Compute BOTH mean and variance in single pass (THIS SHADER)
//   Dispatch 2: Normalize (separate shader)
//
// Algorithm (Welford's two-pass):
//   1. First scan: Compute mean using Welford's algorithm
//   2. Second scan: Compute variance from mean
//
// Note: This is still memory-bound but eliminates 1/3 launch overhead

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> stats: array<f32>;  // [mean, variance]

struct Params {
    size: u32,
    epsilon: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

// Shared memory for reduction
var<workgroup> shared_sum: array<f32, 256>;
var<workgroup> shared_sq_sum: array<f32, 256>;

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let tid = local_id.x;
    let total_threads = 256u;
    
    // ═══════════════════════════════════════════════════════════
    // PHASE 1: Compute local sum for mean
    // ═══════════════════════════════════════════════════════════
    
    var local_sum: f32 = 0.0;
    
    // Grid-stride loop
    for (var i = global_id.x; i < params.size; i = i + total_threads) {
        local_sum = local_sum + input[i];
    }
    
    shared_sum[tid] = local_sum;
    workgroupBarrier();
    
    // Reduce to compute total sum
    for (var stride = 128u; stride > 0u; stride = stride / 2u) {
        if (tid < stride) {
            shared_sum[tid] = shared_sum[tid] + shared_sum[tid + stride];
        }
        workgroupBarrier();
    }
    
    // Thread 0 computes and stores mean
    var mean: f32;
    if (tid == 0u) {
        mean = shared_sum[0] / f32(params.size);
        stats[0] = mean;
    }
    workgroupBarrier();
    
    // Broadcast mean to all threads
    mean = stats[0];
    
    // ═══════════════════════════════════════════════════════════
    // PHASE 2: Compute local squared differences for variance
    // ═══════════════════════════════════════════════════════════
    
    var local_sq_sum: f32 = 0.0;
    
    // Grid-stride loop
    for (var i = global_id.x; i < params.size; i = i + total_threads) {
        let diff = input[i] - mean;
        local_sq_sum = local_sq_sum + diff * diff;
    }
    
    shared_sq_sum[tid] = local_sq_sum;
    workgroupBarrier();
    
    // Reduce to compute total squared sum
    for (var stride = 128u; stride > 0u; stride = stride / 2u) {
        if (tid < stride) {
            shared_sq_sum[tid] = shared_sq_sum[tid] + shared_sq_sum[tid + stride];
        }
        workgroupBarrier();
    }
    
    // Thread 0 computes and stores variance
    if (tid == 0u) {
        let variance = shared_sq_sum[0] / f32(params.size);
        stats[1] = variance;
    }
}
