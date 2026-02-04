// nll_loss.wgsl - Negative Log Likelihood Loss
//
// Standard loss for classification with log probabilities
// Loss = -log_probs[batch_idx, target[batch_idx]]
//
// Commonly used with log_softmax output

struct Params {
    batch_size: u32,
    num_classes: u32,
    ignore_index: i32,  // Class to ignore (e.g., padding), -1 = none
    reduction: u32,      // 0 = none, 1 = mean, 2 = sum
}

@group(0) @binding(0) var<storage, read> log_probs: array<f32>;    // [batch, num_classes]
@group(0) @binding(1) var<storage, read> targets: array<u32>;      // [batch]
@group(0) @binding(2) var<storage, read> weights: array<f32>;      // [num_classes] - optional class weights
@group(0) @binding(3) var<storage, read_write> output: array<f32>; // [batch] or [1] depending on reduction
@group(0) @binding(4) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let b = global_id.x;
    
    if (b >= params.batch_size) {
        return;
    }
    
    let target = i32(targets[b]);
    
    // Check if this sample should be ignored
    if (target == params.ignore_index || target < 0 || target >= i32(params.num_classes)) {
        output[b] = 0.0;
        return;
    }
    
    let target_u = u32(target);
    
    // Get log probability for true class
    let log_prob = log_probs[b * params.num_classes + target_u];
    
    // Apply class weight
    let weight = weights[target_u];
    
    // NLL loss = -log_prob * weight
    let loss = -log_prob * weight;
    
    output[b] = loss;
}
