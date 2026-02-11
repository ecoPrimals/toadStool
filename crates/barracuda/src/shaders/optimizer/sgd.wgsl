// SGD (Stochastic Gradient Descent) Optimizer
// The fundamental optimization algorithm
//
// Update rule: weight = weight - learning_rate * gradient
//
// Optional momentum:
// velocity = momentum * velocity - learning_rate * gradient
// weight = weight + velocity

@group(0) @binding(0) var<storage, read> weights: array<f32>;
@group(0) @binding(1) var<storage, read> gradients: array<f32>;
@group(0) @binding(2) var<storage, read> velocity_in: array<f32>;    // Optional, for momentum
@group(0) @binding(3) var<storage, read_write> weights_out: array<f32>;
@group(0) @binding(4) var<storage, read_write> velocity_out: array<f32>;
@group(0) @binding(5) var<uniform> params: Params;

struct Params {
    learning_rate: f32,
    momentum: f32,        // 0.0 for no momentum, typically 0.9
    weight_decay: f32,    // L2 regularization, typically 0.0001
    dampening: f32,       // Dampening for momentum, typically 0.0
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&weights) {
        return;
    }
    
    let w = weights[idx];
    var g = gradients[idx];
    
    // Apply weight decay (L2 regularization)
    if params.weight_decay != 0.0 {
        g = g + params.weight_decay * w;
    }
    
    // Apply momentum if enabled
    var velocity = 0.0;
    if params.momentum != 0.0 {
        velocity = velocity_in[idx];
        velocity = params.momentum * velocity + (1.0 - params.dampening) * g;
        velocity_out[idx] = velocity;
        g = velocity;
    }
    
    // Update weights
    weights_out[idx] = w - params.learning_rate * g;
}
