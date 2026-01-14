// MSE (Mean Squared Error) Loss
// Fundamental loss function for regression tasks
//
// Formula: MSE = mean((predictions - targets)²)
//
// Supports three reduction modes:
// - Mean: average over all elements (default)
// - Sum: sum of all squared errors
// - None: per-element squared errors (no reduction)

@group(0) @binding(0) var<storage, read> predictions: array<f32>;
@group(0) @binding(1) var<storage, read> targets: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

struct Params {
    reduction_mode: u32,  // 0=mean, 1=sum, 2=none
    size: u32,            // Total number of elements
    _padding: vec2<u32>,  // Alignment to 16 bytes
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.size {
        return;
    }
    
    let pred = predictions[idx];
    let target = targets[idx];
    
    // Compute squared error
    let diff = pred - target;
    let squared_error = diff * diff;
    
    // Apply reduction based on mode
    if params.reduction_mode == 2u {  // None: per-element
        output[idx] = squared_error;
    } else {
        // For mean/sum, we write to output and reduce later
        // This is a parallel reduction pattern
        output[idx] = squared_error;
    }
}
