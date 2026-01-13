// Softmax: Numerically stable softmax activation
// CUDA equivalent: cudnn::Softmax
// Formula: softmax(x_i) = exp(x_i - max(x)) / sum(exp(x_j - max(x)))
// Use cases: Classification output, attention weights

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<storage, read_write> max_val: array<f32>;  // For stability
@group(0) @binding(3) var<storage, read_write> sum_val: array<f32>;

struct Params {
    size: u32,
}
@group(0) @binding(4) var<uniform> params: Params;

// Pass 1: Find max value
@compute @workgroup_size(256)
fn find_max(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let tid = local_id.x;
    let gid = global_id.x;
    
    var<workgroup> shared_data: array<f32, 256>;
    
    var value: f32 = -3.402823e+38;  // -FLT_MAX
    if (gid < params.size) {
        value = input[gid];
    }
    shared_data[tid] = value;
    workgroupBarrier();
    
    // Tree reduction
    for (var stride = 128u; stride > 0u; stride = stride / 2u) {
        if (tid < stride) {
            shared_data[tid] = max(shared_data[tid], shared_data[tid + stride]);
        }
        workgroupBarrier();
    }
    
    if (tid == 0u) {
        max_val[workgroup_id.x] = shared_data[0];
    }
}

// Pass 2: Compute exp(x - max) and sum
@compute @workgroup_size(256)
fn compute_exp_sum(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let tid = local_id.x;
    let gid = global_id.x;
    
    var<workgroup> shared_data: array<f32, 256>;
    
    // Assume max_val[0] contains the global max (computed on CPU or via multi-pass)
    let global_max = max_val[0];
    
    var value: f32 = 0.0;
    if (gid < params.size) {
        let exp_val = exp(input[gid] - global_max);
        output[gid] = exp_val;  // Store exp values
        value = exp_val;
    }
    shared_data[tid] = value;
    workgroupBarrier();
    
    // Tree reduction for sum
    for (var stride = 128u; stride > 0u; stride = stride / 2u) {
        if (tid < stride) {
            shared_data[tid] = shared_data[tid] + shared_data[tid + stride];
        }
        workgroupBarrier();
    }
    
    if (tid == 0u) {
        sum_val[workgroup_id.x] = shared_data[0];
    }
}

// Pass 3: Divide by sum
@compute @workgroup_size(256)
fn normalize(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let gid = global_id.x;
    
    if (gid < params.size) {
        // Assume sum_val[0] contains the global sum
        output[gid] = output[gid] / sum_val[0];
    }
}
