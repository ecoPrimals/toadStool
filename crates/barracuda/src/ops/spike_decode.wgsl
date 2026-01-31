// Spike decoding shader
// Converts spike counts back to continuous values (0.0-1.0) using rate decoding

struct Params {
    n: u32,
    time_steps: u32,
    padding: vec2<u32>,
}

@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn spike_decode(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.n) {
        return;
    }
    
    // Rate decoding: spike count → continuous value
    // Spike count in [0, time_steps] maps to [0.0, 1.0]
    let spike_count = input[idx];
    
    // Convert to normalized value
    let value = f32(spike_count) / f32(params.time_steps);
    
    // Clamp to valid range (in case of overflow)
    output[idx] = clamp(value, 0.0, 1.0);
}
