// Histc - Histogram with custom bins
// Computes histogram of input values into specified bins
// Uses atomic operations for parallel histogram computation
//
// Algorithm:
// 1. For each input value x:
// 2. Find bin index: bin = floor((x - min) / bin_width)
// 3. Atomically increment bin counter

struct Params {
    size: u32,        // Number of input elements
    num_bins: u32,    // Number of histogram bins
    min_val: f32,     // Minimum bin edge
    max_val: f32,     // Maximum bin edge
    bin_width: f32,   // (max - min) / num_bins
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input: array<f32>;
@group(0) @binding(2) var<storage, read_write> histogram: array<atomic<u32>>; // [num_bins]

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.size) {
        return;
    }

    let val = input[idx];
    
    // Check if value is within range
    if (val >= params.min_val && val < params.max_val) {
        // Compute bin index
        let bin_idx = u32(floor((val - params.min_val) / params.bin_width));
        
        // Clamp to valid range (handles edge case where val == max_val)
        let bin = min(bin_idx, params.num_bins - 1u);
        
        // Atomically increment histogram bin
        atomicAdd(&histogram[bin], 1u);
    }
    // Values outside range are ignored
}
