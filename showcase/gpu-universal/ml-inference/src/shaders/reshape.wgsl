// Reshape operation - Change tensor shape without copying data
// Note: Reshape is often a no-op on GPU (just metadata change)
// This shader is for cases where memory layout needs adjustment

struct ReshapeParams {
    total_size: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: ReshapeParams;

@compute @workgroup_size(256)
fn reshape_copy(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.total_size) {
        return;
    }
    
    // Simple copy - reshape is primarily a metadata operation
    output[idx] = input[idx];
}
