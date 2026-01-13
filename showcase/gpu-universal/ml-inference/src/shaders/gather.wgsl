// Gather: Indirect read with index array
// CUDA equivalent: thrust::gather
// Use cases: Embedding lookup, sparse access, graph neural networks

@group(0) @binding(0) var<storage, read> source: array<f32>;
@group(0) @binding(1) var<storage, read> indices: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

struct Params {
    num_elements: u32,
    source_size: u32,
}
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;
    
    if (gid >= params.num_elements) {
        return;
    }
    
    let idx = indices[gid];
    
    // Bounds check
    if (idx < params.source_size) {
        output[gid] = source[idx];
    } else {
        output[gid] = 0.0;  // Out of bounds = 0
    }
}
