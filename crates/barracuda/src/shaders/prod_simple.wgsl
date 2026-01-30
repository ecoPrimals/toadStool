// Product reduction - multiply all elements

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

var<workgroup> shared_prod: f32;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let size = arrayLength(&input);
    let tid = local_id.x;
    
    if (tid == 0u) {
        var product = 1.0;
        for (var i = 0u; i < size; i = i + 1u) {
            product = product * input[i];
        }
        shared_prod = product;
    }
    workgroupBarrier();
    
    if (tid == 0u) {
        output[0] = shared_prod;
    }
}
