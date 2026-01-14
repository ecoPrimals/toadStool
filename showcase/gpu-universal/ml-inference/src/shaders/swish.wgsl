// Swish / SiLU (Sigmoid Linear Unit) Activation
// Used in EfficientNet, MobileNetV3, and modern architectures
//
// Formula: Swish(x) = x * sigmoid(x) = x / (1 + exp(-x))
//
// Properties:
// - Non-monotonic (unlike ReLU)
// - Smooth and differentiable everywhere
// - Self-gated activation

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&input) {
        return;
    }
    
    let x = input[idx];
    
    // Swish(x) = x * sigmoid(x)
    let sigmoid_x = 1.0 / (1.0 + exp(-x));
    output[idx] = x * sigmoid_x;
}
