// poisson_nll_loss.wgsl - Poisson Negative Log Likelihood Loss
//
// Loss for Poisson distributed data (count data, rates)
// Loss = exp(input) - target * input  (if log_input=False)
// Loss = exp(input) - target * log(exp(input))  (if log_input=True)
//
// Used in count regression, neural activity modeling

struct Params {
    size: u32,
    log_input: u32,     // 1 if input is log-space, 0 if normal space
    full: u32,          // 1 to include Stirling approximation term
    epsilon: f32,       // For numerical stability
}

@group(0) @binding(0) var<storage, read> input: array<f32>;        // Predicted values
@group(0) @binding(1) var<storage, read> target: array<f32>;       // Target counts
@group(0) @binding(2) var<storage, read_write> output: array<f32>; // Per-element loss
@group(0) @binding(3) var<uniform> params: Params;

// Stirling approximation for log(n!)
fn stirling_approx(n: f32) -> f32 {
    if (n <= 1.0) {
        return 0.0;
    }
    return n * log(n) - n + 0.5 * log(2.0 * 3.14159265359 * n);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.size) {
        return;
    }
    
    let pred = input[idx];
    let tgt = target[idx];
    
    var loss: f32;
    
    if (params.log_input == 1u) {
        // Input is in log space
        loss = exp(pred) - tgt * pred;
    } else {
        // Input is in normal space
        let pred_clamped = max(pred, params.epsilon);
        loss = pred_clamped - tgt * log(pred_clamped);
    }
    
    // Add Stirling approximation term if requested
    if (params.full == 1u && tgt > 1.0) {
        loss = loss + stirling_approx(tgt);
    }
    
    output[idx] = loss;
}
