// Put - Scatter operation with indexing
// Places values at specified indices in output tensor
//
// Example: put(zeros(4), [0, 2], [10, 30]) → [10, 0, 30, 0]
//
// Algorithm:
// For each value, scatter to position given by index

struct Params {
    output_size: u32,
    num_values: u32,
    accumulate: u32,  // 0 = overwrite, 1 = accumulate
    _pad1: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> values: array<f32>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.num_values) {
        return;
    }

    let out_idx = indices[idx];
    let value = values[idx];
    
    // Bounds check
    if (out_idx < params.output_size) {
        if (params.accumulate != 0u) {
            // Accumulate (requires atomic operations for correctness)
            atomicAdd(&output[out_idx], bitcast<i32>(value));
        } else {
            // Overwrite (last write wins if multiple indices point to same location)
            output[out_idx] = value;
        }
    }
}
