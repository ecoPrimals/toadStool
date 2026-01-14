// Reshape operation - Change tensor shape without copying data
// Note: Reshape is often a no-op on GPU (just metadata change)
// This shader is for cases where memory layout needs adjustment

struct ReshapeParams {
    size: u32,
    _pad: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: ReshapeParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.size) {
        return;
    }
    
    // Simple copy - reshape is primarily a metadata operation
    output[idx] = input[idx];
}
