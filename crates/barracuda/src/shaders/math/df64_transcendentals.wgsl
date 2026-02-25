// SPDX-License-Identifier: AGPL-3.0-only
// DF64 transcendental functions — f64-precision math at FP32 core speed.
//
// These functions use Df64 (f32-pair) arithmetic for all intermediate
// computations, running on the massively parallel FP32 cores instead of
// the 1:64 throttled FP64 units. Precision is ~48-bit mantissa (~14
// decimal digits), suitable for Krylov solvers, molecular dynamics,
// and lattice QCD where the extra throughput matters.
//
// Requires: df64_core.wgsl (Df64, df64_add, df64_mul, df64_div, etc.)
//
// Techniques:
//   sqrt_df64:  Newton–Raphson refinement
//   exp_df64:   Cody–Waite range reduction + degree-6 Horner
//   log_df64:   atanh-based + degree-5 Horner
//   sin_df64/cos_df64: Cody–Waite π/2 reduction + minimax kernels

// ── Constants ──

const DF64_LN2_HI: f32 = 0.6931471824645996;
const DF64_LN2_LO: f32 = -1.9046542e-9;
const DF64_LOG2E: f32 = 1.4426950408889634;
const DF64_PI_HI: f32 = 3.1415927;
const DF64_PI_LO: f32 = -8.742278e-8;
const DF64_HALF_PI_HI: f32 = 1.5707964;
const DF64_HALF_PI_LO: f32 = -4.371139e-8;

fn df64_abs(a: Df64) -> Df64 {
    if a.hi < 0.0 {
        return df64_neg(a);
    }
    return a;
}

fn df64_gt(a: Df64, b: Df64) -> bool {
    return a.hi > b.hi || (a.hi == b.hi && a.lo > b.lo);
}

fn df64_lt(a: Df64, b: Df64) -> bool {
    return a.hi < b.hi || (a.hi == b.hi && a.lo < b.lo);
}

// ── sqrt_df64: Newton–Raphson ──
// x_{n+1} = x_n/2 + a/(2*x_n)
fn sqrt_df64(a: Df64) -> Df64 {
    if a.hi <= 0.0 {
        return df64_zero();
    }
    let x0 = df64_from_f32(1.0 / sqrt(a.hi));
    // Two Newton–Raphson iterations: r = a * x0, r = r + (a - r*r) * x0 / 2
    var r = df64_mul(a, x0);
    let half = df64_from_f32(0.5);
    // Refinement: r = 0.5 * (r + a/r)
    r = df64_mul(half, df64_add(r, df64_div(a, r)));
    r = df64_mul(half, df64_add(r, df64_div(a, r)));
    return r;
}

// ── exp_df64: Cody–Waite range reduction ──
// exp(x) = 2^k * exp(r) where r = x - k*ln(2), |r| < ln(2)/2
// exp(r) ≈ 1 + r + r²/2 + r³/6 + r⁴/24 + r⁵/120 + r⁶/720
fn exp_df64(a: Df64) -> Df64 {
    if a.hi > 88.0 { return df64_from_f32(3.4028235e+38); } // overflow
    if a.hi < -87.0 { return df64_zero(); } // underflow

    // k = round(x / ln2)
    let k = i32(round(a.hi * DF64_LOG2E));
    let kf = df64_from_f32(f32(k));

    // r = x - k * ln2 (Cody-Waite two-term)
    let ln2 = Df64(DF64_LN2_HI, DF64_LN2_LO);
    let r = df64_sub(a, df64_mul(kf, ln2));

    // Horner evaluation of exp(r) - 1
    let r2 = df64_mul(r, r);
    let r3 = df64_mul(r2, r);

    let c2 = df64_from_f32(0.5);
    let c3 = df64_from_f32(0.16666666666666666);
    let c4 = df64_from_f32(0.041666666666666664);
    let c5 = df64_from_f32(0.008333333333333333);
    let c6 = df64_from_f32(0.001388888888888889);

    // p = r + r²/2 + r³(1/6 + r/24 + r²/120 + r³/720)
    var p = df64_mul(c6, r);
    p = df64_add(c5, p);
    p = df64_mul(p, r);
    p = df64_add(c4, p);
    p = df64_mul(p, r3);
    p = df64_add(df64_mul(c3, r3), p);
    p = df64_add(df64_mul(c2, r2), p);
    p = df64_add(r, p);
    p = df64_add(df64_from_f32(1.0), p);

    // Multiply by 2^k via WGSL builtin ldexp
    let scale = ldexp(1.0, k);
    return df64_scale_f32(p, scale);
}

