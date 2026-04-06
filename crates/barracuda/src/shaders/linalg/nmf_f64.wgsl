// Non-Negative Matrix Factorization (NMF) f64 - Multiplicative Update Rules
//
// Decomposes V ≈ W × H where V ∈ ℝ₊^{m×n}, W ∈ ℝ₊^{m×k}, H ∈ ℝ₊^{k×n}
// All elements are non-negative.
//
// Algorithm: Lee & Seung (2001) multiplicative update rules
//   H ← H ⊙ (Wᵀ V) / (Wᵀ W H + ε)
//   W ← W ⊙ (V Hᵀ) / (W H Hᵀ + ε)
// where ⊙ is element-wise multiplication and ε prevents division by zero.
//
// Each update step requires GEMM operations (dispatched via gemm_f64.wgsl)
// and element-wise update kernels (defined here).
//
// Use cases: Drug-disease scoring matrices (~4,000 × 18,000), topic modeling,
//            biomedical knowledge graph decomposition
//
// Reference: Lee & Seung (2001) "Algorithms for Non-negative Matrix Factorization" NeurIPS
//
// Deep Debt Principles:
// - Pure WGSL (universal compute, hardware-agnostic)
// - Full f64 precision via SPIR-V/Vulkan
// - Zero unsafe code
// - Self-contained (no external dependencies)

struct NmfParams {
    m: u32,         // Rows of V (drugs: ~4,000)
    n: u32,         // Columns of V (diseases: ~18,000)
    k: u32,         // Factorization rank (50-200)
    _pad: u32,
}

// ─── Binding Groups ─────────────────────────────────────────────────────────
//
// The NMF pipeline uses multiple dispatch passes. Each entry point below
// documents its own binding expectations. The host (Rust) rebinds buffers
// between passes.

// ─── H Update: H ← H ⊙ numerator / (denominator + ε) ──────────────────────
//
// numerator  = Wᵀ V   [k × n]  (computed via gemm_f64.wgsl)
// denominator = Wᵀ W H [k × n]  (computed via two GEMM passes)
//
// This kernel applies the element-wise multiplicative update.

@group(0) @binding(0) var<uniform> params: NmfParams;
@group(0) @binding(1) var<storage, read_write> H: array<f64>;        // [k × n], updated in-place
@group(0) @binding(2) var<storage, read> numerator: array<f64>;      // Wᵀ V [k × n]
@group(0) @binding(3) var<storage, read> denominator: array<f64>;    // Wᵀ W H [k × n]

const EPS: f64 = 1e-16;

@compute @workgroup_size(256, 1, 1)
fn update_H(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total = params.k * params.n;

    if (idx >= total) {
        return;
    }

    let num = numerator[idx];
    let den = denominator[idx] + EPS;
    H[idx] = H[idx] * num / den;
}

// ─── W Update: W ← W ⊙ numerator / (denominator + ε) ──────────────────────
//
// numerator  = V Hᵀ    [m × k]  (computed via gemm_f64.wgsl)
// denominator = W H Hᵀ  [m × k]  (computed via two GEMM passes)

@group(0) @binding(0) var<uniform> params_w: NmfParams;
@group(0) @binding(1) var<storage, read_write> W: array<f64>;           // [m × k], updated in-place
@group(0) @binding(2) var<storage, read> numerator_w: array<f64>;       // V Hᵀ [m × k]
@group(0) @binding(3) var<storage, read> denominator_w: array<f64>;     // W H Hᵀ [m × k]

@compute @workgroup_size(256, 1, 1)
fn update_W(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total = params_w.m * params_w.k;

    if (idx >= total) {
        return;
    }

    let num = numerator_w[idx];
    let den = denominator_w[idx] + EPS;
    W[idx] = W[idx] * num / den;
}

// ─── Column Normalization: normalize W columns, scale H rows ────────────────
//
// After each iteration, normalize: W[:,j] /= ||W[:,j]||  and H[j,:] *= ||W[:,j]||
// This prevents W from growing unbounded while H shrinks (or vice versa).

struct NormColParams {
    m: u32,
    k: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> norm_params: NormColParams;
@group(0) @binding(1) var<storage, read_write> W_norm: array<f64>;     // [m × k]
@group(0) @binding(2) var<storage, read_write> col_norms: array<f64>;  // [k] output norms

var<workgroup> shared_sum: array<f64, 256>;

@compute @workgroup_size(256, 1, 1)
fn compute_col_norms(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) wg_id: vec3<u32>,
) {
    let tid = local_id.x;
    let col = wg_id.x;
    let m = norm_params.m;
    let k = norm_params.k;

    if (col >= k) {
        return;
    }

    var sum: f64 = 0.0;
    var row = tid;
    while (row < m) {
        let val = W_norm[row * k + col];
        sum = sum + val * val;
        row = row + 256u;
    }

    shared_sum[tid] = sum;
    workgroupBarrier();

    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (tid < stride) {
            shared_sum[tid] = shared_sum[tid] + shared_sum[tid + stride];
        }
        workgroupBarrier();
    }

    if (tid == 0u) {
        let norm = sqrt(shared_sum[0]);
        col_norms[col] = select(norm, 1.0, norm < EPS);
    }
}

