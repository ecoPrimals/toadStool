// Grid Sample - Spatial transformer network sampling
// Samples input at arbitrary grid positions using bilinear interpolation

struct Params {
    batch_size: u32,
    channels: u32,
    in_height: u32,
    in_width: u32,
    out_height: u32,
    out_width: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> grid: array<f32>;  // [B, H_out, W_out, 2] normalized coords
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

// Bilinear interpolation helper
fn bilinear_sample(
    input_ptr: ptr<storage, array<f32>>,
    b: u32,
    c: u32,
    y: f32,
    x: f32,
    params_ref: ptr<uniform, Params>
) -> f32 {
    let h = (*params_ref).in_height;
    let w = (*params_ref).in_width;
    
    // Convert from normalized [-1, 1] to pixel coordinates
    let x_pix = (x + 1.0) * f32(w - 1u) * 0.5;
    let y_pix = (y + 1.0) * f32(h - 1u) * 0.5;
    
    // Get integer and fractional parts
    let x0 = u32(floor(x_pix));
    let y0 = u32(floor(y_pix));
    let x1 = min(x0 + 1u, w - 1u);
    let y1 = min(y0 + 1u, h - 1u);
    
    let wx = x_pix - f32(x0);
    let wy = y_pix - f32(y0);
    
    // Check bounds
    if (x0 >= w || y0 >= h) {
        return 0.0;
    }
    
    // Compute base index for this batch/channel
    let base_idx = (b * (*params_ref).channels + c) * h * w;
    
    // Sample four corners
    let v00 = (*input_ptr)[base_idx + y0 * w + x0];
    let v01 = (*input_ptr)[base_idx + y0 * w + x1];
    let v10 = (*input_ptr)[base_idx + y1 * w + x0];
    let v11 = (*input_ptr)[base_idx + y1 * w + x1];
    
    // Bilinear interpolation
    let v0 = v00 * (1.0 - wx) + v01 * wx;
    let v1 = v10 * (1.0 - wx) + v11 * wx;
    return v0 * (1.0 - wy) + v1 * wy;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_y = global_id.y;
    let out_x = global_id.x;
    
    if (out_y >= params.out_height || out_x >= params.out_width) {
        return;
    }
    
    // Process all batches and channels
    for (var b = 0u; b < params.batch_size; b = b + 1u) {
        // Get grid coordinates for this output position
        let grid_idx = ((b * params.out_height + out_y) * params.out_width + out_x) * 2u;
        let grid_x = grid[grid_idx];
        let grid_y = grid[grid_idx + 1u];
        
        for (var c = 0u; c < params.channels; c = c + 1u) {
            // Sample using bilinear interpolation
            let sampled_value = bilinear_sample(&input, b, c, grid_y, grid_x, &params);
            
            let out_idx = ((b * params.channels + c) * params.out_height + out_y) 
                          * params.out_width + out_x;
            output[out_idx] = sampled_value;
        }
    }
}
