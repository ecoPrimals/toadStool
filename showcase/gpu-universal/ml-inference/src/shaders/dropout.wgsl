// Dropout: Dropout regularization with GPU RNG
// CUDA equivalent: cudnn::Dropout
// Algorithm: Philox RNG for GPU random number generation
// Use cases: Regularization, preventing overfitting

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<storage, read_write> mask: array<u32>;  // Optional: store mask for backprop

struct Params {
    size: u32,
    dropout_prob: f32,  // Probability of dropping (0.0 to 1.0)
    training: u32,      // 0=inference (no dropout), 1=training
    seed: u32,
}
@group(0) @binding(3) var<uniform> params: Params;

// Simple Philox-like RNG for GPU
// Based on: "Parallel Random Numbers: As Easy as 1, 2, 3" by Salmon et al.
fn philox_round(counter: vec2<u32>, key: u32) -> vec2<u32> {
    let multiplier_lo = 0xD2511F53u;
    let multiplier_hi = 0xCD9E8D57u;
    
    let prod_lo = counter.x * multiplier_lo;
    let prod_hi = counter.y * multiplier_hi;
    
    return vec2<u32>(
        prod_hi ^ counter.y ^ key,
        prod_lo ^ counter.x
    );
}

fn philox4x32(counter: u32, key: u32) -> u32 {
    var ctr = vec2<u32>(counter, counter);
    
    // 10 rounds
    for (var i = 0u; i < 10u; i++) {
        ctr = philox_round(ctr, key + i);
    }
    
    return ctr.x ^ ctr.y;
}

fn random_uniform(gid: u32, seed: u32) -> f32 {
    let rand_val = philox4x32(gid, seed);
    // Convert to [0, 1) range
    return f32(rand_val) / 4294967296.0;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;
    
    if (gid >= params.size) {
        return;
    }
    
    // Inference mode: no dropout
    if (params.training == 0u) {
        output[gid] = input[gid];
        return;
    }
    
    // Training mode: apply dropout
    let rand_val = random_uniform(gid, params.seed);
    let keep = rand_val >= params.dropout_prob;
    
    if (keep) {
        // Scale by (1 - dropout_prob) for inverted dropout
        let scale = 1.0 / (1.0 - params.dropout_prob);
        output[gid] = input[gid] * scale;
        mask[gid] = 1u;
    } else {
        output[gid] = 0.0;
        mask[gid] = 0u;
    }
}
