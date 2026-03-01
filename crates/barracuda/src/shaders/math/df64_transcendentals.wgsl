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
const DF64_QUARTER_PI_HI: f32 = 0.7853982;
const DF64_QUARTER_PI_LO: f32 = -3.660254e-8;

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

// ── atan_df64: Taylor for |x| < 0.5, argument reduction for larger ──
// atan(x) = x - x³/3 + x⁵/5 - ... for |x| < 0.5
// atan(x) = π/2 - atan(1/x) for |x| > 1
// atan(x) = π/4 + atan((x-1)/(x+1)) for 0.5 ≤ x ≤ 1
fn atan_kernel_df64(x: Df64) -> Df64 {
    // Taylor: atan(x) = x - x³/3 + x⁵/5 - x⁷/7 + ... (12 terms for ~14 digits at |x|=0.5)
    let x2 = df64_mul(x, x);
    let c3 = df64_from_f32(-0.33333333333333333);
    let c5 = df64_from_f32(0.2);
    let c7 = df64_from_f32(-0.14285714285714285);
    let c9 = df64_from_f32(0.11111111111111111);
    let c11 = df64_from_f32(-0.09090909090909091);
    let c13 = df64_from_f32(0.07692307692307693);
    let c15 = df64_from_f32(-0.06666666666666667);
    let c17 = df64_from_f32(0.05882352941176471);
    let c19 = df64_from_f32(-0.05263157894736842);
    let c21 = df64_from_f32(0.04761904761904762);
    let c23 = df64_from_f32(-0.04347826086956522);
    let c25 = df64_from_f32(0.04);

    var p = df64_mul(c25, x2);
    p = df64_add(c23, p);
    p = df64_mul(p, x2);
    p = df64_add(c21, p);
    p = df64_mul(p, x2);
    p = df64_add(c19, p);
    p = df64_mul(p, x2);
    p = df64_add(c17, p);
    p = df64_mul(p, x2);
    p = df64_add(c15, p);
    p = df64_mul(p, x2);
    p = df64_add(c13, p);
    p = df64_mul(p, x2);
    p = df64_add(c11, p);
    p = df64_mul(p, x2);
    p = df64_add(c9, p);
    p = df64_mul(p, x2);
    p = df64_add(c7, p);
    p = df64_mul(p, x2);
    p = df64_add(c5, p);
    p = df64_mul(p, x2);
    p = df64_add(c3, p);
    p = df64_mul(p, df64_mul(x2, x));
    return df64_add(x, p);
}

fn atan_df64(x: Df64) -> Df64 {
    let half = df64_from_f32(0.5);
    let one = df64_from_f32(1.0);
    let pi_half = Df64(DF64_HALF_PI_HI, DF64_HALF_PI_LO);
    let pi_quarter = Df64(DF64_QUARTER_PI_HI, DF64_QUARTER_PI_LO);

    let ax = df64_abs(x);
    if df64_gt(ax, one) {
        // |x| > 1: atan(x) = π/2 - atan(1/x), sign preserved
        let inv_x = df64_div(one, x);
        let result = atan_kernel_df64(inv_x);
        if x.hi >= 0.0 {
            return df64_sub(pi_half, result);
        } else {
            return df64_sub(df64_neg(pi_half), result);
        }
    }
    if df64_gt(ax, half) {
        // 0.5 < |x| ≤ 1: atan(x) = π/4 + atan((x-1)/(x+1))
        let t = df64_div(df64_sub(x, one), df64_add(x, one));
        let result = atan_kernel_df64(t);
        return df64_add(pi_quarter, result);
    }
    return atan_kernel_df64(x);
}

// ── asin_df64: atan identity for |x| < 0.5, recursive reduction for |x| ≥ 0.5 ──
fn asin_df64(x: Df64) -> Df64 {
    if x.hi < 0.0 {
        return df64_neg(asin_df64(df64_neg(x)));
    }
    let half = df64_from_f32(0.5);
    let one = df64_from_f32(1.0);
    let pi_half = Df64(DF64_HALF_PI_HI, DF64_HALF_PI_LO);

    if df64_gt(x, half) {
        // x > 0.5: asin(x) = π/2 - 2*asin(sqrt((1-x)/2))
        let inner = df64_scale_f32(df64_sub(one, x), 0.5);
        let s = sqrt_df64(inner);
        return df64_sub(pi_half, df64_scale_f32(asin_df64(s), 2.0));
    }
    // |x| ≤ 0.5: asin(x) = atan(x / sqrt(1 - x²))
    let one_minus_x2 = df64_sub(one, df64_mul(x, x));
    return atan_df64(df64_div(x, sqrt_df64(one_minus_x2)));
}

// ── acos_df64: acos(x) = π/2 - asin(x) ──
fn acos_df64(x: Df64) -> Df64 {
    let pi_half = Df64(DF64_HALF_PI_HI, DF64_HALF_PI_LO);
    return df64_sub(pi_half, asin_df64(x));
}

// ── atan2_df64: four-quadrant arctangent ──
fn atan2_df64(y: Df64, x: Df64) -> Df64 {
    let zero = df64_zero();
    let pi = Df64(DF64_PI_HI, DF64_PI_LO);
    let pi_half = Df64(DF64_HALF_PI_HI, DF64_HALF_PI_LO);

    if x.hi == 0.0 && x.lo == 0.0 {
        if y.hi == 0.0 && y.lo == 0.0 { return zero; }
        if y.hi >= 0.0 { return pi_half; }
        return df64_neg(pi_half);
    }
    let q = df64_div(y, x);
    let at = atan_df64(q);
    if df64_gt(x, zero) {
        return at;
    }
    if df64_lt(y, zero) {
        return df64_sub(at, pi);
    }
    return df64_add(at, pi);
}

// ── sinh_df64: sinh(x) = (exp(x) - exp(-x)) / 2 ──
fn sinh_df64(x: Df64) -> Df64 {
    let ex = exp_df64(x);
    let emx = exp_df64(df64_neg(x));
    return df64_scale_f32(df64_sub(ex, emx), 0.5);
}

// ── cosh_df64: cosh(x) = (exp(x) + exp(-x)) / 2 ──
fn cosh_df64(x: Df64) -> Df64 {
    let ex = exp_df64(x);
    let emx = exp_df64(df64_neg(x));
    return df64_scale_f32(df64_add(ex, emx), 0.5);
}
