// Dropout - Regularization (neuromorphic essential)
// During training: randomly zero elements with probability p
// During inference: identity (scale handled in training)
// Simplified: deterministic pattern for testing

struct DropoutParams {
    rate: f32,
    seed: u32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: DropoutParams;

// Simple hash function for pseudo-random
fn hash(x: u32) -> u32 {
    var h = x ^ params.seed;
    h = h * 0x85ebca6bu;
    h = h ^ (h >> 13u);
    h = h * 0xc2b2ae35u;
    h = h ^ (h >> 16u);
    return h;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= arrayLength(&input)) {
        return;
    }
    
    // Generate pseudo-random value
    let random = f32(hash(idx)) / f32(0xffffffffu);
    
    // Apply dropout
    if (random < params.rate) {
        output[idx] = 0.0;
    } else {
        // Scale to maintain expected value
        output[idx] = input[idx] / (1.0 - params.rate);
    }
}
