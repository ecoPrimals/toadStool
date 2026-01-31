// Temporal pooling shader
// Aggregates spike activity over time windows

struct Params {
    n: u32,
    window_size: u32,
    num_windows: u32,
    padding: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn temporal_pool(@builtin(global_invocation_id) gid: vec3<u32>) {
    let window_idx = gid.x;
    if (window_idx >= params.num_windows) {
        return;
    }
    
    // Calculate window bounds
    let start = window_idx * params.window_size;
    let end = min(start + params.window_size, params.n);
    let window_len = end - start;
    
    // Sum spikes in window
    var sum = 0.0;
    for (var i = start; i < end; i = i + 1u) {
        sum = sum + input[i];
    }
    
    // Average firing rate
    output[window_idx] = sum / f32(window_len);
}
