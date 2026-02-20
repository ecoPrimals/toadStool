// su3.wgsl — SU(3) 3×3 complex matrix algebra for lattice gauge fields
//
// Prepend complex_f64.wgsl before this file.
//
// Storage layout:
//   One SU(3) matrix occupies 18 consecutive f64 values in a storage buffer.
//   Row-major: element (i, j) at flat index i*3+j, stored as two f64 (re, im).
//   Buffer layout per matrix:  re00 im00 re01 im01 re02 im02
//                               re10 im10 re11 im11 re12 im12
//                               re20 im20 re21 im21 re22 im22
//
// In-register representation: array<vec2<f64>, 9>
//   Element (i,j) = arr[i*3+j]  where .x=real, .y=imag.
//
// Performance:
//   su3_mul has 27 complex FMA pairs.
//   Inner k-loop annotated with @unroll_hint 3 for WgslLoopUnroller.
//
// hotSpring absorption: lattice/su3.rs (v0.5.16, Feb 2026)
// CPU-validated against standard SU(3) algebra test vectors.

// ── Load from storage buffer ──────────────────────────────────────────────────
// Load the matrix whose first element is at `links[base]`.

fn su3_load(links: ptr<storage, array<f64>, read>, base: u32) -> array<vec2<f64>, 9> {
    var m: array<vec2<f64>, 9>;
    for (var i = 0u; i < 9u; i = i + 1u) {
        let off = base + i * 2u;
        m[i] = c64_new((*links)[off], (*links)[off + 1u]);
    }
    return m;
}

// ── Multiply: C = A × B ───────────────────────────────────────────────────────

fn su3_mul(a: array<vec2<f64>, 9>, b: array<vec2<f64>, 9>) -> array<vec2<f64>, 9> {
    var r: array<vec2<f64>, 9>;
    for (var i = 0u; i < 3u; i = i + 1u) {
        for (var j = 0u; j < 3u; j = j + 1u) {
            var acc = c64_zero();
            // @unroll_hint 3
            for (var k = 0u; k < 3u; k = k + 1u) {
                acc = c64_add(acc, c64_mul(a[i * 3u + k], b[k * 3u + j]));
            }
            r[i * 3u + j] = acc;
        }
    }
    return r;
}

// ── Adjoint (conjugate transpose): B = A† ────────────────────────────────────

fn su3_adjoint(a: array<vec2<f64>, 9>) -> array<vec2<f64>, 9> {
    var r: array<vec2<f64>, 9>;
    for (var i = 0u; i < 3u; i = i + 1u) {
        for (var j = 0u; j < 3u; j = j + 1u) {
            r[j * 3u + i] = c64_conj(a[i * 3u + j]);
        }
    }
    return r;
}

// ── Trace ─────────────────────────────────────────────────────────────────────

fn su3_trace(a: array<vec2<f64>, 9>) -> vec2<f64> {
    return c64_add(c64_add(a[0], a[4]), a[8]);
}

fn su3_re_trace(a: array<vec2<f64>, 9>) -> f64 {
    return a[0].x + a[4].x + a[8].x;
}

// ── Add / scale ───────────────────────────────────────────────────────────────

fn su3_add(a: array<vec2<f64>, 9>, b: array<vec2<f64>, 9>) -> array<vec2<f64>, 9> {
    var r: array<vec2<f64>, 9>;
    for (var i = 0u; i < 9u; i = i + 1u) { r[i] = c64_add(a[i], b[i]); }
    return r;
}

fn su3_scale(a: array<vec2<f64>, 9>, s: f64) -> array<vec2<f64>, 9> {
    var r: array<vec2<f64>, 9>;
    for (var i = 0u; i < 9u; i = i + 1u) { r[i] = c64_scale(a[i], s); }
    return r;
}

// ── Plaquette product: U_mu(x) * U_nu(x+mu) * U_mu†(x+nu) * U_nu†(x) ────────
// All four links supplied as pre-loaded matrices.

fn su3_plaquette(u_mu: array<vec2<f64>, 9>,
                 u_nu_fwd: array<vec2<f64>, 9>,
                 u_mu_fwd_nu: array<vec2<f64>, 9>,
                 u_nu: array<vec2<f64>, 9>) -> array<vec2<f64>, 9> {
    return su3_mul(
        su3_mul(u_mu, u_nu_fwd),
        su3_mul(su3_adjoint(u_mu_fwd_nu), su3_adjoint(u_nu)),
    );
}

// ── Identity ──────────────────────────────────────────────────────────────────

fn su3_identity() -> array<vec2<f64>, 9> {
    var r: array<vec2<f64>, 9>;
    for (var i = 0u; i < 9u; i = i + 1u) { r[i] = c64_zero(); }
    r[0] = c64_one();
    r[4] = c64_one();
    r[8] = c64_one();
    return r;
}
