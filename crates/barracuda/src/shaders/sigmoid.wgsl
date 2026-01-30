// Sigmoid: Sigmoid activation function
// CUDA equivalent: cudnn::Activation(SIGMOID)
// Formula: sigmoid(x) = 1 / (1 + exp(-x))
// Use cases: Binary classification, gate activations (LSTM, GRU)

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;
    
    if (gid >= arrayLength(&output)) {
        return;
    }
    
    let x = input[gid];
    
    // Numerically stable sigmoid
    if (x >= 0.0) {
        let z = exp(-x);
        output[gid] = 1.0 / (1.0 + z);
    } else {
        let z = exp(x);
        output[gid] = z / (1.0 + z);
    }
}
