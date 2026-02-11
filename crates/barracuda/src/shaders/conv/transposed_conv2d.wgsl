// TransposedConv2D (Deconvolution) - Upsampling operation
// Also known as fractionally-strided convolution
// Used in U-Net decoder, image super-resolution, GANs

struct TransposedConv2DParams {
    batch_size: u32,
    in_channels: u32,
    out_channels: u32,
    input_h: u32,
    input_w: u32,
    output_h: u32,
    output_w: u32,
    kernel_h: u32,
    kernel_w: u32,
    stride_h: u32,
    stride_w: u32,
    padding_h: u32,
    padding_w: u32,
    output_padding_h: u32,
    output_padding_w: u32,
    _pad: u32, // Padding to 64 bytes
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weights: array<f32>;
@group(0) @binding(2) var<storage, read> bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: TransposedConv2DParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_c = global_id.z;
    let out_y = global_id.y;
    let out_x = global_id.x;
    
    // Check bounds
    if (out_c >= params.out_channels || out_y >= params.output_h || out_x >= params.output_w) {
        return;
    }
    
    var sum = 0.0;
    
    // For each batch
    for (var b = 0u; b < params.batch_size; b = b + 1u) {
        // For each input channel
        for (var in_c = 0u; in_c < params.in_channels; in_c = in_c + 1u) {
            // For each kernel position
            for (var kh = 0u; kh < params.kernel_h; kh = kh + 1u) {
                for (var kw = 0u; kw < params.kernel_w; kw = kw + 1u) {
                    // Calculate which input position contributes to this output
                    // Transposed conv: output[i] comes from input[i/stride] if aligned
                    
                    // Check if this output position is affected by this kernel position
                    let out_y_offset = i32(out_y) + i32(params.padding_h) - i32(kh);
                    let out_x_offset = i32(out_x) + i32(params.padding_w) - i32(kw);
                    
                    // Check if aligned with stride
                    if (out_y_offset % i32(params.stride_h) == 0 && out_x_offset % i32(params.stride_w) == 0) {
                        let in_y = out_y_offset / i32(params.stride_h);
                        let in_x = out_x_offset / i32(params.stride_w);
                        
                        // Check if input position is valid
                        if (in_y >= 0 && in_y < i32(params.input_h) && 
                            in_x >= 0 && in_x < i32(params.input_w)) {
                            
                            // Calculate input index
                            let input_idx = b * params.in_channels * params.input_h * params.input_w +
                                          in_c * params.input_h * params.input_w +
                                          u32(in_y) * params.input_w +
                                          u32(in_x);
                            
                            // Calculate weight index
                            // Weights: [in_channels, out_channels, kernel_h, kernel_w]
                            let weight_idx = in_c * params.out_channels * params.kernel_h * params.kernel_w +
                                           out_c * params.kernel_h * params.kernel_w +
                                           kh * params.kernel_w +
                                           kw;
                            
                            sum += input[input_idx] * weights[weight_idx];
                        }
                    }
                }
            }
        }
    }
    
    // Add bias
    sum += bias[out_c];
    
    // Write output
    let output_idx = out_c * params.output_h * params.output_w +
                     out_y * params.output_w +
                     out_x;
    output[output_idx] = sum;
}
