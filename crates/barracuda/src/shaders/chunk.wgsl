// Chunk - Split tensor into N chunks along dimension (complete implementation)
// Divides input tensor into equal-sized chunks
//
// Example: chunk([B, 10, H, W], chunks=2, dim=1) → [[B, 5, H, W], [B, 5, H, W]]
//
// Algorithm:
// For each chunk, copy the appropriate slice of the input tensor

struct Params {
    chunk_idx: u32,      // Which chunk we're computing
    chunk_size: u32,     // Size of each chunk along split dimension
    split_dim: u32,      // Dimension to split
    dim_size: u32,       // Size of dimension being split
    inner_size: u32,     // Product of dimensions after split_dim
    outer_size: u32,     // Product of dimensions before split_dim
    output_size: u32,    // Total output size for this chunk
    _pad1: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_idx = global_id.x;
    if (out_idx >= params.output_size) {
        return;
    }

    // Decompose output index
    let outer = out_idx / (params.chunk_size * params.inner_size);
    let temp = out_idx % (params.chunk_size * params.inner_size);
    let chunk_coord = temp / params.inner_size;
    let inner = temp % params.inner_size;
    
    // Map to input index
    let input_coord = params.chunk_idx * params.chunk_size + chunk_coord;
    let in_idx = outer * params.dim_size * params.inner_size 
                 + input_coord * params.inner_size 
                 + inner;
    
    output[out_idx] = input[in_idx];
}
