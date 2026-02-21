// vacf_f64.wgsl — GPU-resident Velocity Autocorrelation Function
//
// For each (t0, lag) pair, computes:
//   c_raw[t0 * max_lag + lag] = (1/N) Σ_i  v(t0,i)·v(t0+lag,i)
//
// The Rust host normalises by c_raw[0..][0] and averages over time origins.
//
// Bindings:
//   @binding(0) params      array<u32>[4] : [n_particles, n_frames, max_lag, _pad]
//   @binding(1) velocities  array<f64>   : [T × N × 3] (t outer, particle inner)
//   @binding(2) c_raw       array<f64>   : [T × L], one slot per (t0, lag)
//
// Dispatch: ceil(max_lag / 16) × ceil(n_frames / 16) workgroups.
// Each thread handles one (lag, t0) pair.

@group(0) @binding(0) var<storage, read>       params:     array<u32>;
@group(0) @binding(1) var<storage, read>       velocities: array<f64>;
@group(0) @binding(2) var<storage, read_write> c_raw:      array<f64>;

@compute @workgroup_size(16, 16)
fn vacf_pair(
    @builtin(global_invocation_id) gid: vec3<u32>,
) {
    let lag = gid.x;
    let t0  = gid.y;

    let n_particles = params[0];
    let n_frames    = params[1];
    let max_lag     = params[2];

    if lag >= max_lag || t0 >= n_frames { return; }
    let t1 = t0 + lag;
    if t1 >= n_frames { return; }

    // (1/N) Σ_i  v(t0,i)·v(t1,i)
    var dot: f64 = f64(0.0);
    let base0 = t0 * n_particles * 3u;
    let base1 = t1 * n_particles * 3u;

    for (var i = 0u; i < n_particles; i = i + 1u) {
        let o0 = base0 + i * 3u;
        let o1 = base1 + i * 3u;
        dot = dot
            + velocities[o0]       * velocities[o1]
            + velocities[o0 + 1u]  * velocities[o1 + 1u]
            + velocities[o0 + 2u]  * velocities[o1 + 2u];
    }

    c_raw[t0 * max_lag + lag] = dot / f64(n_particles);
}
