// Mish Activation
// Self-regularizing smooth non-monotonic activation function
//
// Mish(x) = x * tanh(softplus(x))
//         = x * tanh(ln(1 + exp(x)))
//
// Properties:
// - Smooth, non-monotonic
// - Unbounded above, bounded below
// - Self-regularizing (like SELU but different mechanism)
//
// Used in: YOLOv4, modern computer vision, deep networks
// Benefits: Better accuracy than ReLU/Swish in many tasks

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

// Softplus(x) = ln(1 + exp(x))
// Numerically stable implementation for large x
fn softplus(x: f32) -> f32 {
    if x > 20.0 {
        // For large x, softplus(x) ≈ x
        return x;
    } else if x < -20.0 {
        // For very negative x, softplus(x) ≈ 0
        return 0.0;
    } else {
        return log(1.0 + exp(x));
    }
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&input) {
        return;
    }
    
    let x = input[idx];
    
    // Mish(x) = x * tanh(softplus(x))
    let sp = softplus(x);
    output[idx] = x * tanh(sp);
}
