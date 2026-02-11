// cyclical_lr.wgsl - Cyclical learning rate schedule
//
// Cycles learning rate between bounds for better convergence
// Reference: "Cyclical Learning Rates for Training Neural Networks"
// by Smith (2017)
//
// Note: This is a utility shader, typically computed on CPU
// Included for completeness in universal compute platform

struct Params {
    current_iter: u32,
    step_size: u32,      // Half-cycle length
    base_lr: f32,
    max_lr: f32,
    mode: u32,           // 0 = triangular, 1 = triangular2, 2 = exp_range
    gamma: f32,          // For exp_range mode
    _padding: vec2<u32>,
}

@group(0) @binding(0) var<storage, read_write> output: array<f32>; // Single LR value
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(1)
fn main() {
    let cycle = f32(params.current_iter / (2u * params.step_size)) + 1.0;
    let x = abs(f32(params.current_iter % (2u * params.step_size)) / f32(params.step_size) - 1.0);
    
    var lr: f32;
    
    if (params.mode == 0u) {
        // Triangular
        lr = params.base_lr + (params.max_lr - params.base_lr) * max(0.0, 1.0 - x);
    } else if (params.mode == 1u) {
        // Triangular2 (amplitude decreases)
        lr = params.base_lr + (params.max_lr - params.base_lr) / pow(2.0, cycle - 1.0) * max(0.0, 1.0 - x);
    } else {
        // Exp range
        lr = params.base_lr + (params.max_lr - params.base_lr) * pow(params.gamma, f32(params.current_iter)) * max(0.0, 1.0 - x);
    }
    
    output[0] = lr;
}
