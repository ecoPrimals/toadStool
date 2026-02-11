// AdaDelta Optimizer
// Extension of AdaGrad that seeks to reduce monotonically decreasing learning rate
//
// Update rule:
// E[g²] = rho * E[g²] + (1 - rho) * g²
// delta = sqrt((E[delta²] + epsilon) / (E[g²] + epsilon)) * g
// E[delta²] = rho * E[delta²] + (1 - rho) * delta²
// weight = weight - delta
//
// Benefits: No learning rate hyperparameter needed!
// Used in: When you want to avoid tuning learning rates

@group(0) @binding(0) var<storage, read> weights: array<f32>;
@group(0) @binding(1) var<storage, read> gradients: array<f32>;
@group(0) @binding(2) var<storage, read> acc_grad_in: array<f32>;    // E[g²]
@group(0) @binding(3) var<storage, read> acc_delta_in: array<f32>;   // E[delta²]
@group(0) @binding(4) var<storage, read_write> weights_out: array<f32>;
@group(0) @binding(5) var<storage, read_write> acc_grad_out: array<f32>;
@group(0) @binding(6) var<storage, read_write> acc_delta_out: array<f32>;
@group(0) @binding(7) var<uniform> params: Params;

struct Params {
    rho: f32,             // Decay rate, typically 0.95
    epsilon: f32,         // Numerical stability, typically 1e-6
    weight_decay: f32,    // L2 regularization
    _padding: u32,
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&weights) {
        return;
    }
    
    let w = weights[idx];
    var g = gradients[idx];
    let eg2 = acc_grad_in[idx];
    let ed2 = acc_delta_in[idx];
    
    // Apply weight decay
    if params.weight_decay != 0.0 {
        g = g + params.weight_decay * w;
    }
    
    // Accumulate gradient squared
    let eg2_new = params.rho * eg2 + (1.0 - params.rho) * g * g;
    acc_grad_out[idx] = eg2_new;
    
    // Compute update
    let rms_delta = sqrt(ed2 + params.epsilon);
    let rms_grad = sqrt(eg2_new + params.epsilon);
    let delta = (rms_delta / rms_grad) * g;
    
    // Accumulate delta squared
    let ed2_new = params.rho * ed2 + (1.0 - params.rho) * delta * delta;
    acc_delta_out[idx] = ed2_new;
    
    // Update weights
    weights_out[idx] = w - delta;
}
