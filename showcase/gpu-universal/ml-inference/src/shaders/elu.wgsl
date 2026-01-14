// ELU (Exponential Linear Unit) Activation
// Addresses dying ReLU with smooth negative part
//
// Formula: ELU(x) = x if x > 0, else α * (exp(x) - 1)
// where α is typically 1.0
//
// Properties:
// - Smooth and differentiable everywhere
// - Mean activation closer to zero
// - Reduces bias shift effect

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

struct Params {
    alpha: f32,           // Typically 1.0
    _padding: vec3<f32>,  // Alignment to 16 bytes
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&input) {
        return;
    }
    
    let x = input[idx];
    
    // ELU: x if x > 0, else alpha * (exp(x) - 1)
    if x > 0.0 {
        output[idx] = x;
    } else {
        output[idx] = params.alpha * (exp(x) - 1.0);
    }
}
