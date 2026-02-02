// Tanh: Hyperbolic tangent activation function
// CUDA equivalent: cudnn::Activation(TANH)
// Formula: tanh(x) = (exp(x) - exp(-x)) / (exp(x) + exp(-x))
// Use cases: RNN activations, normalization, bounded outputs

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;
    
    if (gid >= arrayLength(&output)) {
        return;
    }
    
    let x = input[gid];
    
    // Numerically stable tanh using builtin
    // WGSL has tanh() builtin which is optimized and stable
    output[gid] = tanh(x);
}
