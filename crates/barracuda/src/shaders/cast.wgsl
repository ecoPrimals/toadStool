// Cast - Type conversion operation
// Simplified: f32 identity (type handled by tensor metadata)
// In production, would support multiple types

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= arrayLength(&input)) {
        return;
    }
    
    // For f32->f32, this is identity
    // In production, would have type conversion logic
    output[idx] = input[idx];
}
