// Conv2D - 2D convolution compute shader
//
// **Purpose**: GPU-accelerated 2D convolution for CNNs
// **Input**: [N, C_in, H, W] — batch, channels, height, width
// **Kernel**: [C_out, C_in, kH, kW]
// **Output**: [N, C_out, H_out, W_out]
//
// **Features**: Stride and padding via uniforms, workgroup tiling

struct Conv2DParams {
    n: u32,
    c_in: u32,
    h_in: u32,
    w_in: u32,
    c_out: u32,
    k_h: u32,
    k_w: u32,
    stride_h: u32,
    stride_w: u32,
    pad_h: u32,
    pad_w: u32,
    h_out: u32,
    w_out: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> kernel: array<f32>;
@group(0) @binding(2) var<storage, read> bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: Conv2DParams;

fn input_idx(n: u32, c: u32, h: u32, w: u32) -> u32 {
    return n * params.c_in * params.h_in * params.w_in
         + c * params.h_in * params.w_in
         + h * params.w_in
         + w;
}

fn kernel_idx(c_out: u32, c_in: u32, kh: u32, kw: u32) -> u32 {
    return c_out * params.c_in * params.k_h * params.k_w
         + c_in * params.k_h * params.k_w
         + kh * params.k_w
         + kw;
}

fn output_idx(n: u32, c: u32, h: u32, w: u32) -> u32 {
    return n * params.c_out * params.h_out * params.w_out
         + c * params.h_out * params.w_out
         + h * params.w_out
         + w;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let flat = gid.x;
    let total = params.n * params.c_out * params.h_out * params.w_out;
    if (flat >= total) { return; }

    let out_w = flat % params.w_out;
    let out_h = (flat / params.w_out) % params.h_out;
    let c_out = (flat / (params.w_out * params.h_out)) % params.c_out;
    let n = flat / (params.w_out * params.h_out * params.c_out);

    if (n >= params.n || c_out >= params.c_out || out_h >= params.h_out || out_w >= params.w_out) {
        return;
    }

    {
        let out_x = out_w;
        var sum = 0.0;

        for (var c_in = 0u; c_in < params.c_in; c_in = c_in + 1u) {
            for (var ky = 0u; ky < params.k_h; ky = ky + 1u) {
                for (var kx = 0u; kx < params.k_w; kx = kx + 1u) {
                    let in_h = i32(out_h) * i32(params.stride_h) + i32(ky) - i32(params.pad_h);
                    let in_w = i32(out_x) * i32(params.stride_w) + i32(kx) - i32(params.pad_w);

                    if (in_h >= 0 && in_h < i32(params.h_in) && in_w >= 0 && in_w < i32(params.w_in)) {
                        let in_idx = input_idx(
                            n,
                            c_in,
                            u32(in_h),
                            u32(in_w),
                        );
                        let k_idx = kernel_idx(c_out, c_in, ky, kx);
                        sum = sum + input[in_idx] * kernel[k_idx];
                    }
                }
            }
        }

        let out_idx = output_idx(n, c_out, out_h, out_x);
        let b = bias[c_out];
        output[out_idx] = sum + b;
    }
}
