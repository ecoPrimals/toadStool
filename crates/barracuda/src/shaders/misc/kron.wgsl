// Kronecker Product - Tensor product of matrices (complete implementation)
// Computes Kronecker product: C[i*m+k, j*n+l] = A[i,j] * B[k,l]
//
// Example: kron([[1,2],[3,4]], [[5,6],[7,8]]) → [[5,6,10,12],[7,8,14,16],[15,18,20,24],[21,24,28,32]]
//
// Algorithm:
// Each thread computes one element of the output matrix

struct Params {
    a_rows: u32,
    a_cols: u32,
    b_rows: u32,
    b_cols: u32,
    out_rows: u32,    // a_rows * b_rows
    out_cols: u32,    // a_cols * b_cols
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> matrix_a: array<f32>;
@group(0) @binding(2) var<storage, read> matrix_b: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let out_row = global_id.y;
    let out_col = global_id.x;
    
    if (out_row >= params.out_rows || out_col >= params.out_cols) {
        return;
    }

    // Decompose output position into A and B indices
    let a_row = out_row / params.b_rows;
    let b_row = out_row % params.b_rows;
    let a_col = out_col / params.b_cols;
    let b_col = out_col % params.b_cols;
    
    // Compute Kronecker product element
    let a_val = matrix_a[a_row * params.a_cols + a_col];
    let b_val = matrix_b[b_row * params.b_cols + b_col];
    
    output[out_row * params.out_cols + out_col] = a_val * b_val;
}
