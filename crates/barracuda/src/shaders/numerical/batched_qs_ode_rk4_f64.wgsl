// batched_qs_ode_rk4_f64.wgsl — Full-GPU RK4 parameter sweep for QS/c-di-GMP ODE
//
// **Wetspring handoff absorption** — bioinformatics life-science workload.
//
// Purpose: Map the biofilm formation landscape by running B independent
// parameter sets simultaneously.  Each GPU thread integrates one complete
// trajectory to steady state.  At B=10,000 this gives a ~1,000× speedup
// over sequential CPU integration.
//
// ODE system (Waters 2008, Eq. 1–5):
//   dN/dt = μ·N·(1 - N/K)  -  d·N                         (logistic growth)
//   dA/dt = k_ai·N  -  d_ai·A                             (autoinducer)
//   dH/dt = k_h·Hill(A, K_h, n_h)  -  d_h·H              (HapR)
//   dC/dt = k_dgc·(1 - k_rep·H)  -  (k_pde + k_act·H)·C  (c-di-GMP)
//   dB/dt = k_bio·Hill(C, K_bio, n_bio)·(1−B)  -  d_bio·B (biofilm)
//
// State vector: [N, A, H, C, B]   (5 variables)
// Parameters (per batch, 17 values):
//   [mu, K_cap, d_n, k_ai, d_ai, k_h, K_h, n_h, d_h,
//    k_dgc, k_rep, k_pde, k_act, k_bio, K_bio, n_bio, d_bio]
//
// Integrator: Classic RK4 (fixed step)
//   k1 = f(t,     y)
//   k2 = f(t+h/2, y + h/2·k1)
//   k3 = f(t+h/2, y + h/2·k2)
//   k4 = f(t+h,   y + h·k3)
//   y_new = y + h/6·(k1 + 2·k2 + 2·k3 + k4)
//
// Bindings:
//   0: QsOdeConfig uniform (n_batches, n_steps, h, t0, state_clamp_max)
//   1: initial_states  [B × 5]  f64 — one initial condition per batch
//   2: params          [B × 17] f64 — parameter set per batch
//   3: output_states   [B × 5]  f64 — final state after n_steps

enable f64;

const N_VARS:   u32 = 5u;
const N_PARAMS: u32 = 17u;

struct QsOdeConfig {
    n_batches:      u32,
    n_steps:        u32,
    _pad0:          u32,
    _pad1:          u32,
    h:              f64,   // Fixed RK4 step size
    t0:             f64,   // Initial time (informational; ODE is autonomous)
    clamp_max:      f64,   // Upper bound on any state (prevents blow-up during sweep)
    clamp_min:      f64,   // Lower bound (non-negativity)
}

@group(0) @binding(0) var<uniform>             config:         QsOdeConfig;
@group(0) @binding(1) var<storage, read>       initial_states: array<f64>;  // [B × 5]
@group(0) @binding(2) var<storage, read>       batch_params:   array<f64>;  // [B × 17]
@group(0) @binding(3) var<storage, read_write> output_states:  array<f64>;  // [B × 5]

// ── Hill activation ───────────────────────────────────────────────────────────
// Hill(x, K, n) = xⁿ / (Kⁿ + xⁿ)
// Clamped to [0, 1]; input x clamped to ≥ 0 to avoid NaN from negative pow().
fn hill(x: f64, K: f64, n: f64) -> f64 {
    let xc = max(x, 0.0);
    let xn = pow(xc, n);
    let Kn = pow(max(K, 1e-30), n);
    return xn / (Kn + xn);
}

