// Clamp values to range [0, 6] (ReLU6-style for simplicity)
// Can be extended with parameters for arbitrary ranges

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

const MIN_VAL: f32 = 0.0;
const MAX_VAL: f32 = 6.0;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= arrayLength(&input)) {
        return;
    }
    
    let x = input[idx];
    output[idx] = clamp(x, MIN_VAL, MAX_VAL);
}
