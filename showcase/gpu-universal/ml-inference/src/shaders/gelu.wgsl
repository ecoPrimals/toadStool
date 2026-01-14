// GELU (Gaussian Error Linear Unit) Activation
// Used extensively in BERT, GPT, and modern transformers
//
// Formula: GELU(x) = x * Φ(x)
// where Φ(x) is the cumulative distribution function of standard normal distribution
//
// Approximation: GELU(x) ≈ 0.5 * x * (1 + tanh(√(2/π) * (x + 0.044715 * x³)))

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

// Constants for GELU approximation
const SQRT_2_OVER_PI: f32 = 0.7978845608;  // sqrt(2/pi)
const GELU_CONSTANT: f32 = 0.044715;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if idx >= arrayLength(&input) {
        return;
    }
    
    let x = input[idx];
    
    // Compute GELU using tanh approximation
    // This is faster and more numerically stable than erf-based version
    let x_cubed = x * x * x;
    let inner = SQRT_2_OVER_PI * (x + GELU_CONSTANT * x_cubed);
    let tanh_val = tanh(inner);
    
    output[idx] = 0.5 * x * (1.0 + tanh_val);
}