// ── QS/c-di-GMP derivative function ─────────────────────────────────────────
// y = [N, A, H, C, B];  p[0..16] = parameter array
fn qs_deriv(y0: f64, y1: f64, y2: f64, y3: f64, y4: f64,
            p_base: u32) -> array<f64, 5> {
    let mu    = batch_params[p_base + 0u];
    let K_cap = batch_params[p_base + 1u];
    let d_n   = batch_params[p_base + 2u];
    let k_ai  = batch_params[p_base + 3u];
    let d_ai  = batch_params[p_base + 4u];
    let k_h   = batch_params[p_base + 5u];
    let K_h   = batch_params[p_base + 6u];
    let n_h   = batch_params[p_base + 7u];
    let d_h   = batch_params[p_base + 8u];
    let k_dgc = batch_params[p_base + 9u];
    let k_rep = batch_params[p_base + 10u];
    let k_pde = batch_params[p_base + 11u];
    let k_act = batch_params[p_base + 12u];
    let k_bio = batch_params[p_base + 13u];
    let K_bio = batch_params[p_base + 14u];
    let n_bio = batch_params[p_base + 15u];
    let d_bio = batch_params[p_base + 16u];

    var dy: array<f64, 5>;
    // dN/dt
    dy[0] = mu * y0 * (1.0 - y0 / max(K_cap, 1e-30)) - d_n * y0;
    // dA/dt
    dy[1] = k_ai * y0 - d_ai * y1;
    // dH/dt
    dy[2] = k_h * hill(y1, K_h, n_h) - d_h * y2;
    // dC/dt
    dy[3] = k_dgc * (1.0 - k_rep * y2) - (k_pde + k_act * y2) * y3;
    // dB/dt  (biofilm fraction clamped to [0,1] via logistic structure)
    dy[4] = k_bio * hill(y3, K_bio, n_bio) * (1.0 - y4) - d_bio * y4;
    return dy;
}

// ── RK4 step ─────────────────────────────────────────────────────────────────
fn rk4_step(y0: f64, y1: f64, y2: f64, y3: f64, y4: f64,
            p_base: u32, h: f64) -> array<f64, 5> {
    let h2 = h * 0.5;

    // k1 = f(y)
    let k1 = qs_deriv(y0, y1, y2, y3, y4, p_base);

    // k2 = f(y + h/2·k1)
    let k2 = qs_deriv(
        y0 + h2 * k1[0], y1 + h2 * k1[1], y2 + h2 * k1[2],
        y3 + h2 * k1[3], y4 + h2 * k1[4], p_base);

    // k3 = f(y + h/2·k2)
    let k3 = qs_deriv(
        y0 + h2 * k2[0], y1 + h2 * k2[1], y2 + h2 * k2[2],
        y3 + h2 * k2[3], y4 + h2 * k2[4], p_base);

    // k4 = f(y + h·k3)
    let k4 = qs_deriv(
        y0 + h * k3[0], y1 + h * k3[1], y2 + h * k3[2],
        y3 + h * k3[3], y4 + h * k3[4], p_base);

    var y_new: array<f64, 5>;
    let sixth = 1.0 / 6.0;
    y_new[0] = y0 + h * sixth * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
    y_new[1] = y1 + h * sixth * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
    y_new[2] = y2 + h * sixth * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]);
    y_new[3] = y3 + h * sixth * (k1[3] + 2.0 * k2[3] + 2.0 * k3[3] + k4[3]);
    y_new[4] = y4 + h * sixth * (k1[4] + 2.0 * k2[4] + 2.0 * k3[4] + k4[4]);
    return y_new;
}

// ── Main ─────────────────────────────────────────────────────────────────────
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let b = global_id.x;
    if (b >= config.n_batches) { return; }

    let s_base = b * N_VARS;
    let p_base = b * N_PARAMS;

    // Load initial state
    var y0 = initial_states[s_base + 0u];
    var y1 = initial_states[s_base + 1u];
    var y2 = initial_states[s_base + 2u];
    var y3 = initial_states[s_base + 3u];
    var y4 = initial_states[s_base + 4u];

    let cmax = config.clamp_max;
    let cmin = config.clamp_min;
    let h    = config.h;

    // Integrate for n_steps steps
    for (var step = 0u; step < config.n_steps; step = step + 1u) {
        let yn = rk4_step(y0, y1, y2, y3, y4, p_base, h);

        // Non-negativity + blow-up guard
        y0 = clamp(yn[0], cmin, cmax);
        y1 = clamp(yn[1], cmin, cmax);
        y2 = clamp(yn[2], cmin, cmax);
        y3 = clamp(yn[3], cmin, cmax);
        y4 = clamp(yn[4], cmin, 1.0);  // biofilm fraction ∈ [0,1]
    }

    // Write final state
    output_states[s_base + 0u] = y0;
    output_states[s_base + 1u] = y1;
    output_states[s_base + 2u] = y2;
    output_states[s_base + 3u] = y3;
    output_states[s_base + 4u] = y4;
}
