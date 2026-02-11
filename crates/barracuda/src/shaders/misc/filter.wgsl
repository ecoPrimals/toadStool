// Filter: Stream compaction with predicate
// CUDA equivalent: thrust::copy_if, cub::DeviceSelect
// Algorithm: Predicate evaluation + prefix sum + scatter
// Use cases: Sparse operations, conditional selection

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> flags: array<u32>;  // 1 if keep, 0 if discard
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<storage, read_write> count: array<atomic<u32>>;  // Output count

struct Params {
    size: u32,
    operation: u32,  // 0=GreaterThan, 1=LessThan, 2=Equal, 3=NotEqual
    threshold: f32,
}
@group(0) @binding(4) var<uniform> params: Params;

// Pass 1: Evaluate predicate
@compute @workgroup_size(256)
fn evaluate_predicate(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;
    
    if (gid >= params.size) {
        return;
    }
    
    let value = input[gid];
    var keep: bool = false;
    
    switch (params.operation) {
        case 0u: { // GreaterThan
            keep = value > params.threshold;
        }
        case 1u: { // LessThan
            keep = value < params.threshold;
        }
        case 2u: { // Equal
            keep = abs(value - params.threshold) < 0.0001;
        }
        case 3u: { // NotEqual
            keep = abs(value - params.threshold) >= 0.0001;
        }
        default: {}
    }
    
    flags[gid] = select(0u, 1u, keep);
}

// Pass 2: Compact (requires prefix sum on flags first, done separately)
@compute @workgroup_size(256)
fn compact(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let gid = global_id.x;
    
    if (gid >= params.size) {
        return;
    }
    
    if (flags[gid] == 1u) {
        // flags contains prefix sum after scan pass
        // This would need the scanned indices
        let output_idx = atomicAdd(&count[0], 1u);
        output[output_idx] = input[gid];
    }
}
