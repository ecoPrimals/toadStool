// Pure Rust WGSL shader for ReLU activation
// No FFI, no unsafe - just modern idiomatic Rust + WebGPU!

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    
    if (i < arrayLength(&input)) {
        // ReLU: max(0, x)
        output[i] = max(0.0, input[i]);
    }
}

