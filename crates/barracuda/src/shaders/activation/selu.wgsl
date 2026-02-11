// SELU - Scaled Exponential Linear Unit
//
// Deep Debt Principles:
// - Pure WGSL implementation (universal compute)
// - Zero unsafe code (memory safe)
// - Hardware-agnostic (works on any GPU/CPU via WebGPU)
// - Self-contained logic (no external dependencies)

// SELU constants from the paper "Self-Normalizing Neural Networks"
const ALPHA: f32 = 1.67326324;
const SCALE: f32 = 1.05070098;

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> size: u32;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= size) {
        return;
    }
    
    let x = input[idx];
    if (x >= 0.0) {
        output[idx] = SCALE * x;
    } else {
        output[idx] = SCALE * ALPHA * (exp(x) - 1.0);
    }
}
