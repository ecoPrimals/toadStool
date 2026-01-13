// Transpose: Transpose matrix (rows, cols) -> (cols, rows)
// CUDA equivalent: cublas::geam
// Algorithm: Tiled transpose with coalesced memory access
// Use cases: Matrix operations, layout transforms, attention

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

struct Params {
    rows: u32,
    cols: u32,
}
@group(0) @binding(2) var<uniform> params: Params;

// Shared memory tile (16x16 with padding to avoid bank conflicts)
var<workgroup> tile: array<f32, 272>;  // 16x17 = 272 (extra column for padding)

fn tile_index(row: u32, col: u32) -> u32 {
    return row * 17u + col;  // 17 to avoid bank conflicts
}

@compute @workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let tx = local_id.x;
    let ty = local_id.y;
    
    // Input position (reading)
    let input_col = workgroup_id.x * 16u + tx;
    let input_row = workgroup_id.y * 16u + ty;
    
    // Load tile from input (coalesced)
    if (input_row < params.rows && input_col < params.cols) {
        let input_idx = input_row * params.cols + input_col;
        tile[tile_index(ty, tx)] = input[input_idx];
    }
    workgroupBarrier();
    
    // Output position (writing transposed)
    let output_col = workgroup_id.y * 16u + tx;
    let output_row = workgroup_id.x * 16u + ty;
    
    // Write tile to output (coalesced, transposed)
    if (output_row < params.cols && output_col < params.rows) {
        let output_idx = output_row * params.rows + output_col;
        output[output_idx] = tile[tile_index(tx, ty)];  // Note: tx and ty swapped
    }
}
