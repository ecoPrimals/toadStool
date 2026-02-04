// dilated_conv2d.wgsl - Dilated (atrous) 2D convolution
//
// Dilated convolution increases receptive field without increasing parameters
// by introducing gaps (dilation) between kernel elements.
//
// Use cases: Semantic segmentation, dense prediction tasks

struct Params {
    batch_size: u32,
    in_channels: u32,
    out_channels: u32,
    in_height: u32,
    in_width: u32,
    kernel_height: u32,
    kernel_width: u32,
    out_height: u32,
    out_width: u32,
    stride_h: u32,
    stride_w: u32,
    pad_h: u32,
    pad_w: u32,
    dilation_h: u32, // NEW: dilation factor for height
    dilation_w: u32, // NEW: dilation factor for width
    _padding: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;     // [B, C_in, H, W]
@group(0) @binding(1) var<storage, read> weight: array<f32>;    // [C_out, C_in, Kh, Kw]
@group(0) @binding(2) var<storage, read> bias: array<f32>;      // [C_out]
@group(0) @binding(3) var<storage, read_write> output: array<f32>; // [B, C_out, H_out, W_out]
@group(0) @binding(4) var<uniform> params: Params;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let b = global_id.z % params.batch_size;
    let c_out = global_id.z / params.batch_size;
    let oh = global_id.y;
    let ow = global_id.x;
    
    if (c_out >= params.out_channels || oh >= params.out_height || ow >= params.out_width) {
        return;
    }
    
    var sum: f32 = 0.0;
    
    // Dilated convolution: kernel elements are spaced by dilation factor
    for (var c_in: u32 = 0u; c_in < params.in_channels; c_in = c_in + 1u) {
        for (var kh: u32 = 0u; kh < params.kernel_height; kh = kh + 1u) {
            for (var kw: u32 = 0u; kw < params.kernel_width; kw = kw + 1u) {
                // Apply dilation: effective kernel position = kh * dilation_h
                let ih_raw = i32(oh * params.stride_h) - i32(params.pad_h) + i32(kh * params.dilation_h);
                let iw_raw = i32(ow * params.stride_w) - i32(params.pad_w) + i32(kw * params.dilation_w);
                
                // Bounds check
                if (ih_raw >= 0 && ih_raw < i32(params.in_height) && 
                    iw_raw >= 0 && iw_raw < i32(params.in_width)) {
                    
                    let ih = u32(ih_raw);
                    let iw = u32(iw_raw);
                    
                    // Input: [B, C_in, H, W]
                    let in_idx = b * params.in_channels * params.in_height * params.in_width +
                                c_in * params.in_height * params.in_width +
                                ih * params.in_width +
                                iw;
                    
                    // Weight: [C_out, C_in, Kh, Kw]
                    let w_idx = c_out * params.in_channels * params.kernel_height * params.kernel_width +
                               c_in * params.kernel_height * params.kernel_width +
                               kh * params.kernel_width +
                               kw;
                    
                    sum = sum + input[in_idx] * weight[w_idx];
                }
            }
        }
    }
    
    // Add bias
    sum = sum + bias[c_out];
    
    // Write output
    let out_idx = b * params.out_channels * params.out_height * params.out_width +
                  c_out * params.out_height * params.out_width +
                  oh * params.out_width +
                  ow;
    output[out_idx] = sum;
}
