// SELU (Scaled Exponential Linear Unit) Activation
// Self-normalizing activation function for deep networks
//
// SELU(x) = scale * x                           if x > 0
//         = scale * alpha * (exp(x) - 1)        if x <= 0
//
// With specific constants that enable self-normalization:
// alpha ≈ 1.67326324
// scale ≈ 1.05070098
//
// Used in: Self-Normalizing Neural Networks (SNNs)
// Benefits: Automatically converges towards zero mean and unit variance

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

// SELU constants (proven optimal for self-normalization)
const ALPHA: f32 = 1.67326324;
const SCALE: f32 = 1.05070098;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&input) {
        return;
    }
    
    let x = input[idx];
    
    // SELU computation with self-normalizing constants
    if x > 0.0 {
        output[idx] = SCALE * x;
    } else {
        output[idx] = SCALE * ALPHA * (exp(x) - 1.0);
    }
}
