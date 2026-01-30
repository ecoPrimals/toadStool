// Natural logarithm - element-wise
// log(x)

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= arrayLength(&input)) {
        return;
    }
    
    let x = input[idx];
    
    // Handle edge cases
    if (x <= 0.0) {
        output[idx] = -3.402823466e+38; // -FLT_MAX (effectively -inf)
    } else {
        output[idx] = log(x);
    }
}
