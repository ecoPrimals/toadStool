// Pad operation - Add padding to tensors
// Supports constant, reflect, and replicate modes

struct PadParams {
    input_height: u32,   // offset 0
    input_width: u32,    // offset 4
    output_height: u32,  // offset 8
    output_width: u32,   // offset 12
    pad_top: u32,        // offset 16
    pad_left: u32,       // offset 20
    pad_value: f32,      // offset 24
    _pad: u32,           // offset 28
    _pad2: u32,          // offset 32
    _pad3: u32,          // offset 36 (total 40 bytes)
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
        // Constant padding
        value = params.pad_value;
    } else {
        // Inside input region, copy directly
        let in_idx = u32(in_y) * params.input_width + u32(in_x);
        value = input[in_idx];
    }
    
    // Write output
    let out_idx = out_y * params.output_width + out_x;
    output[out_idx] = value;
}
