// Sign function - element-wise
// sign(x) = -1 if x < 0, 0 if x == 0, 1 if x > 0

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= arrayLength(&input)) {
        return;
    }
    
    let x = input[idx];
    
    if (x < 0.0) {
        output[idx] = -1.0;
    } else if (x > 0.0) {
        output[idx] = 1.0;
    } else {
        output[idx] = 0.0;
    }
}
