// Trace - Sum of diagonal elements (complete parallel reduction)
// Computes sum of diagonal elements: tr(A) = Σ A[i,i]
//
// Algorithm:
// 1. Parallel reduction of diagonal elements
// 2. Sum accumulation via atomic operations

struct Params {
    size: u32,       // Matrix size (n×n)
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> matrix: array<f32>;         // [size, size]
@group(0) @binding(2) var<storage, read_write> trace_buffer: array<f32>; // [1] - output

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.size) {
        return;
    }

    // Read diagonal element
    let diag_val = matrix[idx * params.size + idx];
    
    // Atomic accumulation
    atomicAdd(&trace_buffer[0], bitcast<i32>(diag_val));
}
