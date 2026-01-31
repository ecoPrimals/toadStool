// GC content calculation shader
// Counts G and C nucleotides using atomic operations

struct Params {
    n: u32,
}

@group(0) @binding(0) var<storage, read> sequence: array<u32>;
@group(0) @binding(1) var<storage, read_write> gc_count: atomic<u32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn gc_content(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.n) {
        return;
    }
    
    let base = sequence[idx];
    
    // Check if base is G (71) or C (67) in ASCII
    // Also accept lowercase g (103) and c (99)
    if (base == 71u || base == 67u || base == 103u || base == 99u) {
        atomicAdd(&gc_count, 1u);
    }
}
