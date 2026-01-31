// Complexity filter shader
// Identifies low-complexity regions by counting unique bases in sliding windows

struct Params {
    n: u32,
    window_size: u32,
    min_unique: u32,
}

@group(0) @binding(0) var<storage, read> sequence: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn complexity_filter(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = gid.x;
    if (pos >= params.n) {
        return;
    }
    
    // Check if window fits at this position
    if (pos + params.window_size > params.n) {
        output[pos] = 0.0;  // Not enough sequence left for window
        return;
    }
    
    // Count unique bases in window using simple array
    // Track which ASCII codes we've seen (simplified to A=65, T=84, G=71, C=67)
    var seen_A = false;
    var seen_T = false;
    var seen_G = false;
    var seen_C = false;
    
    for (var i = 0u; i < params.window_size; i = i + 1u) {
        let base = sequence[pos + i];
        if (base == 65u || base == 97u) { seen_A = true; }      // A or a
        else if (base == 84u || base == 116u) { seen_T = true; } // T or t
        else if (base == 71u || base == 103u) { seen_G = true; } // G or g
        else if (base == 67u || base == 99u) { seen_C = true; }  // C or c
    }
    
    var unique_count = 0u;
    if (seen_A) { unique_count = unique_count + 1u; }
    if (seen_T) { unique_count = unique_count + 1u; }
    if (seen_G) { unique_count = unique_count + 1u; }
    if (seen_C) { unique_count = unique_count + 1u; }
    
    // Flag as low complexity if unique count below threshold
    output[pos] = select(0.0, 1.0, unique_count < params.min_unique);
}
