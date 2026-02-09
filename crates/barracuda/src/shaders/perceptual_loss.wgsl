// Perceptual Loss - Feature-based perceptual loss
// Compares high-level features instead of pixels
// Used in style transfer and super-resolution
//
// Algorithm:
// If weights provided:
//   loss = sum(weights[i] * (features1[i] - features2[i])^2) / size
// Else:
//   loss = sum((features1[i] - features2[i])^2) / size

struct Params {
    size: u32,
    has_weights: u32, // 1 if weights provided, 0 otherwise
    num_weights: u32, // Number of weight groups
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> features1: array<f32>;
@group(0) @binding(1) var<storage, read> features2: array<f32>;
@group(0) @binding(2) var<storage, read> weights: array<f32>; // Optional per-layer weights
@group(0) @binding(3) var<storage, read_write> loss_buffer: array<atomic<i32>>; // [1] - atomic reduction
@group(0) @binding(4) var<storage, read_write> output: array<f32>; // [1] - final loss
@group(0) @binding(5) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.size) {
        return;
    }
    
    let diff = features1[idx] - features2[idx];
    let sq_diff = diff * diff;
    
    var weighted_sq_diff: f32;
    if (params.has_weights == 1u) {
        // Compute which weight group this element belongs to
        let features_per_weight = params.size / params.num_weights;
        let weight_idx = idx / features_per_weight;
        let weight = weights[weight_idx];
        weighted_sq_diff = weight * sq_diff;
    } else {
        weighted_sq_diff = sq_diff;
    }
    
    // Atomic add to loss buffer
    atomicAdd(&loss_buffer[0], bitcast<i32>(weighted_sq_diff));
}

// Second pass: compute final loss (mean)
@compute @workgroup_size(1)
fn compute_mean_loss(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let total_loss = bitcast<f32>(atomicLoad(&loss_buffer[0]));
    output[0] = total_loss / f32(params.size);
}
