// ROI Align - Region of Interest Align (He et al., Mask R-CNN)
// Extracts fixed-size feature maps from regions using bilinear interpolation
// Avoids quantization artifacts of ROI Pooling
//
// Algorithm:
// 1. For each ROI: (x1, y1, x2, y2)
// 2. Divide ROI into pooled_height x pooled_width bins
// 3. Sample points within each bin (sampling_ratio)
// 4. Bilinearly interpolate features at each sample point
// 5. Average interpolated values within each bin

struct Params {
    num_rois: u32,
    channels: u32,
    height: u32,
    width: u32,
    pooled_height: u32,
    pooled_width: u32,
    spatial_scale: f32,  // Scale factor from input image to feature map
    sampling_ratio: u32, // Number of sampling points per bin (typically 2)
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> features: array<f32>;  // [1, C, H, W]
@group(0) @binding(2) var<storage, read> rois: array<f32>;      // [num_rois, 4] (x1, y1, x2, y2)
@group(0) @binding(3) var<storage, read_write> output: array<f32>; // [num_rois, C, pooled_H, pooled_W]

fn bilinear_interpolate(x: f32, y: f32, c: u32) -> f32 {
    // Clamp to valid range
    let x_clamped = clamp(x, 0.0, f32(params.width - 1));
    let y_clamped = clamp(y, 0.0, f32(params.height - 1));
    
    // Get integer and fractional parts
    let x0 = u32(floor(x_clamped));
    let y0 = u32(floor(y_clamped));
    let x1 = min(x0 + 1u, params.width - 1u);
    let y1 = min(y0 + 1u, params.height - 1u);
    
    let fx = x_clamped - f32(x0);
    let fy = y_clamped - f32(y0);
    
    // Fetch corner values
    let idx_00 = (c * params.height + y0) * params.width + x0;
    let idx_01 = (c * params.height + y0) * params.width + x1;
    let idx_10 = (c * params.height + y1) * params.width + x0;
    let idx_11 = (c * params.height + y1) * params.width + x1;
    
    let v00 = features[idx_00];
    let v01 = features[idx_01];
    let v10 = features[idx_10];
    let v11 = features[idx_11];
    
    // Bilinear interpolation
    let v0 = v00 * (1.0 - fx) + v01 * fx;
    let v1 = v10 * (1.0 - fx) + v11 * fx;
    return v0 * (1.0 - fy) + v1 * fy;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let roi_idx = global_id.z;
    let c = global_id.y;
    let bin_idx = global_id.x;
    
    if (roi_idx >= params.num_rois || c >= params.channels || 
        bin_idx >= params.pooled_height * params.pooled_width) {
        return;
    }
    
    let ph = bin_idx / params.pooled_width;
    let pw = bin_idx % params.pooled_width;
    
    // Get ROI bounds (scaled to feature map coordinates)
    let roi_offset = roi_idx * 4u;
    let x1 = rois[roi_offset] * params.spatial_scale;
    let y1 = rois[roi_offset + 1u] * params.spatial_scale;
    let x2 = rois[roi_offset + 2u] * params.spatial_scale;
    let y2 = rois[roi_offset + 3u] * params.spatial_scale;
    
    let roi_width = x2 - x1;
    let roi_height = y2 - y1;
    
    // Bin size in feature map coordinates
    let bin_width = roi_width / f32(params.pooled_width);
    let bin_height = roi_height / f32(params.pooled_height);
    
    // Sample points within this bin
    let num_samples = params.sampling_ratio * params.sampling_ratio;
    var sum = 0.0;
    
    for (var iy = 0u; iy < params.sampling_ratio; iy++) {
        for (var ix = 0u; ix < params.sampling_ratio; ix++) {
            // Sample position within bin
            let sample_x = x1 + (f32(pw) + (f32(ix) + 0.5) / f32(params.sampling_ratio)) * bin_width;
            let sample_y = y1 + (f32(ph) + (f32(iy) + 0.5) / f32(params.sampling_ratio)) * bin_height;
            
            // Bilinearly interpolate
            sum += bilinear_interpolate(sample_x, sample_y, c);
        }
    }
    
    // Average over samples
    let avg = sum / f32(num_samples);
    
    let out_idx = ((roi_idx * params.channels + c) * params.pooled_height + ph) * params.pooled_width + pw;
    output[out_idx] = avg;
}
