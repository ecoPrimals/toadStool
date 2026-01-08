// Pure Rust WGSL shader for 2D convolution
// Modern, type-safe, no FFI needed

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weights: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

struct Conv2DParams {
    in_channels: u32,
    out_channels: u32,
    input_h: u32,
    input_w: u32,
    kernel_h: u32,
    kernel_w: u32,
    output_h: u32,
    output_w: u32,
}

@group(0) @binding(3) var<uniform> params: Conv2DParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_y = global_id.y;
    let out_x = global_id.x;
    let out_c = global_id.z;
    
    if (out_y >= params.output_h || out_x >= params.output_w || out_c >= params.out_channels) {
        return;
    }
    
    var sum = 0.0;
    
    // Convolve across all input channels
    for (var in_c = 0u; in_c < params.in_channels; in_c = in_c + 1u) {
        for (var ky = 0u; ky < params.kernel_h; ky = ky + 1u) {
            for (var kx = 0u; kx < params.kernel_w; kx = kx + 1u) {
                let in_y = out_y + ky;
                let in_x = out_x + kx;
                
                let input_idx = ((in_c * params.input_h + in_y) * params.input_w + in_x);
                let weight_idx = (((out_c * params.in_channels + in_c) * params.kernel_h + ky) * params.kernel_w + kx);
                
                sum = sum + input[input_idx] * weights[weight_idx];
            }
        }
    }
    
    let output_idx = ((out_c * params.output_h + out_y) * params.output_w + out_x);
    output[output_idx] = sum;
}

