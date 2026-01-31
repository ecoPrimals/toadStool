// Pattern matching shader
// Naive string matching algorithm

struct Params {
    target_len: u32,
    pattern_len: u32,
}

@group(0) @binding(0) var<storage, read> target_seq: array<u32>;
@group(0) @binding(1) var<storage, read> pattern: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256)
fn pattern_match(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pos = gid.x;
    if (pos >= params.target_len) {
        return;
    }
    
    // Check if pattern can fit at this position
    if (pos + params.pattern_len > params.target_len) {
        output[pos] = 0.0;
        return;
    }
    
    // Compare pattern byte by byte
    var matches = true;
    for (var i = 0u; i < params.pattern_len; i = i + 1u) {
        if (target_seq[pos + i] != pattern[i]) {
            matches = false;
            break;
        }
    }
    
    output[pos] = select(0.0, 1.0, matches);
}
