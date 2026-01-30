// Simple Softmax for small tensors (single workgroup)
// Formula: softmax(x_i) = exp(x_i - max(x)) / sum(exp(x_j - max(x)))

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

var<workgroup> shared_max: f32;
var<workgroup> shared_sum: f32;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let size = arrayLength(&input);
    let idx = global_id.x;
    let tid = local_id.x;
    
    // Phase 1: Find max (numerically stable softmax)
    if (tid == 0u) {
        var max_val = input[0];
        for (var i = 1u; i < size; i = i + 1u) {
            max_val = max(max_val, input[i]);
        }
        shared_max = max_val;
    }
    workgroupBarrier();
    
    // Phase 2: Compute exp and sum
    if (tid == 0u) {
        var sum = 0.0;
        for (var i = 0u; i < size; i = i + 1u) {
            sum = sum + exp(input[i] - shared_max);
        }
        shared_sum = sum;
    }
    workgroupBarrier();
    
    // Phase 3: Normalize
    if (idx < size) {
        output[idx] = exp(input[idx] - shared_max) / shared_sum;
    }
}
