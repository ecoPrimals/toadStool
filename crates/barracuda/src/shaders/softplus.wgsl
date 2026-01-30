// Softplus activation - smooth approximation of ReLU
// softplus(x) = ln(1 + exp(x))

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= arrayLength(&input)) {
        return;
    }
    
    let x = input[idx];
    
    // For numerical stability:
    // if x > 20, softplus(x) ≈ x
    // if x < -20, softplus(x) ≈ 0
    if (x > 20.0) {
        output[idx] = x;
    } else if (x < -20.0) {
        output[idx] = 0.0;
    } else {
        output[idx] = log(1.0 + exp(x));
    }
}