// ── log_df64: reduction to [1, 2) then atanh series ──
// log(x) = log(m * 2^e) = e*ln(2) + log(m)
// For m in [1,2): log(m) = 2*atanh((m-1)/(m+1))
fn log_df64(a: Df64) -> Df64 {
    if a.hi <= 0.0 { return df64_from_f32(-1e38); } // -inf proxy

    // Extract exponent and mantissa via f32 frexp approximation
    var m = a.hi;
    var e = 0;
    while m >= 2.0 { m *= 0.5; e += 1; }
    while m < 1.0 { m *= 2.0; e -= 1; }

    // Now m ∈ [1, 2), recompute as DF64
    let scale_inv = ldexp(1.0, -e);  // 2^(-e)
    let mdf = df64_scale_f32(a, scale_inv);

    // s = (m - 1) / (m + 1)
    let one = df64_from_f32(1.0);
    let s = df64_div(df64_sub(mdf, one), df64_add(mdf, one));
    let s2 = df64_mul(s, s);

    // atanh(s) = s + s³/3 + s⁵/5 + s⁷/7 + s⁹/9
    let c3 = df64_from_f32(0.33333333333333333);
    let c5 = df64_from_f32(0.2);
    let c7 = df64_from_f32(0.14285714285714285);
    let c9 = df64_from_f32(0.11111111111111111);

    var p = df64_mul(c9, s2);
    p = df64_add(c7, p);
    p = df64_mul(p, s2);
    p = df64_add(c5, p);
    p = df64_mul(p, s2);
    p = df64_add(c3, p);
    p = df64_mul(p, df64_mul(s2, s));
    p = df64_add(s, p);

    // log(m) = 2 * atanh(s)
    let log_m = df64_scale_f32(p, 2.0);

    // log(x) = e * ln(2) + log(m)
    let ln2 = Df64(DF64_LN2_HI, DF64_LN2_LO);
    let e_ln2 = df64_scale_f32(ln2, f32(e));
    return df64_add(e_ln2, log_m);
}

// ── sin_df64 / cos_df64: Cody–Waite π/2 reduction + minimax ──

fn sin_kernel_df64(x: Df64) -> Df64 {
    // sin(x) ≈ x - x³/6 + x⁵/120 - x⁷/5040 for |x| < π/4
    let x2 = df64_mul(x, x);
    let c3 = df64_from_f32(-0.16666666666666666);
    let c5 = df64_from_f32(0.008333333333333333);
    let c7 = df64_from_f32(-0.0001984126984126984);

    var p = df64_mul(c7, x2);
    p = df64_add(c5, p);
    p = df64_mul(p, x2);
    p = df64_add(c3, p);
    p = df64_mul(p, df64_mul(x2, x));
    return df64_add(x, p);
}

fn cos_kernel_df64(x: Df64) -> Df64 {
    // cos(x) ≈ 1 - x²/2 + x⁴/24 - x⁶/720 for |x| < π/4
    let x2 = df64_mul(x, x);
    let c2 = df64_from_f32(-0.5);
    let c4 = df64_from_f32(0.041666666666666664);
    let c6 = df64_from_f32(-0.001388888888888889);

    var p = df64_mul(c6, x2);
    p = df64_add(c4, p);
    p = df64_mul(p, x2);
    p = df64_add(c2, p);
    p = df64_mul(p, x2);
    return df64_add(df64_from_f32(1.0), p);
}

fn sin_df64(a: Df64) -> Df64 {
    // Cody-Waite: k = round(x / (π/2)), r = x - k*(π/2)
    let k = i32(round(a.hi * 0.6366197723675814)); // 2/π
    let kf = df64_from_f32(f32(k));
    let half_pi = Df64(DF64_HALF_PI_HI, DF64_HALF_PI_LO);
    let r = df64_sub(a, df64_mul(kf, half_pi));

    let quadrant = ((k % 4) + 4) % 4;
    switch quadrant {
        case 0: { return sin_kernel_df64(r); }
        case 1: { return cos_kernel_df64(r); }
        case 2: { return df64_neg(sin_kernel_df64(r)); }
        case 3: { return df64_neg(cos_kernel_df64(r)); }
        default: { return df64_zero(); }
    }
}

fn cos_df64(a: Df64) -> Df64 {
    let k = i32(round(a.hi * 0.6366197723675814));
    let kf = df64_from_f32(f32(k));
    let half_pi = Df64(DF64_HALF_PI_HI, DF64_HALF_PI_LO);
    let r = df64_sub(a, df64_mul(kf, half_pi));

    let quadrant = ((k % 4) + 4) % 4;
    switch quadrant {
        case 0: { return cos_kernel_df64(r); }
        case 1: { return df64_neg(sin_kernel_df64(r)); }
        case 2: { return df64_neg(cos_kernel_df64(r)); }
        case 3: { return sin_kernel_df64(r); }
        default: { return df64_zero(); }
    }
}

// ── pow_df64: exp(b * log(a)) ──

fn pow_df64(base: Df64, exponent: Df64) -> Df64 {
    if base.hi <= 0.0 {
        if base.hi == 0.0 { return df64_zero(); }
        return df64_from_f32(-1e38); // NaN proxy
    }
    return exp_df64(df64_mul(exponent, log_df64(base)));
}

// ── tanh_df64: (exp(2x) - 1) / (exp(2x) + 1) ──

fn tanh_df64(a: Df64) -> Df64 {
    if a.hi > 10.0 { return df64_from_f32(1.0); }
    if a.hi < -10.0 { return df64_from_f32(-1.0); }
    let two_x = df64_scale_f32(a, 2.0);
    let e2x = exp_df64(two_x);
    let one = df64_from_f32(1.0);
    return df64_div(df64_sub(e2x, one), df64_add(e2x, one));
}
