// Pure Rust WGSL shader for matrix multiplication
// C = A * B where A is (M, K), B is (K, N), C is (M, N)

@group(0) @binding(0) var<storage, read> A: array<f32>;
@group(0) @binding(1) var<storage, read> B: array<f32>;
@group(0) @binding(2) var<storage, read_write> C: array<f32>;

struct MatmulParams {
    M: u32,
    K: u32,
    N: u32,
}

@group(0) @binding(3) var<uniform> params: MatmulParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let row = global_id.y;
    let col = global_id.x;
    
    if (row < params.M && col < params.N) {
        var sum = 0.0;
        for (var k = 0u; k < params.K; k = k + 1u) {
            sum = sum + A[row * params.K + k] * B[k * params.N + col];
        }
        C[row * params.N + col] = sum;
    }
}

