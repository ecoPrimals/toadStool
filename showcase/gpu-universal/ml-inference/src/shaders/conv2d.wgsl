// Conv2D: Standard 2D Convolution
// CUDA equivalent: cudnnConvolutionForward
// The fundamental operation for CNNs (ResNet, VGG, YOLO, etc.)
//
// Features:
// - Configurable stride, padding, dilation
// - Bias addition
// - Runtime dimensions (Deep Debt principle)
//
// Use cases: Feature extraction in all major CNN architectures

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weights: array<f32>;
@group(0) @binding(2) var<storage, read> bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;

struct Conv2DParams {
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
    dilation_h: u32,
    dilation_w: u32,
    _pad: u32,  // Padding to 64 bytes
}

@group(0) @binding(4) var<uniform> params: Conv2DParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_x = global_id.x;
    let out_y = global_id.y;
    let out_c = global_id.z;
    
    // Bounds check
    if (out_x >= params.output_w || out_y >= params.output_h || out_c >= params.out_channels) {
        return;
    }
    
    var sum = 0.0;
    
    // Convolve across all input channels
    for (var in_c = 0u; in_c < params.in_channels; in_c++) {
        for (var ky = 0u; ky < params.kernel_h; ky++) {
            for (var kx = 0u; kx < params.kernel_w; kx++) {
                // Calculate input coordinates with stride, padding, and dilation
                let in_y_unpadded = out_y * params.stride_h + ky * params.dilation_h;
                let in_x_unpadded = out_x * params.stride_w + kx * params.dilation_w;
                
                // Check if we're in padded region
                if (in_y_unpadded < params.padding_h || in_x_unpadded < params.padding_w) {
                    continue; // Zero padding
                }
                
                let in_y = in_y_unpadded - params.padding_h;
                let in_x = in_x_unpadded - params.padding_w;
                
                // Check bounds (zero padding)
                if (in_y >= params.input_h || in_x >= params.input_w) {
                    continue;
                }
                
                // Input index: [batch, in_c, in_y, in_x]
                // For now, assume batch=0 (can be extended for batched processing)
                let input_idx = ((in_c * params.input_h + in_y) * params.input_w + in_x);
                
                // Weight index: [out_c, in_c, ky, kx]
                let weight_idx = (((out_c * params.in_channels + in_c) * params.kernel_h + ky) * params.kernel_w + kx);
                
                sum += input[input_idx] * weights[weight_idx];
            }
        }
    }
    
    // Add bias
    sum += bias[out_c];
    
    // Output index: [batch, out_c, out_y, out_x]
    let output_idx = ((out_c * params.output_h + out_y) * params.output_w + out_x);
    output[output_idx] = sum;
}

