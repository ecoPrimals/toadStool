// Tanh: Hyperbolic tangent activation
// CUDA equivalent: cudnn::Activation(TANH)
// Formula: tanh(x) = (exp(x) - exp(-x)) / (exp(x) + exp(-x))
// Use cases: Activation function, output normalization

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;
    
    if (gid >= arrayLength(&output)) {
        return;
    }
    
    let x = input[gid];
    
    // Use built-in tanh for numerical stability
    output[gid] = tanh(x);
}
