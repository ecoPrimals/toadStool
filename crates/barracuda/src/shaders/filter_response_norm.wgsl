// Filter Response Normalization (FRN) - Normalization without batch dependency
// Normalizes activations per filter, not per batch
// Enables single-sample inference
//
// Algorithm:
// 1. Compute squared norm for each filter: nu = sqrt(sum(x^2) / spatial_size)
// 2. Normalize: x_norm = x / (nu + epsilon)
// 3. Scale and shift: output = gamma * x_norm + beta

struct Params {
    batch_size: u32,
    channels: u32,
    height: u32,
    width: u32,
    spatial_size: u32,
    epsilon: f32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> gamma: array<f32>;  // [channels]
@group(0) @binding(2) var<storage, read> beta: array<f32>;     // [channels]
@group(0) @binding(3) var<storage, read_write> sum_sq_buffer: array<f32>; // [batch * channels] - for reduction
@group(0) @binding(4) var<storage, read_write> output: array<f32>;
@group(0) @binding(5) var<uniform> params: Params;

// Step 1: Compute squared sum for each filter
@compute @workgroup_size(256)
fn compute_sum_sq(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_elements = params.batch_size * params.channels * params.spatial_size;
    
    if (idx >= total_elements) {
        return;
    }
    
    // Compute which batch, channel, and spatial position
    let spatial_idx = idx % params.spatial_size;
    let channel_idx = (idx / params.spatial_size) % params.channels;
    let batch_idx = idx / (params.channels * params.spatial_size);
    
    let value = input[idx];
    let sq_value = value * value;
    
    // Atomic add to sum_sq_buffer[batch_idx * channels + channel_idx]
    let buffer_idx = batch_idx * params.channels + channel_idx;
    atomicAdd(&sum_sq_buffer[buffer_idx], sq_value);
}

// Step 2: Normalize and apply scale/shift
@compute @workgroup_size(256)
fn normalize_and_scale(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_elements = params.batch_size * params.channels * params.spatial_size;
    
    if (idx >= total_elements) {
        return;
    }
    
    // Compute which batch, channel, and spatial position
    let spatial_idx = idx % params.spatial_size;
    let channel_idx = (idx / params.spatial_size) % params.channels;
    let batch_idx = idx / (params.channels * params.spatial_size);
    
    // Get sum of squares for this filter
    let buffer_idx = batch_idx * params.channels + channel_idx;
    let sum_sq = sum_sq_buffer[buffer_idx];
    
    // Compute nu = sqrt(sum_sq / spatial_size)
    let nu = sqrt(sum_sq / f32(params.spatial_size));
    
    // Normalize: x_norm = x / (nu + epsilon)
    let normalized = input[idx] / (nu + params.epsilon);
    
    // Scale and shift: output = gamma * normalized + beta
    output[idx] = gamma[channel_idx] * normalized + beta[channel_idx];
}
