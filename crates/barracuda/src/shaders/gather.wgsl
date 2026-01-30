// Gather - Advanced indexing operation
// output[i] = input[indices[i]]
// Simplified 1D version

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> indices: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= arrayLength(&indices)) {
        return;
    }
    
    let src_idx = indices[idx];
    
    if (src_idx < arrayLength(&input)) {
        output[idx] = input[src_idx];
    } else {
        output[idx] = 0.0;
    }
}
