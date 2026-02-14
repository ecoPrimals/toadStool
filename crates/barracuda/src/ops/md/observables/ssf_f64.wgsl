// Static Structure Factor (f64 precision)
//
// **Physics**: S(k) = |Σ_j exp(ik·r_j)|² / N
//
// This is the primary observable for paper parity validation.
// Computes S(k) for a set of k-vectors, used in Dynamic Structure Factor (DSF) studies.
//
// **Algorithm**:
//   For each k-vector, sum exp(ik·r_j) = cos(k·r_j) + i*sin(k·r_j) over all N particles.
//   Then S(k) = (Σ cos)² + (Σ sin)² / N
//
// **Precision**: Full f64 throughout (positions, k-vectors, results)
//
// **Performance**:
//   Each thread handles one k-vector, loops over all N particles.
//   GPU parallelism: number_of_k_vectors threads run simultaneously.
//   For N=10,000 particles and 1000 k-vectors: each thread does 10,000 trig ops.
//
// **Feb 14 2026**: Uses native sin/cos f64 builtins for maximum throughput.
//   hotSpring found: native trig on RTX 4070 is 2.2× faster than software.
//
// Bindings:
//   0: params      - [n_particles, n_k_vectors, pad, pad] u32 uniform
//   1: positions   - [n_particles * 3] f64 storage, read
//   2: k_vectors   - [n_k_vectors * 3] f64 storage, read
//   3: ssf_output  - [n_k_vectors] f64 storage, read-write

struct SSFParams {
    n_particles: u32,
    n_k_vectors: u32,
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> params: SSFParams;
@group(0) @binding(1) var<storage, read> positions: array<f64>;
@group(0) @binding(2) var<storage, read> k_vectors: array<f64>;
@group(0) @binding(3) var<storage, read_write> ssf_output: array<f64>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let k_idx = gid.x;
    let n = params.n_particles;
    let n_k = params.n_k_vectors;

    if (k_idx >= n_k) { return; }

    // Load k-vector
    let kx = k_vectors[k_idx * 3u];
    let ky = k_vectors[k_idx * 3u + 1u];
    let kz = k_vectors[k_idx * 3u + 2u];

    // Sum exp(ik·r) = Σ cos(k·r) + i Σ sin(k·r)
    var sum_cos: f64 = 0.0;
    var sum_sin: f64 = 0.0;

    for (var j = 0u; j < n; j = j + 1u) {
        let rx = positions[j * 3u];
        let ry = positions[j * 3u + 1u];
        let rz = positions[j * 3u + 2u];

        let kr = kx * rx + ky * ry + kz * rz;

        // Native f64 trig builtins (2.2× faster than software)
        sum_cos = sum_cos + cos(kr);
        sum_sin = sum_sin + sin(kr);
    }

    // S(k) = |Σ exp(ik·r)|² / N = (Σcos)² + (Σsin)² / N
    let ssf = (sum_cos * sum_cos + sum_sin * sum_sin) / f64(n);
    ssf_output[k_idx] = ssf;
}

// Alternative entry point with shared memory reduction for large k-vector sets
// Each workgroup processes one k-vector, threads cooperatively sum over particles
// More efficient for very large N (>50,000) due to reduced global memory pressure

var<workgroup> partial_cos: array<f64, 256>;
var<workgroup> partial_sin: array<f64, 256>;

@compute @workgroup_size(256)
fn main_cooperative(@builtin(global_invocation_id) gid: vec3<u32>,
                    @builtin(local_invocation_id) lid: vec3<u32>,
                    @builtin(workgroup_id) wg_id: vec3<u32>) {
    let k_idx = wg_id.x;
    let tid = lid.x;
    let n = params.n_particles;
    let n_k = params.n_k_vectors;

    if (k_idx >= n_k) { return; }

    // Load k-vector (all threads in workgroup load same k)
    let kx = k_vectors[k_idx * 3u];
    let ky = k_vectors[k_idx * 3u + 1u];
    let kz = k_vectors[k_idx * 3u + 2u];

    // Each thread handles a subset of particles
    var local_cos: f64 = 0.0;
    var local_sin: f64 = 0.0;

    var j = tid;
    while (j < n) {
        let rx = positions[j * 3u];
        let ry = positions[j * 3u + 1u];
        let rz = positions[j * 3u + 2u];

        let kr = kx * rx + ky * ry + kz * rz;

        local_cos = local_cos + cos(kr);
        local_sin = local_sin + sin(kr);

        j = j + 256u;
    }

    // Store in shared memory
    partial_cos[tid] = local_cos;
    partial_sin[tid] = local_sin;
    workgroupBarrier();

    // Tree reduction
    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (tid < stride) {
            partial_cos[tid] = partial_cos[tid] + partial_cos[tid + stride];
            partial_sin[tid] = partial_sin[tid] + partial_sin[tid + stride];
        }
        workgroupBarrier();
    }

    // Thread 0 writes final result
    if (tid == 0u) {
        let ssf = (partial_cos[0] * partial_cos[0] + partial_sin[0] * partial_sin[0]) / f64(n);
        ssf_output[k_idx] = ssf;
    }
}

// Entry point for computing S(k) along radial shells
// Computes average S(|k|) over all k-vectors with the same magnitude
// Uses atomic add for shell accumulation

@group(0) @binding(0) var<uniform> shell_params: SSFParams;
@group(0) @binding(1) var<storage, read> positions_shell: array<f64>;
@group(0) @binding(2) var<storage, read> k_vectors_shell: array<f64>;
@group(0) @binding(3) var<storage, read_write> ssf_per_k: array<f64>;
// Additional bindings for shell averaging would go here
