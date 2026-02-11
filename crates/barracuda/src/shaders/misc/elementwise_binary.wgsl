// ElementwiseBinary: C = A op B (Add, Sub, Mul, Div)
// CUDA equivalent: thrust::transform (binary)
// Use cases: Residual connections, loss computation

@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

struct Params {
    size: u32,
    operation: u32,  // 0=Add, 1=Sub, 2=Mul, 3=Div
}
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.size) {
        return;
    }
    
    let val_a = a[idx];
    let val_b = b[idx];
    
    var result: f32;
    switch (params.operation) {
        case 0u: { // Add
            result = val_a + val_b;
        }
        case 1u: { // Sub
            result = val_a - val_b;
        }
        case 2u: { // Mul
            result = val_a * val_b;
        }
        case 3u: { // Div
            result = val_a / val_b;
        }
        default: {
            result = 0.0;
        }
    }
    
    output[idx] = result;
}
