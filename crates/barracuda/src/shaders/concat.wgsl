// Concatenate tensors along axis 0
// Simplified version: concatenate two 1D tensors

@group(0) @binding(0) var<storage, read> input1: array<f32>;
@group(0) @binding(1) var<storage, read> input2: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let size1 = arrayLength(&input1);
    let size2 = arrayLength(&input2);
    let total = size1 + size2;
    
    if (idx >= total) {
        return;
    }
    
    if (idx < size1) {
        output[idx] = input1[idx];
    } else {
        output[idx] = input2[idx - size1];
    }
}
