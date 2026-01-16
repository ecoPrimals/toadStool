//! Tiled Matrix Multiplication - 32x32 tiles (optimized for extreme scales)
//!
//! **OPTIMIZATION**: Larger tiles = better for 4096+ matrices
//!
//! Key differences from 16x16:
//!   - 32x32 tiles = 1024 floats = 4KB per tile
//!   - More data reuse in shared memory
//!   - Better for very large matrices (4096+)
//!   - Higher memory bandwidth utilization

@group(0) @binding(0) var<storage, read> A: array<f32>;
@group(0) @binding(1) var<storage, read> B: array<f32>;
@group(0) @binding(2) var<storage, read_write> C: array<f32>;

struct MatmulParams {
    M: u32,
    K: u32,
    N: u32,
}

@group(0) @binding(3) var<uniform> params: MatmulParams;

// Larger shared memory tiles (32x32 = 1024 floats = 4KB per tile)
var<workgroup> tileA: array<f32, 1024>;  // 32x32 tile of A
var<workgroup> tileB: array<f32, 1024>;  // 32x32 tile of B

const TILE_SIZE: u32 = 32u;

@compute @workgroup_size(16, 16)  // Note: workgroup is 16x16, processes 32x32 tile
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let row = global_id.y;
    let col = global_id.x;
    
    let local_row = local_id.y;
    let local_col = local_id.x;
    
    // Early bounds check
    if (row >= params.M || col >= params.N) {
        return;
    }
    
    var sum = 0.0;
    let num_tiles = (params.K + TILE_SIZE - 1u) / TILE_SIZE;
    
    // Tiled computation
    for (var tile = 0u; tile < num_tiles; tile = tile + 1u) {
        // Each thread loads 4 elements (2x2 block) to fill 32x32 tile with 16x16 threads
        // Load A tile elements
        for (var i = 0u; i < 2u; i = i + 1u) {
            for (var j = 0u; j < 2u; j = j + 1u) {
                let a_row = row + i * 16u;
                let a_col = tile * TILE_SIZE + local_col + j * 16u;
                let tile_row = local_row + i * 16u;
                let tile_col = local_col + j * 16u;
                
                if (a_row < params.M && a_col < params.K) {
                    tileA[tile_row * TILE_SIZE + tile_col] = A[a_row * params.K + a_col];
                } else {
                    tileA[tile_row * TILE_SIZE + tile_col] = 0.0;
                }
            }
        }
        
        // Load B tile elements
        for (var i = 0u; i < 2u; i = i + 1u) {
            for (var j = 0u; j < 2u; j = j + 1u) {
                let b_row = tile * TILE_SIZE + local_row + i * 16u;
                let b_col = col + j * 16u;
                let tile_row = local_row + i * 16u;
                let tile_col = local_col + j * 16u;
                
                if (b_row < params.K && b_col < params.N) {
                    tileB[tile_row * TILE_SIZE + tile_col] = B[b_row * params.N + b_col];
                } else {
                    tileB[tile_row * TILE_SIZE + tile_col] = 0.0;
                }
            }
        }
        
        workgroupBarrier();
        
        // Compute using shared memory
        for (var k = 0u; k < TILE_SIZE; k = k + 1u) {
            sum = sum + tileA[local_row * TILE_SIZE + k] * tileB[k * TILE_SIZE + local_col];
        }
        
        workgroupBarrier();
    }
    
    // Write result
    C[row * params.N + col] = sum;
}
