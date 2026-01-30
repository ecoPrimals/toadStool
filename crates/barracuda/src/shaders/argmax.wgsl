// Argmax - find index of maximum value
// Simplified version: find max in 1D tensor

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

var<workgroup> shared_max: f32;
var<workgroup> shared_idx: u32;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let size = arrayLength(&input);
    let tid = local_id.x;
    
    if (tid == 0u) {
        var max_val = input[0];
        var max_idx = 0u;
        
        for (var i = 1u; i < size; i = i + 1u) {
            if (input[i] > max_val) {
                max_val = input[i];
                max_idx = i;
            }
        }
        
        shared_max = max_val;
        shared_idx = max_idx;
    }
    workgroupBarrier();
    
    if (tid == 0u) {
        output[0] = shared_idx;
    }
}
