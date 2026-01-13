// AvgPool2D: 2D average pooling
// CUDA equivalent: cudnn::Pooling(AVG)
// Use cases: Smooth downsampling, global average pooling, feature aggregation

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

struct Params {
    batch_size: u32,
    channels: u32,
    input_height: u32,
    input_width: u32,
    output_height: u32,
    output_width: u32,
    kernel_h: u32,
    kernel_w: u32,
    stride_h: u32,
    stride_w: u32,
    padding_h: u32,
    padding_w: u32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_x = global_id.x;
    let out_y = global_id.y;
    let batch_channel = global_id.z;
    
    if (out_x >= params.output_width || out_y >= params.output_height) {
        return;
    }
    
    let batch_idx = batch_channel / params.channels;
    let channel_idx = batch_channel % params.channels;
    
    if (batch_idx >= params.batch_size) {
        return;
    }
    
    let in_y_start = out_y * params.stride_h;
    let in_x_start = out_x * params.stride_w;
    
    var sum: f32 = 0.0;
    var count: f32 = 0.0;
    
    for (var ky = 0u; ky < params.kernel_h; ky++) {
        for (var kx = 0u; kx < params.kernel_w; kx++) {
            let in_y = in_y_start + ky;
            let in_x = in_x_start + kx;
            
            if (in_y < params.padding_h || in_y >= (params.input_height + params.padding_h) ||
                in_x < params.padding_w || in_x >= (params.input_width + params.padding_w)) {
                continue;
            }
            
            let actual_y = in_y - params.padding_h;
            let actual_x = in_x - params.padding_w;
            
            let input_idx = batch_idx * (params.channels * params.input_height * params.input_width) +
                          channel_idx * (params.input_height * params.input_width) +
                          actual_y * params.input_width +
                          actual_x;
            
            sum += input[input_idx];
            count += 1.0;
        }
    }
    
    let output_idx = batch_idx * (params.channels * params.output_height * params.output_width) +
                    channel_idx * (params.output_height * params.output_width) +
                    out_y * params.output_width +
                    out_x;
    
    output[output_idx] = sum / count;
}
