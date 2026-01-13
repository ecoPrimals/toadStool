// Pad operation - Add padding to tensors
// Supports constant, reflect, and replicate modes

struct PadParams {
    input_height: u32,
    input_width: u32,
    pad_top: u32,
    pad_bottom: u32,
    pad_left: u32,
    pad_right: u32,
    output_height: u32,
    output_width: u32,
    pad_mode: u32,  // 0=constant, 1=reflect, 2=replicate
    pad_value: f32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: PadParams;

@compute @workgroup_size(8, 8)
fn pad_2d(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_y = global_id.y;
    let out_x = global_id.x;
    
    // Check bounds
    if (out_y >= params.output_height || out_x >= params.output_width) {
        return;
    }
    
    // Calculate input coordinates
    let in_y = i32(out_y) - i32(params.pad_top);
    let in_x = i32(out_x) - i32(params.pad_left);
    
    var value: f32;
    
    // Check if we're in padding region
    if (in_y < 0 || in_y >= i32(params.input_height) || 
        in_x < 0 || in_x >= i32(params.input_width)) {
        
        // Handle padding based on mode
        if (params.pad_mode == 0u) {
            // Constant padding
            value = params.pad_value;
        } else if (params.pad_mode == 1u) {
            // Reflect padding
            var reflect_y = in_y;
            var reflect_x = in_x;
            
            // Reflect y
            if (reflect_y < 0) {
                reflect_y = -reflect_y - 1;
            } else if (reflect_y >= i32(params.input_height)) {
                reflect_y = 2 * i32(params.input_height) - reflect_y - 1;
            }
            
            // Reflect x
            if (reflect_x < 0) {
                reflect_x = -reflect_x - 1;
            } else if (reflect_x >= i32(params.input_width)) {
                reflect_x = 2 * i32(params.input_width) - reflect_x - 1;
            }
            
            // Clamp to valid range
            reflect_y = clamp(reflect_y, 0, i32(params.input_height) - 1);
            reflect_x = clamp(reflect_x, 0, i32(params.input_width) - 1);
            
            let in_idx = u32(reflect_y) * params.input_width + u32(reflect_x);
            value = input[in_idx];
        } else {
            // Replicate padding (mode 2)
            let rep_y = clamp(in_y, 0, i32(params.input_height) - 1);
            let rep_x = clamp(in_x, 0, i32(params.input_width) - 1);
            
            let in_idx = u32(rep_y) * params.input_width + u32(rep_x);
            value = input[in_idx];
        }
    } else {
        // Inside input region, copy directly
        let in_idx = u32(in_y) * params.input_width + u32(in_x);
        value = input[in_idx];
    }
    
    // Write output
    let out_idx = out_y * params.output_width + out_x;
    output[out_idx] = value;
}