@compute @workgroup_size(256, 1, 1)
fn normalize_W_cols(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let m = norm_params.m;
    let k = norm_params.k;
    let total = m * k;

    if (idx >= total) {
        return;
    }

    let col = idx % k;
    W_norm[idx] = W_norm[idx] / col_norms[col];
}

// Scale H rows by the column norms from W normalization
struct ScaleHParams {
    k: u32,
    n: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> scale_params: ScaleHParams;
@group(0) @binding(1) var<storage, read_write> H_scale: array<f64>;   // [k × n]
@group(0) @binding(2) var<storage, read> norms: array<f64>;           // [k]

@compute @workgroup_size(256, 1, 1)
fn scale_H_rows(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let k = scale_params.k;
    let n = scale_params.n;
    let total = k * n;

    if (idx >= total) {
        return;
    }

    let row = idx / n;
    H_scale[idx] = H_scale[idx] * norms[row];
}

// ─── Frobenius Norm: ||V - W H||_F for convergence check ───────────────────
//
// Computed as element-wise (V[i,j] - (WH)[i,j])² summed over all (i,j).
// WH is precomputed via gemm_f64.wgsl.

struct FrobParams {
    total: u32,     // m × n
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<uniform> frob_params: FrobParams;
@group(0) @binding(1) var<storage, read> V_frob: array<f64>;      // [m × n]
@group(0) @binding(2) var<storage, read> WH_frob: array<f64>;     // [m × n] (precomputed)
@group(0) @binding(3) var<storage, read_write> frob_out: array<f64>;  // [1] partial sum

var<workgroup> shared_frob: array<f64, 256>;

@compute @workgroup_size(256, 1, 1)
fn frobenius_residual(
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) wg_id: vec3<u32>,
) {
    let tid = local_id.x;
    let total = frob_params.total;

    var sum: f64 = 0.0;
    var idx = global_id.x;
    while (idx < total) {
        let diff = V_frob[idx] - WH_frob[idx];
        sum = sum + diff * diff;
        idx = idx + 256u * 65535u;
    }

    shared_frob[tid] = sum;
    workgroupBarrier();

    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (tid < stride) {
            shared_frob[tid] = shared_frob[tid] + shared_frob[tid + stride];
        }
        workgroupBarrier();
    }

    if (tid == 0u) {
        frob_out[wg_id.x] = shared_frob[0];
    }
}

// ─── Cosine Similarity: for scoring factor matrices after NMF ───────────────
//
// Given H [k × n], compute pairwise cosine similarity between columns:
//   sim(i,j) = H[:,i] · H[:,j] / (||H[:,i]|| × ||H[:,j]||)
// Used for drug-disease scoring after NMF decomposition.

struct CosineParams {
    k: u32,         // Feature dimension (NMF rank)
    n: u32,         // Number of columns (diseases or drugs)
    col_i: u32,     // First column index
    col_j: u32,     // Second column index
}

@group(0) @binding(0) var<uniform> cos_params: CosineParams;
@group(0) @binding(1) var<storage, read> H_cos: array<f64>;          // [k × n]
@group(0) @binding(2) var<storage, read_write> sim_out: array<f64>;  // [1] result

var<workgroup> shared_dot: array<f64, 256>;
var<workgroup> shared_norm_a: array<f64, 256>;
var<workgroup> shared_norm_b: array<f64, 256>;

@compute @workgroup_size(256, 1, 1)
fn cosine_similarity(
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let tid = local_id.x;
    let k = cos_params.k;
    let n = cos_params.n;
    let ci = cos_params.col_i;
    let cj = cos_params.col_j;

    var dot: f64 = 0.0;
    var na: f64 = 0.0;
    var nb: f64 = 0.0;

    var row = tid;
    while (row < k) {
        let a = H_cos[row * n + ci];
        let b = H_cos[row * n + cj];
        dot = dot + a * b;
        na = na + a * a;
        nb = nb + b * b;
        row = row + 256u;
    }

    shared_dot[tid] = dot;
    shared_norm_a[tid] = na;
    shared_norm_b[tid] = nb;
    workgroupBarrier();

    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (tid < stride) {
            shared_dot[tid] = shared_dot[tid] + shared_dot[tid + stride];
            shared_norm_a[tid] = shared_norm_a[tid] + shared_norm_a[tid + stride];
            shared_norm_b[tid] = shared_norm_b[tid] + shared_norm_b[tid + stride];
        }
        workgroupBarrier();
    }

    if (tid == 0u) {
        let denom = sqrt(shared_norm_a[0]) * sqrt(shared_norm_b[0]);
        if (denom > EPS) {
            sim_out[0] = shared_dot[0] / denom;
        } else {
            sim_out[0] = 0.0;
        }
    }
}
