// matmul_cpu_tiled.wgsl — Double-buffered 32×32 tiled matmul for CPU (llvmpipe)
//
// Absorbed from neuralSpring local evolutions (neuralSpring handoff #11).
//
// Key optimisations vs the naive 16×16 tiled shader:
//   1. TILE = 32 → 4× more arithmetic reuse per global-memory load;
//      two 32×32 f32 tiles ≈ 8 KB, fits in typical L1 data cache.
//   2. Double-buffered tiles — while computing tileX_A / tileX_B, the
//      same invocation preloads the NEXT tile into tileY_A / tileY_B.
//      On llvmpipe this keeps the LLVM vectoriser fed without stalls.
//   3. 2×2 micro-kernel — each thread produces four output elements,
//      doubling arithmetic intensity vs one element per thread.
//   4. 4× k-loop unroll — four independent FMA chains fill the latency
//      window for llvmpipe's software scheduler (→ fmuladd IR).
//   5. fma() intrinsic — WGSL fma() lowers to LLVM fmuladd, which the
//      x86 backend can map to vfmadd instructions.
//
// C = A × B,  A:[M,K], B:[K,N], C:[M,N]

const TILE: u32 = 32u;

struct MatMulParams {
    m: u32,
    k: u32,
    n: u32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read>       A:      array<f32>;
@group(0) @binding(1) var<storage, read>       B:      array<f32>;
@group(0) @binding(2) var<storage, read_write> C:      array<f32>;
@group(0) @binding(3) var<uniform>             params: MatMulParams;

// Two pairs of shared-memory tiles for double buffering.
// Each tile holds 32×32 f32 = 4 KB.  Total workgroup memory: 4 × 4 KB = 16 KB.
var<workgroup> tileA_curr: array<f32, 1024>;   // current A tile
var<workgroup> tileB_curr: array<f32, 1024>;   // current B tile
var<workgroup> tileA_next: array<f32, 1024>;   // prefetch A tile
var<workgroup> tileB_next: array<f32, 1024>;   // prefetch B tile

// 16×16 workgroup, each thread computes a 2×2 output block → 32×32 output tile.
@compute @workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) global_id:    vec3<u32>,
    @builtin(local_invocation_id)  local_id:     vec3<u32>,
    @builtin(workgroup_id)         workgroup_id: vec3<u32>,
) {
    // Each thread is responsible for a 2×2 output block.
    let out_row = workgroup_id.y * TILE + local_id.y * 2u;
    let out_col = workgroup_id.x * TILE + local_id.x * 2u;

    let lrow = local_id.y;
    let lcol = local_id.x;

    // 4-element accumulators for the 2×2 output block:
    //   acc_rr[r][c]  where r,c ∈ {0,1}
    var acc00 = 0.0f;
    var acc01 = 0.0f;
    var acc10 = 0.0f;
    var acc11 = 0.0f;

    let num_tiles = (params.k + TILE - 1u) / TILE;

    // ── Pre-load tile 0 into "current" buffers ────────────────────────────
    // Each 16×16 workgroup cooperatively loads the 32×32 tile:
    // every invocation loads 4 elements (2 rows × 2 cols).
    for (var dr = 0u; dr < 2u; dr = dr + 1u) {
        for (var dc = 0u; dc < 2u; dc = dc + 1u) {
            let srow = lrow * 2u + dr;
            let scol = lcol * 2u + dc;

            let a_row = workgroup_id.y * TILE + srow;
            let a_col = scol;   // tile 0: col offset = 0
            if (a_row < params.m && a_col < params.k) {
                tileA_curr[srow * TILE + scol] = A[a_row * params.k + a_col];
            } else {
                tileA_curr[srow * TILE + scol] = 0.0f;
            }

            let b_row = srow;   // tile 0: row offset = 0
            let b_col = workgroup_id.x * TILE + scol;
            if (b_row < params.k && b_col < params.n) {
                tileB_curr[srow * TILE + scol] = B[b_row * params.n + b_col];
            } else {
                tileB_curr[srow * TILE + scol] = 0.0f;
            }
        }
    }
    workgroupBarrier();

    // ── Double-buffered tile loop ─────────────────────────────────────────
    for (var t = 0u; t < num_tiles; t = t + 1u) {

        // Prefetch next tile (if it exists) into "next" buffers while we
        // compute with "current".  On CPU this keeps the load pipeline busy.
        let next_t = t + 1u;
        if (next_t < num_tiles) {
            for (var dr = 0u; dr < 2u; dr = dr + 1u) {
                for (var dc = 0u; dc < 2u; dc = dc + 1u) {
                    let srow = lrow * 2u + dr;
                    let scol = lcol * 2u + dc;

                    let a_row = workgroup_id.y * TILE + srow;
                    let a_col = next_t * TILE + scol;
                    if (a_row < params.m && a_col < params.k) {
                        tileA_next[srow * TILE + scol] = A[a_row * params.k + a_col];
                    } else {
                        tileA_next[srow * TILE + scol] = 0.0f;
                    }

                    let b_row = next_t * TILE + srow;
                    let b_col = workgroup_id.x * TILE + scol;
                    if (b_row < params.k && b_col < params.n) {
                        tileB_next[srow * TILE + scol] = B[b_row * params.n + b_col];
                    } else {
                        tileB_next[srow * TILE + scol] = 0.0f;
                    }
                }
            }
        }

        // ── Compute 2×2 block from current tile (4× k-unrolled) ──────────
        // Unrolling by 4 produces 4 independent FMA chains — each chain can
        // be scheduled independently by llvmpipe's ILP scheduler.
        let k_limit = min(TILE, params.k - t * TILE);
        var k = 0u;

        // Unrolled body: 4 steps at a time
        for (; k + 4u <= k_limit; k = k + 4u) {
            let a0_r0 = tileA_curr[lrow * 2u       * TILE + k];
            let a1_r0 = tileA_curr[lrow * 2u       * TILE + k + 1u];
            let a2_r0 = tileA_curr[lrow * 2u       * TILE + k + 2u];
            let a3_r0 = tileA_curr[lrow * 2u       * TILE + k + 3u];
            let a0_r1 = tileA_curr[(lrow * 2u + 1u) * TILE + k];
            let a1_r1 = tileA_curr[(lrow * 2u + 1u) * TILE + k + 1u];
            let a2_r1 = tileA_curr[(lrow * 2u + 1u) * TILE + k + 2u];
            let a3_r1 = tileA_curr[(lrow * 2u + 1u) * TILE + k + 3u];

            let b0_c0 = tileB_curr[k       * TILE + lcol * 2u];
            let b1_c0 = tileB_curr[(k + 1u) * TILE + lcol * 2u];
            let b2_c0 = tileB_curr[(k + 2u) * TILE + lcol * 2u];
            let b3_c0 = tileB_curr[(k + 3u) * TILE + lcol * 2u];
            let b0_c1 = tileB_curr[k       * TILE + lcol * 2u + 1u];
            let b1_c1 = tileB_curr[(k + 1u) * TILE + lcol * 2u + 1u];
            let b2_c1 = tileB_curr[(k + 2u) * TILE + lcol * 2u + 1u];
            let b3_c1 = tileB_curr[(k + 3u) * TILE + lcol * 2u + 1u];

            // fma() → LLVM fmuladd → vfmadd (AVX-512 on x86)
            acc00 = fma(a0_r0, b0_c0, fma(a1_r0, b1_c0, fma(a2_r0, b2_c0, fma(a3_r0, b3_c0, acc00))));
            acc01 = fma(a0_r0, b0_c1, fma(a1_r0, b1_c1, fma(a2_r0, b2_c1, fma(a3_r0, b3_c1, acc01))));
            acc10 = fma(a0_r1, b0_c0, fma(a1_r1, b1_c0, fma(a2_r1, b2_c0, fma(a3_r1, b3_c0, acc10))));
            acc11 = fma(a0_r1, b0_c1, fma(a1_r1, b1_c1, fma(a2_r1, b2_c1, fma(a3_r1, b3_c1, acc11))));
        }
        // Tail (< 4 remaining k-steps)
        for (; k < k_limit; k = k + 1u) {
            let a_r0 = tileA_curr[lrow * 2u        * TILE + k];
            let a_r1 = tileA_curr[(lrow * 2u + 1u) * TILE + k];
            let b_c0 = tileB_curr[k * TILE + lcol * 2u];
            let b_c1 = tileB_curr[k * TILE + lcol * 2u + 1u];
            acc00 = fma(a_r0, b_c0, acc00);
            acc01 = fma(a_r0, b_c1, acc01);
            acc10 = fma(a_r1, b_c0, acc10);
            acc11 = fma(a_r1, b_c1, acc11);
        }

        // Swap current ↔ next: barrier ensures next is fully loaded.
        workgroupBarrier();
        // Copy next → current for the following iteration.
        // (WGSL has no pointer aliasing, so we do an explicit copy via
        // the same 2×2 load pattern each thread already owns.)
        if (next_t < num_tiles) {
            for (var dr = 0u; dr < 2u; dr = dr + 1u) {
                for (var dc = 0u; dc < 2u; dc = dc + 1u) {
                    let srow = lrow * 2u + dr;
                    let scol = lcol * 2u + dc;
                    tileA_curr[srow * TILE + scol] = tileA_next[srow * TILE + scol];
                    tileB_curr[srow * TILE + scol] = tileB_next[srow * TILE + scol];
                }
            }
            workgroupBarrier();
        }
    }

    // ── Write 2×2 output block ─────────────────────────────────────────────
    if (out_row < params.m && out_col < params.n) {
        C[out_row * params.n + out_col] = acc00;
    }
    if (out_row < params.m && out_col + 1u < params.n) {
        C[out_row * params.n + out_col + 1u] = acc01;
    }
    if (out_row + 1u < params.m && out_col < params.n) {
        C[(out_row + 1u) * params.n + out_col] = acc10;
    }
    if (out_row + 1u < params.m && out_col + 1u < params.n) {
        C[(out_row + 1u) * params.n + out_col + 1u] = acc11;
    }
}
