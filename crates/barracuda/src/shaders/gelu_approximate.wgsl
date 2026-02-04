// GELU Approximate - Fast approximation of Gaussian Error Linear Unit
// gelu(x) ≈ 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x^3)))

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> size: u32;

const SQRT_2_OVER_PI: f32 = 0.7978845608;  // sqrt(2/π)
const COEFF: f32 = 0.044715;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= size) {
        return;
    }
    
    let x = input[idx];
    let x_cubed = x * x * x;
    let inner = SQRT_2_OVER_PI * (x + COEFF * x_cubed);
    let result = 0.5 * x * (1.0 + tanh(inner));
    
    output[idx] = result;
}
