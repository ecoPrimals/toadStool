// One-hot encoding
// Convert indices to one-hot vectors

struct OneHotParams {
    num_classes: u32,
    _padding: vec3<u32>,
    _padding2: vec4<u32>,
    _padding3: vec4<u32>,
    _padding4: vec4<u32>, // Total 64 bytes
}

@group(0) @binding(0) var<storage, read> indices: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: OneHotParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let num_indices = arrayLength(&indices);
    
    if (idx >= num_indices * params.num_classes) {
        return;
    }
    
    let batch_idx = idx / params.num_classes;
    let class_idx = idx % params.num_classes;
    
    if (batch_idx < num_indices) {
        if (class_idx == indices[batch_idx]) {
            output[idx] = 1.0;
        } else {
            output[idx] = 0.0;
        }
    }
}
