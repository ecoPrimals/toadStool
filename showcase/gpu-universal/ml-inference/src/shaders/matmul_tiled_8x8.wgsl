//! Tiled Matrix Multiplication - 8x8 tiles (optimized for production scales)
//!
//! **OPTIMIZATION**: Smaller tiles = less overhead, better for 512-2048 matrices
//!
//! Key differences from 16x16:
//!   - 8x8 tiles = 64 floats = 256 bytes (vs 1KB)
//!   - Fewer barrier synchronizations
//!   - Better for production transformer sizes
//!   - Less shared memory pressure

@group(0) @binding(0) var<storage, read> A: array<f32>;
@group(0) @binding(1) var<storage, read> B: array<f32>;
@group(0) @binding(2) var<storage, read_write> C: array<f32>;

struct MatmulParams {
    M: u32,
    K: u32,
    N: u32,
}

@group(0) @binding(3) var<uniform> params: MatmulParams;

// Smaller shared memory tiles (8x8 = 64 floats = 256 bytes per tile)
var<workgroup> tileA: array<f32, 64>;  // 8x8 tile of A
var<workgroup> tileB: array<f32, 64>;  // 8x8 tile of B

const TILE_SIZE: u32 = 8u;

@compute @workgroup_size(8, 8)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
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
        // Load A tile
        let a_col = tile * TILE_SIZE + local_col;
        if (a_col < params.K) {
            tileA[local_row * TILE_SIZE + local_col] = A[row * params.K + a_col];
        } else {
            tileA[local_row * TILE_SIZE + local_col] = 0.0;
        }
        
        // Load B tile
        let b_row = tile * TILE_SIZE + local_row;
        if (b_row < params.K) {
            tileB[local_row * TILE_SIZE + local_col] = B[b_row * params.N + col];
        } else {
            tileB[local_row * TILE_SIZE + local_col] = 0.0;
        }
        
        // Single barrier after loading
        workgroupBarrier();
        
        // Compute using shared memory
        for (var k = 0u; k < TILE_SIZE; k = k + 1u) {
            sum = sum + tileA[local_row * TILE_SIZE + k] * tileB[k * TILE_SIZE + local_col];
        }
        
        // Barrier before next tile
        workgroupBarrier();
    }
    
    // Write result
    C[row * params.N + col] = sum;
}
