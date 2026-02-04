// SGDW - SGD with Decoupled Weight Decay (Loshchilov & Hutter)
// More principled weight decay than L2 regularization
// Decouples weight decay from gradient-based update
//
// Algorithm:
// 1. Momentum update: v_t = momentum * v_{t-1} + g_t
// 2. Parameter update: w_t = w - lr * v_t - lr * λ * w
// Note: Weight decay is applied directly to parameters, not through gradients

struct Params {
    size: u32,
    learning_rate: f32,
    momentum: f32,
    weight_decay: f32,
    dampening: f32,
    nesterov: u32,  // 0 or 1
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> parameters: array<f32>;
@group(0) @binding(2) var<storage, read> gradients: array<f32>;
@group(0) @binding(3) var<storage, read_write> velocity: array<f32>;
@group(0) @binding(4) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.size) {
        return;
    }

    let grad = gradients[idx];
    let param = parameters[idx];
    let v = velocity[idx];
    
    // Update velocity (momentum)
    var new_v: f32;
    if (params.momentum > 0.0) {
        if (v == 0.0) {
            // First step: no dampening
            new_v = grad;
        } else {
            // Subsequent steps: apply momentum and dampening
            new_v = params.momentum * v + (1.0 - params.dampening) * grad;
        }
    } else {
        new_v = grad;
    }
    velocity[idx] = new_v;
    
    // Compute gradient update
    var update: f32;
    if (params.nesterov != 0u) {
        // Nesterov momentum: use look-ahead gradient
        update = grad + params.momentum * new_v;
    } else {
        // Standard momentum
        update = new_v;
    }
    
    // Apply gradient update and decoupled weight decay
    // w_t = w - lr * update - lr * λ * w
    let grad_update = params.learning_rate * update;
    let decay_update = params.learning_rate * params.weight_decay * param;
    
    output[idx] = param - grad_update - decay_update;
}
