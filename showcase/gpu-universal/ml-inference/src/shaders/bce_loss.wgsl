// BCE (Binary Cross Entropy) Loss
// Binary classification loss function
//
// BCE(p, t) = -[t * log(p) + (1 - t) * log(1 - p)]
//
// where p = predictions (probabilities in [0, 1])
//       t = targets (0 or 1)
//
// Used in: Binary classification, multi-label classification, GANs

@group(0) @binding(0) var<storage, read> predictions: array<f32>;
@group(0) @binding(1) var<storage, read> targets: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

struct Params {
    epsilon: f32,         // Small constant to prevent log(0), typically 1e-7
    reduction_mode: u32,  // 0=mean, 1=sum, 2=none
    size: u32,
    _padding: u32,
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= params.size {
        return;
    }
    
    let pred = predictions[idx];
    let target = targets[idx];
    
    // Clamp predictions to [epsilon, 1-epsilon] to prevent log(0)
    let pred_clamped = clamp(pred, params.epsilon, 1.0 - params.epsilon);
    
    // Compute BCE: -[t * log(p) + (1 - t) * log(1 - p)]
    let loss = -(target * log(pred_clamped) + (1.0 - target) * log(1.0 - pred_clamped));
    
    output[idx] = loss;
}
