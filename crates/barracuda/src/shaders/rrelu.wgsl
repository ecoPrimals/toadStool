// RReLU - Randomized Leaky ReLU
//
// Deep Debt Principles:
// - Pure WGSL implementation (universal compute)
// - Zero unsafe code (memory safe)
// - Hardware-agnostic (works on any GPU/CPU via WebGPU)
// - Self-contained logic (no external dependencies)
// - Deterministic randomness (seed-based LCG)

struct Params {
    size: u32,
    lower: f32,
    upper: f32,
    seed: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

// Linear Congruential Generator for pseudo-random numbers
fn lcg(seed: u32) -> u32 {
    return (1103515245u * seed + 12345u) & 0x7fffffffu;
}

fn random_f32(seed: u32) -> f32 {
    return f32(lcg(seed)) / f32(0x7fffffffu);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.size) {
        return;
    }
    
    let x = input[idx];
    
    if (x > 0.0) {
        output[idx] = x;
    } else {
        // Generate random slope in [lower, upper] range
        let seed = params.seed + idx;
        let rand = random_f32(seed);
        let slope = params.lower + rand * (params.upper - params.lower);
        output[idx] = slope * x;
    }
}
