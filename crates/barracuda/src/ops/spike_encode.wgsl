// Spike encoding shader
// Converts continuous values (0.0-1.0) into spike counts using rate coding

struct Params {
    n: u32,
    time_steps: u32,
    padding: vec2<u32>,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn spike_encode(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.n) {
        return;
    }
    
    // Rate coding: input value → spike frequency
    // Value in [0.0, 1.0] maps to [0, time_steps] spikes
    let value = input[idx];
    
    // Clamp to valid range
    let clamped = clamp(value, 0.0, 1.0);
    
    // Convert to spike count (rate coding)
    let spike_count = u32(clamped * f32(params.time_steps));
    
    output[idx] = spike_count;
}
