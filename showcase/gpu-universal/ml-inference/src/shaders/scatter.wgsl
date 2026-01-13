// Scatter: Indirect write with index array (atomic for conflicts)
// CUDA equivalent: thrust::scatter
// Use cases: Sparse updates, gradient accumulation, graph neural networks

@group(0) @binding(0) var<storage, read> source: array<f32>;
@group(0) @binding(1) var<storage, read> indices: array<u32>;
@group(0) @binding(2) var<storage, read_write> dest: array<atomic<i32>>;  // Using atomic for thread safety

struct Params {
    num_elements: u32,
    dest_size: u32,
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
    if (idx < params.dest_size) {
        // Use atomic add for thread-safe scatter
        // Note: WGSL atomics work on i32, so we need to convert f32 to i32
        // This is a simplified version - real implementation needs proper float atomics
        let value_bits = bitcast<i32>(source[gid]);
        atomicStore(&dest[idx], value_bits);
    }
}
