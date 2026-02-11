// dequantize.wgsl - Convert quantized integers to floating point
//
// Dequantization: Convert low-precision integers back to FP32
// Used for inference with quantized models

struct Params {
    size: u32,
    scale: f32,
    zero_point: f32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;     // Quantized values (as f32, will be cast to i32)
@group(0) @binding(1) var<storage, read_write> output: array<f32>; // Dequantized floats
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.size) {
        return;
    }
    
    // Dequantize: (quantized_value - zero_point) * scale
    // Cast f32 to i32 for quantized integer value, then back to f32 for computation
    let quantized_int = i32(input[idx]);
    let quantized = f32(quantized_int);
    let dequantized = (quantized - params.zero_point) * params.scale;
    
    output[idx] = dequantized;
}
