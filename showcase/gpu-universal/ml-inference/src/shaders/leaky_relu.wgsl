// LeakyReLU Activation
// Addresses dying ReLU problem by allowing small negative slope
//
// Formula: LeakyReLU(x) = max(αx, x)
// where α is typically 0.01 (negative slope coefficient)
//
// Properties:
// - Prevents dying neurons (unlike ReLU)
// - Non-zero gradient for negative inputs
// - Widely used in GANs and deep networks

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

struct Params {
    negative_slope: f32,  // Alpha: typically 0.01
    _padding: vec3<f32>,  // Alignment to 16 bytes
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&input) {
        return;
    }
    
    let x = input[idx];
    
    // LeakyReLU: x if x > 0, else alpha * x
    if x > 0.0 {
        output[idx] = x;
    } else {
        output[idx] = params.negative_slope * x;
    }
}
