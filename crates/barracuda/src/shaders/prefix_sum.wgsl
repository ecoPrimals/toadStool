// Prefix Sum - Inclusive scan (GPU parallel scan for masked_select/nonzero)
// Computes running sum: output[i] = sum(input[0..=i])
//
// Algorithm: Blelloch scan (work-efficient parallel prefix sum)
// 1. Up-sweep: Build reduction tree
// 2. Down-sweep: Propagate sums
//
// Note: This is a simplified single-pass version.
// Production implementation would use hierarchical scan for large inputs.

struct Params {
    size: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<u32>;
@group(0) @binding(3) var<storage, read_write> scratch: array<u32>;  // Working buffer

// Simple inclusive scan (sequential for now, can be parallelized)
@compute @workgroup_size(1)
fn inclusive_scan(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var sum = 0u;
    for (var i = 0u; i < params.size; i++) {
        sum += input[i];
        output[i] = sum;
    }
}
