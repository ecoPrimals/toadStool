// Flip - reverse order of elements
// Simplified: flip 1D tensor

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let size = arrayLength(&input);
    
    if (idx >= size) {
        return;
    }
    
    output[idx] = input[size - 1u - idx];
}
