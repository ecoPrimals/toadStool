// Concatenate operation - Join tensors along a dimension
// Supports concatenation along any axis

struct ConcatParams {
    input1_size: u32,
    input2_size: u32,
    axis_dim1: u32,    // Size of input1 along concat axis
    axis_dim2: u32,    // Size of input2 along concat axis
    stride: u32,       // Stride for the concat axis
}

@group(0) @binding(0) var<storage, read> input1: array<f32>;
@group(0) @binding(1) var<storage, read> input2: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: ConcatParams;

@compute @workgroup_size(256)
fn concat_1d(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_size = params.input1_size + params.input2_size;
    
    if (idx >= total_size) {
        return;
    }
    
    // Simple 1D concatenation
    if (idx < params.input1_size) {
        output[idx] = input1[idx];
    } else {
        output[idx] = input2[idx - params.input1_size];
    }
}

@compute @workgroup_size(256)
fn concat_axis(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total_size = params.input1_size + params.input2_size;
    
    if (idx >= total_size) {
        return;
    }
    
    // Calculate which input and offset
    let block = idx / params.stride;
    let offset = idx % params.stride;
    
    let block_size1 = params.axis_dim1 * params.stride;
    let block_size2 = params.axis_dim2 * params.stride;
    let total_block = block_size1 + block_size2;
    
    let block_idx = idx / total_block;
    let in_block_idx = idx % total_block;
    
    if (in_block_idx < block_size1) {
        // From input1
        let src_idx = block_idx * block_size1 + in_block_idx;
        if (src_idx < params.input1_size) {
            output[idx] = input1[src_idx];
        }
    } else {
        // From input2
        let src_idx = block_idx * block_size2 + (in_block_idx - block_size1);
        if (src_idx < params.input2_size) {
            output[idx] = input2[src_idx];
        }
    }
}
