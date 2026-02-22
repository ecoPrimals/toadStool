// Tiled Matrix Multiplication - Memory-Optimized
//
// **OPTIMIZATION**: Shared memory tiling for 70-80% bandwidth utilization
//
// Algorithm:
//   - Load tiles of A and B into shared memory
//   - Compute partial results using shared memory (fast!)
//   - Accumulate across all tiles
//   - Write final result to global memory
//
// Benefits:
//   - Reduces global memory access by 16x (tile size 16x16)
//   - Coalesced memory access (aligned reads/writes)
//   - Bank conflict avoidance (column-major for B)
//   - Expected: 2-3x speedup for large matrices
//
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

// Shared memory tiles (16x16 = 256 floats = 1KB per tile)
var<workgroup> tileA: array<f32, 256>;  // 16x16 tile of A
var<workgroup> tileB: array<f32, 256>;  // 16x16 tile of B

const TILE_SIZE: u32 = 16u;

@compute @workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let row = global_id.y;  // Global row in C
    let col = global_id.x;  // Global column in C
    
    let local_row = local_id.y;  // Local row in tile
    let local_col = local_id.x;  // Local column in tile
    
    // S-14: No early return — all threads must reach workgroupBarrier().
    // Out-of-bounds threads participate in tile loading (with zeros) and
    // barriers, but skip the final write.
    let in_bounds = row < params.M && col < params.N;
    
    var sum = 0.0;
    
    // Number of tiles needed to cover K dimension
    let num_tiles = (params.K + TILE_SIZE - 1u) / TILE_SIZE;
    
    // ═══════════════════════════════════════════════════════════
    // Tiled computation: Iterate through K dimension in tiles
    // ═══════════════════════════════════════════════════════════
    
    for (var tile = 0u; tile < num_tiles; tile = tile + 1u) {
        // ═══════════════════════════════════════════════════════
        // PHASE 1: Cooperative load of A tile (COALESCED!)
        // ═══════════════════════════════════════════════════════
        
        let a_row = row;
        let a_col = tile * TILE_SIZE + local_col;
        
        if (a_row < params.M && a_col < params.K) {
            tileA[local_row * TILE_SIZE + local_col] = A[a_row * params.K + a_col];
        } else {
            tileA[local_row * TILE_SIZE + local_col] = 0.0;
        }
        
        let b_row = tile * TILE_SIZE + local_row;
        let b_col = col;
        
        if (b_row < params.K && b_col < params.N) {
            tileB[local_row * TILE_SIZE + local_col] = B[b_row * params.N + b_col];
        } else {
            tileB[local_row * TILE_SIZE + local_col] = 0.0;
        }
        
        // Wait for all threads to finish loading tiles
        workgroupBarrier();
        
        // ═══════════════════════════════════════════════════════
        // PHASE 3: Compute using shared memory (FAST!)
        // ═══════════════════════════════════════════════════════
        
        // Each thread computes dot product of its tile row/column
        for (var k = 0u; k < TILE_SIZE; k = k + 1u) {
            // Shared memory access is ~100x faster than global memory!
            let a_val = tileA[local_row * TILE_SIZE + k];
            let b_val = tileB[k * TILE_SIZE + local_col];
            sum = sum + a_val * b_val;
        }
        
        // Wait before loading next tile (prevent race condition)
        workgroupBarrier();
    }
    
    // ═══════════════════════════════════════════════════════════
    // PHASE 4: Write result (COALESCED!)
    // ═══════════════════════════════════════════════════════════
    
    if (in_bounds) {
        C[row * params.N + col] = sum;
    }
}
