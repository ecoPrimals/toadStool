// AvgPool2D - Average pooling (2D)
// Simplified version: 2x2 pooling with stride 2

struct AvgPool2DParams {
    input_width: u32,
    input_height: u32,
    pool_size: u32,
    stride: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: AvgPool2DParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_x = global_id.x;
    let out_y = global_id.y;
    
    let output_width = params.input_width / params.stride;
    let output_height = params.input_height / params.stride;
    
    if (out_x >= output_width || out_y >= output_height) {
        return;
    }
    
    // Calculate input region
    let in_x = out_x * params.stride;
    let in_y = out_y * params.stride;
    
    // Compute average in pool window
    var sum = 0.0;
    var count = 0u;
    
    for (var dy = 0u; dy < params.pool_size; dy = dy + 1u) {
        for (var dx = 0u; dx < params.pool_size; dx = dx + 1u) {
            let x = in_x + dx;
            let y = in_y + dy;
            
            if (x < params.input_width && y < params.input_height) {
                let idx = y * params.input_width + x;
                sum = sum + input[idx];
                count = count + 1u;
            }
        }
    }
    
    let out_idx = out_y * output_width + out_x;
    output[out_idx] = sum / f32(count);
}
