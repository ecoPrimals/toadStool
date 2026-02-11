// Where/Select - conditional selection
// output[i] = condition[i] ? x[i] : y[i]

@group(0) @binding(0) var<storage, read> condition: array<f32>;
@group(0) @binding(1) var<storage, read> x: array<f32>;
@group(0) @binding(2) var<storage, read> y: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= arrayLength(&output)) {
        return;
    }
    
    // Condition > 0.0 means true
    if (condition[idx] > 0.0) {
        output[idx] = x[idx];
    } else {
        output[idx] = y[idx];
    }
}
