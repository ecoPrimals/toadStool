// matmul_gpu_evolved.wgsl — Double-buffered 32×32 tiled matmul for large GPU matrices
//
// Absorbed from neuralSpring local evolutions (neuralSpring handoff #11).
// Effective when M ≥ 256 and N ≥ 256 on a discrete/integrated GPU.
//
// Key optimisations vs matmul_tiled.wgsl (16×16 single-buffer):
//   1. TILE = 32 → 4× more arithmetic reuse per global-memory round-trip;
//      reduces bandwidth pressure which is the dominant cost on large matmuls.
//   2. Double-buffered tiles — issues loads for tile t+1 while the ALU is
//      computing tile t; overlaps memory latency with arithmetic (SM-style
//      warp scheduling on NVIDIA / wave64 scheduling on AMD).
//   3. 2×2 micro-kernel — each thread accumulates 4 outputs, doubling
//      arithmetic intensity (FLOPs per byte loaded) vs the single-element kernel.
//   4. 4× k-loop unroll — creates 4 independent FMA dependency chains,
//      filling SM warp latency windows (8 cycles/DFMA on Turing).
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

// Double-buffered 32×32 tiles (4 KB each, 16 KB total workgroup memory).
var<workgroup> tileA_curr: array<f32, 1024>;
var<workgroup> tileB_curr: array<f32, 1024>;
var<workgroup> tileA_next: array<f32, 1024>;
var<workgroup> tileB_next: array<f32, 1024>;

// 16×16 workgroup; each thread handles a 2×2 output block → 32×32 output tile/WG.
@compute @workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) global_id:    vec3<u32>,
    @builtin(local_invocation_id)  local_id:     vec3<u32>,
    @builtin(workgroup_id)         workgroup_id: vec3<u32>,
) {
    let out_row = workgroup_id.y * TILE + local_id.y * 2u;
    let out_col = workgroup_id.x * TILE + local_id.x * 2u;

    let lrow = local_id.y;
    let lcol = local_id.x;

    var acc00 = 0.0f;
    var acc01 = 0.0f;
    var acc10 = 0.0f;
    var acc11 = 0.0f;

    let num_tiles = (params.k + TILE - 1u) / TILE;

    // ── Pre-load tile 0 ───────────────────────────────────────────────────
    for (var dr = 0u; dr < 2u; dr = dr + 1u) {
        for (var dc = 0u; dc < 2u; dc = dc + 1u) {
            let srow = lrow * 2u + dr;
            let scol = lcol * 2u + dc;

            let a_row = workgroup_id.y * TILE + srow;
            let a_col = scol;
            if (a_row < params.m && a_col < params.k) {
                tileA_curr[srow * TILE + scol] = A[a_row * params.k + a_col];
            } else {
                tileA_curr[srow * TILE + scol] = 0.0f;
            }

            let b_row = srow;
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

        // Prefetch tile t+1 while computing tile t.
        // On GPU, this overlaps async global-memory loads with the FMA pipeline.
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

        // ── 2×2 micro-kernel, 4× k-unrolled ──────────────────────────────
        // Four independent accumulator chains hide FMA issue latency.
        let k_limit = min(TILE, params.k - t * TILE);
        var k = 0u;

        for (; k + 4u <= k_limit; k = k + 4u) {
            // Load 2 A-rows × 4 k-steps
            let a00 = tileA_curr[lrow * 2u        * TILE + k];
            let a01 = tileA_curr[lrow * 2u        * TILE + k + 1u];
            let a02 = tileA_curr[lrow * 2u        * TILE + k + 2u];
            let a03 = tileA_curr[lrow * 2u        * TILE + k + 3u];
            let a10 = tileA_curr[(lrow * 2u + 1u) * TILE + k];
            let a11 = tileA_curr[(lrow * 2u + 1u) * TILE + k + 1u];
            let a12 = tileA_curr[(lrow * 2u + 1u) * TILE + k + 2u];
            let a13 = tileA_curr[(lrow * 2u + 1u) * TILE + k + 3u];

            // Load 2 B-cols × 4 k-steps
            let b00 = tileB_curr[k        * TILE + lcol * 2u];
            let b10 = tileB_curr[(k + 1u) * TILE + lcol * 2u];
            let b20 = tileB_curr[(k + 2u) * TILE + lcol * 2u];
            let b30 = tileB_curr[(k + 3u) * TILE + lcol * 2u];
            let b01 = tileB_curr[k        * TILE + lcol * 2u + 1u];
            let b11 = tileB_curr[(k + 1u) * TILE + lcol * 2u + 1u];
            let b21 = tileB_curr[(k + 2u) * TILE + lcol * 2u + 1u];
            let b31 = tileB_curr[(k + 3u) * TILE + lcol * 2u + 1u];

            // 4 independent FMA chains → 4-way ILP on GPU issue units
            acc00 = fma(a00, b00, fma(a01, b10, fma(a02, b20, fma(a03, b30, acc00))));
            acc01 = fma(a00, b01, fma(a01, b11, fma(a02, b21, fma(a03, b31, acc01))));
            acc10 = fma(a10, b00, fma(a11, b10, fma(a12, b20, fma(a13, b30, acc10))));
            acc11 = fma(a10, b01, fma(a11, b11, fma(a12, b21, fma(a13, b31, acc11))));
        }
        // Tail: remaining < 4 k-steps
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

        // Synchronise then swap current ↔ prefetched buffers.
        workgroupBarrier();
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
