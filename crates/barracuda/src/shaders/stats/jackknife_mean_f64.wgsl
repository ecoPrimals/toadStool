// Leave-one-out jackknife for the mean — GPU parallel.
// Thread i computes leave-out-i mean = (full_sum - data[i]) / (n-1).
// Also computes partial (θ_i - θ_bar)^2 for variance reduction on CPU.

struct Params {
    n: u32,
    full_sum_lo: u32,
    full_sum_hi: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> data: array<f64>;
@group(0) @binding(1) var<storage, read_write> leave_means: array<f64>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.n) {
        return;
    }

    let full_sum = bitcast<f64>(vec2<u32>(params.full_sum_lo, params.full_sum_hi));
    let n_f = f64(params.n);
    let leave_mean = (full_sum - data[idx]) / (n_f - 1.0);
    leave_means[idx] = leave_mean;
}
