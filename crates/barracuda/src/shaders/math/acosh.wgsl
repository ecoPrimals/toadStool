// Inverse hyperbolic cosine operation
// acosh(x) = ln(x + sqrt(x² - 1))
// Defined for x >= 1

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> metadata: Metadata;

struct Metadata {
    size: u32,
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= metadata.size) {
        return;
    }
    
    let x = input[idx];
    
    // acosh is only defined for x >= 1
    // WGSL acosh() returns NaN for x < 1
    output[idx] = acosh(x);
}
