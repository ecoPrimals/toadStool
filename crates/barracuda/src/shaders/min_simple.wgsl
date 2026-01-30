// Min reduction - Find minimum value

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

var<workgroup> shared_min: f32;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let size = arrayLength(&input);
    let tid = local_id.x;
    
    if (tid == 0u) {
        var min_val = input[0];
        for (var i = 1u; i < size; i = i + 1u) {
            min_val = min(min_val, input[i]);
        }
        shared_min = min_val;
    }
    workgroupBarrier();
    
    if (tid == 0u) {
        output[0] = shared_min;
    }
}
