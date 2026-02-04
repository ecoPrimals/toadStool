// Gather ND - N-dimensional gather operation
//
// Deep Debt Principles:
// - Pure WGSL implementation (universal compute)
// - Zero unsafe code (memory safe)
// - Hardware-agnostic (works on any GPU/CPU via WebGPU)
// - Self-contained logic (no external dependencies)

struct Params {
    input_size: u32,
    indices_size: u32,
    output_size: u32,
    input_rank: u32,
    indices_rank: u32,
    index_rank: u32,
    batch_size: u32,
    num_indices: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input: array<f32>;
@group(0) @binding(2) var<storage, read> indices: array<f32>; // Stored as f32 for flexibility
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<storage, read> input_shape: array<u32>;
@group(0) @binding(5) var<storage, read> indices_shape: array<u32>;
@group(0) @binding(6) var<storage, read> output_shape: array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.output_size) {
        return;
    }
    
    // Decompose output index
    // Output shape: [batch_size, num_indices] + input_shape[index_rank..]
    var output_coords: array<u32, 8> = array<u32, 8>(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u);
    var temp_idx = idx;
    
    // Compute output coordinates
    for (var i = 0u; i < params.indices_rank - 1u && i < 8u; i++) {
        var dim_size = output_shape[i];
        output_coords[i] = temp_idx % dim_size;
        temp_idx = temp_idx / dim_size;
    }
    
    let batch_idx = output_coords[0];
    let index_idx = output_coords[1];
    
    // Get indices for this output element
    let indices_offset = batch_idx * params.num_indices * params.index_rank + index_idx * params.index_rank;
    
    // Read index coordinates
    var index_coords: array<u32, 8> = array<u32, 8>(0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u);
    for (var i = 0u; i < params.index_rank && i < 8u; i++) {
        index_coords[i] = u32(indices[indices_offset + i]);
    }
    
    // Compute input linear index from index coordinates
    var input_strides: array<u32, 8> = array<u32, 8>(1u, 1u, 1u, 1u, 1u, 1u, 1u, 1u);
    for (var i = params.input_rank - 2u; i >= 0u && i < 8u; i--) {
        input_strides[i] = input_strides[i + 1u] * input_shape[i + 1u];
    }
    
    var input_idx: u32 = 0u;
    for (var i = 0u; i < params.index_rank && i < 8u; i++) {
        input_idx = input_idx + index_coords[i] * input_strides[i];
    }
    
    // Handle remaining dimensions from input
    // For now, simplified: assume we're gathering full slices
    // TODO: Handle partial gathering correctly
    
    // Bounds check
    if (input_idx < params.input_size) {
        output[idx] = input[input_idx];
    } else {
        output[idx] = 0.0;
    }
}
