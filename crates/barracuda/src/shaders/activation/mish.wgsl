// Mish - Self-regularizing activation function
// mish(x) = x * tanh(softplus(x)) = x * tanh(ln(1 + e^x))

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
    
    // softplus(x) = ln(1 + e^x)
    // For numerical stability, use different formula for large x
    var softplus: f32;
    if (x > 20.0) {
        softplus = x;  // For large x, ln(1 + e^x) ≈ x
    } else {
        softplus = log(1.0 + exp(x));
    }
    
    // mish(x) = x * tanh(softplus(x))
    let result = x * tanh(softplus);
    
    output[idx] = result;
}
