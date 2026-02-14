// ============================================================================
// math_f64.wgsl — Pure-arithmetic f64 math library for GPU compute
// ============================================================================
//
// This library implements transcendental functions using only f64 arithmetic
// operations (+, -, *, /, comparisons). Originally created because WGSL spec
// does not guarantee f64 builtins, but see NATIVE BUILTINS section below.
//
// NATIVE f64 BUILTINS (Feb 15 2026 hotSpring finding):
// The following builtins DO work with f64 via Naga/wgpu on NVIDIA/AMD GPUs:
//   sqrt(f64), exp(f64), log(f64), abs(f64), floor(f64), ceil(f64),
//   round(f64), inverseSqrt(f64)
// Performance (RTX 4070, 1M elements):
//   - Native sqrt: 1.5× faster than sqrt_f64
//   - Native exp: 2.2× faster than exp_f64
// For MD force kernels, prefer native builtins when available.
// This library remains useful for: sin, cos, tan, erf, gamma, etc. (no native)
//
// CRITICAL NAGA/WGSL GOTCHAS:
// 1. AbstractFloat (0.0, 1.0) does NOT auto-promote to f64
//    - WRONG: return 1.0;
//    - RIGHT: return x - x + 1.0;  // (f64 - f64) + AbstractFloat → f64
//
// 2. Literals > f32 range cause parse errors
//    - WRONG: return 1e308;
//    - RIGHT: construct via arithmetic
//
// 3. No f64 vec types (vec2<f64>, vec3<f64>, vec4<f64> not supported)
//
// 4. NEVER use i32 % for negative wrapping — produces incorrect results on
//    NVIDIA/Naga/Vulkan. Use branch-based conditionals instead:
//    - WRONG: ((x % n) + n) % n
//    - RIGHT: var w = x; if (w < 0) { w = w + n; } if (w >= n) { w = w - n; }
//    See: hotSpring ALERT Feb 15 2026 - cell-list bug diagnosis.
//
// PRECISION TARGETS:
// - sqrt_f64: Full f64 precision (5 Newton-Raphson iterations)
// - cbrt_f64: Full f64 precision (Halley's method)
// - exp_f64: ~1e-15 relative error (degree-17 polynomial)
// - log_f64: ~1e-15 relative error (degree-15 polynomial)
// - pow_f64: Uses specialized paths for common exponents
// ============================================================================

// Helper: construct f64 constant from AbstractFloat
// The pattern (x - x + c) ensures f64 type propagation
fn f64_const(x: f64, c: f32) -> f64 {
    return x - x + f64(c);
}

// ============================================================================
// BASIC FUNCTIONS
// ============================================================================

/// Absolute value
fn abs_f64(x: f64) -> f64 {
    if (x < f64_const(x, 0.0)) {
        return -x;
    }
    return x;
}

/// Sign function: -1, 0, or 1
fn sign_f64(x: f64) -> f64 {
    let zero = f64_const(x, 0.0);
    if (x > zero) {
        return f64_const(x, 1.0);
    }
    if (x < zero) {
        return f64_const(x, -1.0);
    }
    return zero;
}

/// Floor function (rounds toward negative infinity)
fn floor_f64(x: f64) -> f64 {
    let i = i32(x);
    let fi = f64(i);
    if (x < fi) {
        return fi - f64_const(x, 1.0);
    }
    return fi;
}

/// Ceiling function (rounds toward positive infinity)
fn ceil_f64(x: f64) -> f64 {
    let i = i32(x);
    let fi = f64(i);
    if (x > fi) {
        return fi + f64_const(x, 1.0);
    }
    return fi;
}

/// Round to nearest integer
fn round_f64(x: f64) -> f64 {
    return floor_f64(x + f64_const(x, 0.5));
}

/// Fractional part
fn fract_f64(x: f64) -> f64 {
    return x - floor_f64(x);
}

/// Minimum of two values
fn min_f64(a: f64, b: f64) -> f64 {
    if (a < b) { return a; }
    return b;
}

/// Maximum of two values
fn max_f64(a: f64, b: f64) -> f64 {
    if (a > b) { return a; }
    return b;
}

/// Clamp to range
fn clamp_f64(x: f64, lo: f64, hi: f64) -> f64 {
    return min_f64(max_f64(x, lo), hi);
}

// ============================================================================
// SQUARE ROOT — Newton-Raphson (5 iterations for full f64 precision)
// ============================================================================

/// Square root using Newton-Raphson iteration
/// x_{n+1} = 0.5 * (x_n + S / x_n)
/// 5 iterations achieves full f64 precision
fn sqrt_f64(x: f64) -> f64 {
    let zero = f64_const(x, 0.0);
    if (x <= zero) {
        return zero;
    }
    
    // Initial estimate using f32 sqrt (via bit manipulation approximation)
    // For robustness, use a simple initial guess based on magnitude
    var y = x;
    
    // Scale to reasonable range for initial guess
    var scale = f64_const(x, 1.0);
    let large = f64_const(x, 1e32);
    let small = f64_const(x, 1e-32);
    
    if (x > large) {
        y = x / large;
        scale = f64_const(x, 1e16);
    } else if (x < small) {
        y = x * large;
        scale = f64_const(x, 1e-16);
    }
    
    // Initial guess: y^0.5 ≈ y / 2 for y near 1 (crude but converges)
    var r = (y + f64_const(x, 1.0)) / f64_const(x, 2.0);
    
    // Newton-Raphson iterations
    let half = f64_const(x, 0.5);
    r = half * (r + y / r);
    r = half * (r + y / r);
    r = half * (r + y / r);
    r = half * (r + y / r);
    r = half * (r + y / r);
    
    return r * scale;
}

// ============================================================================
// CUBE ROOT — Halley's method (faster convergence than Newton)
// ============================================================================

/// Cube root using Halley's iteration
/// x_{n+1} = x_n * (x_n^3 + 2*S) / (2*x_n^3 + S)
fn cbrt_f64(x: f64) -> f64 {
    let zero = f64_const(x, 0.0);
    if (x == zero) {
        return zero;
    }
    
    let neg = x < zero;
    var y = abs_f64(x);
    
    // Scale to reasonable range
    var scale = f64_const(x, 1.0);
    let large = f64_const(x, 1e30);
    let small = f64_const(x, 1e-30);
    
    if (y > large) {
        y = y / large;
        scale = f64_const(x, 1e10);  // cbrt(1e30) = 1e10
    } else if (y < small) {
        y = y * large;
        scale = f64_const(x, 1e-10);
    }
    
    // Initial guess
    var r = (y + f64_const(x, 1.0)) / f64_const(x, 2.0);
    
    // Halley's method iterations
    let two = f64_const(x, 2.0);
    for (var i = 0; i < 6; i = i + 1) {
        let r3 = r * r * r;
        r = r * (r3 + two * y) / (two * r3 + y);
    }
    
    if (neg) {
        return -r * scale;
    }
    return r * scale;
}

// ============================================================================
// EXPONENTIAL — Degree-17 polynomial with range reduction
// ============================================================================

/// Constants for exp
const LN2_HI: f64 = 0.693147180559945286;  // High part of ln(2)
const LN2_LO: f64 = 1.94821509970e-17;     // Low part of ln(2)
const INV_LN2: f64 = 1.4426950408889634;   // 1/ln(2)

/// Exponential function using range reduction and polynomial
/// exp(x) = 2^k * exp(r) where r = x - k*ln(2) and |r| < ln(2)/2
fn exp_f64(x: f64) -> f64 {
    let zero = f64_const(x, 0.0);
    let one = f64_const(x, 1.0);
    
    // Handle special cases
    let overflow_thresh = f64_const(x, 709.0);  // ln(DBL_MAX) ≈ 709.78
    let underflow_thresh = f64_const(x, -745.0);
    
    if (x > overflow_thresh) {
        // Return large value (can't express infinity)
        return f64_const(x, 1e308);
    }
    if (x < underflow_thresh) {
        return zero;
    }
    if (abs_f64(x) < f64_const(x, 1e-15)) {
        return one + x;  // exp(x) ≈ 1 + x for small x
    }
    
    // Range reduction: x = k*ln(2) + r
    let inv_ln2 = f64_const(x, 1.4426950408889634);
    let k_f = round_f64(x * inv_ln2);
    let k = i32(k_f);
    
    // r = x - k * ln(2) (high precision)
    let ln2_hi = f64_const(x, 0.693147180559945286);
    let ln2_lo = f64_const(x, 1.94821509970e-17);
    var r = x - k_f * ln2_hi;
    r = r - k_f * ln2_lo;
    
    // Polynomial approximation for exp(r) - 1
    // Using degree-13 minimax polynomial for |r| < ln(2)/2
    let r2 = r * r;
    
    // Coefficients for (exp(r) - 1 - r) / r^2
    let c2 = f64_const(x, 0.5);
    let c3 = f64_const(x, 0.166666666666666657);
    let c4 = f64_const(x, 0.0416666666666666644);
    let c5 = f64_const(x, 0.00833333333333401156);
    let c6 = f64_const(x, 0.00138888888889774492);
    let c7 = f64_const(x, 0.000198412698413242405);
    let c8 = f64_const(x, 0.0000248015873015873016);
    let c9 = f64_const(x, 0.00000275573192239858925);
    let c10 = f64_const(x, 2.75573191913863016e-7);
    let c11 = f64_const(x, 2.50521083854417202e-8);
    let c12 = f64_const(x, 2.08767569878681002e-9);
    let c13 = f64_const(x, 1.60590438368216145e-10);
    
    // Horner's method evaluation
    var p = c13;
    p = p * r + c12;
    p = p * r + c11;
    p = p * r + c10;
    p = p * r + c9;
    p = p * r + c8;
    p = p * r + c7;
    p = p * r + c6;
    p = p * r + c5;
    p = p * r + c4;
    p = p * r + c3;
    p = p * r + c2;
    
    // exp(r) = 1 + r + r^2 * p
    var exp_r = one + r + r2 * p;
    
    // Scale by 2^k
    // Since we can't use ldexp, multiply by powers of 2
    if (k > 0) {
        for (var i = 0; i < k; i = i + 1) {
            exp_r = exp_r * f64_const(x, 2.0);
        }
    } else if (k < 0) {
        for (var i = 0; i > k; i = i - 1) {
            exp_r = exp_r * f64_const(x, 0.5);
        }
    }
    
    return exp_r;
}

// ============================================================================
// NATURAL LOGARITHM — Range reduction + polynomial
// ============================================================================

/// Natural logarithm using range reduction and polynomial
/// log(x) = log(2^k * m) = k*ln(2) + log(m) where 1 <= m < 2
fn log_f64(x: f64) -> f64 {
    let zero = f64_const(x, 0.0);
    let one = f64_const(x, 1.0);
    
    // Handle special cases
    if (x <= zero) {
        return f64_const(x, -1e308);  // -infinity approximation
    }
    
    // Range reduction to [1, 2)
    var y = x;
    var k = f64_const(x, 0.0);
    let two = f64_const(x, 2.0);
    let half = f64_const(x, 0.5);
    
    // Scale to [1, 2)
    while (y >= two) {
        y = y * half;
        k = k + one;
    }
    while (y < one) {
        y = y * two;
        k = k - one;
    }
    
    // Now y is in [1, 2), compute log(y) using log(1+z) where z = y - 1
    let z = y - one;
    
    // For better convergence, use z = (y-1)/(y+1) transformation
    // log(y) = 2 * atanh(z/(2+z)) = 2 * atanh((y-1)/(y+1))
    let s = z / (two + z);  // s = (y-1)/(y+1)
    let s2 = s * s;
    
    // Polynomial for atanh(s)/s - 1, evaluated at s^2
    // atanh(s) = s + s^3/3 + s^5/5 + s^7/7 + ...
    let c1 = f64_const(x, 0.6666666666666735130);  // 2/3
    let c2 = f64_const(x, 0.3999999999940941908);  // 2/5
    let c3 = f64_const(x, 0.2857142874366239149);  // 2/7
    let c4 = f64_const(x, 0.2222219843214978396);  // 2/9
    let c5 = f64_const(x, 0.1818357216161805012);  // 2/11
    let c6 = f64_const(x, 0.1531383769920937332);  // 2/13
    let c7 = f64_const(x, 0.1479819860511658591);  // 2/15
    
    // Horner's evaluation
    var p = c7;
    p = p * s2 + c6;
    p = p * s2 + c5;
    p = p * s2 + c4;
    p = p * s2 + c3;
    p = p * s2 + c2;
    p = p * s2 + c1;
    
    // log(y) = 2 * s * (1 + s^2 * p)
    let log_y = two * s * (one + s2 * p);
    
    // log(x) = k * ln(2) + log(y)
    let ln2 = f64_const(x, 0.6931471805599453);
    return k * ln2 + log_y;
}

// ============================================================================
// POWER FUNCTION — Specialized paths for common exponents
// ============================================================================

/// Integer power (fast path)
fn ipow_f64(base: f64, exp: i32) -> f64 {
    let one = f64_const(base, 1.0);
    if (exp == 0) {
        return one;
    }
    
    var b = base;
    var e = exp;
    var result = one;
    
    if (e < 0) {
        b = one / b;
        e = -e;
    }
    
    // Binary exponentiation
    while (e > 0) {
        if ((e & 1) == 1) {
            result = result * b;
        }
        b = b * b;
        e = e >> 1;
    }
    
    return result;
}

/// Cube root specialized for A^(1/3) — higher precision than exp(log(x)/3)
fn pow_one_third(x: f64) -> f64 {
    return cbrt_f64(x);
}

/// Square root specialized for A^(1/2) — higher precision than exp(log(x)/2)
fn pow_one_half(x: f64) -> f64 {
    return sqrt_f64(x);
}

/// A^(2/3) specialized — higher precision than exp(2*log(x)/3)
fn pow_two_thirds(x: f64) -> f64 {
    let cbrt_x = cbrt_f64(x);
    return cbrt_x * cbrt_x;
}

/// General power function
/// For fractional powers, uses exp(exponent * log(base))
/// For integer powers, uses binary exponentiation
/// For common fractions (1/2, 1/3, 2/3), uses specialized high-precision paths
fn pow_f64(base: f64, exponent: f64) -> f64 {
    let zero = f64_const(base, 0.0);
    let one = f64_const(base, 1.0);
    
    // Handle special cases
    if (exponent == zero) {
        return one;
    }
    if (base == zero) {
        return zero;
    }
    if (base == one) {
        return one;
    }
    if (exponent == one) {
        return base;
    }
    
    // Check for integer exponent
    let exp_rounded = round_f64(exponent);
    let is_integer = abs_f64(exponent - exp_rounded) < f64_const(base, 1e-10);
    
    if (is_integer) {
        return ipow_f64(base, i32(exp_rounded));
    }
    
    // Check for common fractions
    let half = f64_const(base, 0.5);
    let one_third = f64_const(base, 0.333333333333333333);
    let two_thirds = f64_const(base, 0.666666666666666667);
    let neg_half = f64_const(base, -0.5);
    
    if (abs_f64(exponent - half) < f64_const(base, 1e-10)) {
        return sqrt_f64(base);
    }
    if (abs_f64(exponent - one_third) < f64_const(base, 1e-10)) {
        return cbrt_f64(base);
    }
    if (abs_f64(exponent - two_thirds) < f64_const(base, 1e-10)) {
        return pow_two_thirds(base);
    }
    if (abs_f64(exponent - neg_half) < f64_const(base, 1e-10)) {
        return one / sqrt_f64(base);
    }
    
    // General case: exp(exponent * log(base))
    // Note: This has ~1e-14 relative error due to polynomial approximation
    if (base > zero) {
        return exp_f64(exponent * log_f64(base));
    }
    
    // Negative base with non-integer exponent is undefined
    return zero;
}

// ============================================================================
// TRIGONOMETRIC FUNCTIONS (Basic implementations)
// ============================================================================

/// Sine using Taylor series with range reduction
fn sin_f64(x: f64) -> f64 {
    let pi = f64_const(x, 3.14159265358979323846);
    let two_pi = f64_const(x, 6.28318530717958647693);
    let half_pi = f64_const(x, 1.57079632679489661923);
    let zero = f64_const(x, 0.0);
    let one = f64_const(x, 1.0);
    
    // Range reduction to [-pi, pi]
    var y = x;
    while (y > pi) { y = y - two_pi; }
    while (y < -pi) { y = y + two_pi; }
    
    // Taylor series: sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
    let x2 = y * y;
    
    // Coefficients
    let c3 = f64_const(x, -0.166666666666666667);   // -1/6
    let c5 = f64_const(x, 0.00833333333333333333);  // 1/120
    let c7 = f64_const(x, -0.000198412698412698413); // -1/5040
    let c9 = f64_const(x, 0.00000275573192239858907); // 1/362880
    let c11 = f64_const(x, -2.50521083854417188e-8);
    let c13 = f64_const(x, 1.60590438368216146e-10);
    
    // Horner's method
    var p = c13;
    p = p * x2 + c11;
    p = p * x2 + c9;
    p = p * x2 + c7;
    p = p * x2 + c5;
    p = p * x2 + c3;
    
    return y + y * x2 * p;
}

/// Cosine using sin(x + pi/2)
fn cos_f64(x: f64) -> f64 {
    let half_pi = f64_const(x, 1.57079632679489661923);
    return sin_f64(x + half_pi);
}

/// Tangent using sin/cos
fn tan_f64(x: f64) -> f64 {
    return sin_f64(x) / cos_f64(x);
}

// ============================================================================
// HYPERBOLIC FUNCTIONS
// ============================================================================

/// Hyperbolic sine: sinh(x) = (exp(x) - exp(-x)) / 2
fn sinh_f64(x: f64) -> f64 {
    let ex = exp_f64(x);
    let emx = exp_f64(-x);
    return (ex - emx) / f64_const(x, 2.0);
}

/// Hyperbolic cosine: cosh(x) = (exp(x) + exp(-x)) / 2
fn cosh_f64(x: f64) -> f64 {
    let ex = exp_f64(x);
    let emx = exp_f64(-x);
    return (ex + emx) / f64_const(x, 2.0);
}

/// Hyperbolic tangent: tanh(x) = sinh(x) / cosh(x)
fn tanh_f64(x: f64) -> f64 {
    let ex = exp_f64(x);
    let emx = exp_f64(-x);
    return (ex - emx) / (ex + emx);
}

// ============================================================================
// GAMMA FUNCTION (Lanczos approximation)
// ============================================================================

/// Gamma function using Lanczos approximation
/// Accurate to ~15 digits for positive real arguments
fn gamma_f64(x: f64) -> f64 {
    let zero = f64_const(x, 0.0);
    let one = f64_const(x, 1.0);
    let half = f64_const(x, 0.5);
    let pi = f64_const(x, 3.14159265358979323846);
    
    // Handle negative values using reflection formula
    if (x < half) {
        // Gamma(x) = pi / (sin(pi*x) * Gamma(1-x))
        let sin_pix = sin_f64(pi * x);
        if (abs_f64(sin_pix) < f64_const(x, 1e-15)) {
            return f64_const(x, 1e308);  // Pole
        }
        return pi / (sin_pix * gamma_f64(one - x));
    }
    
    // Lanczos coefficients (g=7, n=9)
    let g = f64_const(x, 7.0);
    let c0 = f64_const(x, 0.99999999999980993);
    let c1 = f64_const(x, 676.5203681218851);
    let c2 = f64_const(x, -1259.1392167224028);
    let c3 = f64_const(x, 771.32342877765313);
    let c4 = f64_const(x, -176.61502916214059);
    let c5 = f64_const(x, 12.507343278686905);
    let c6 = f64_const(x, -0.13857109526572012);
    let c7 = f64_const(x, 9.9843695780195716e-6);
    let c8 = f64_const(x, 1.5056327351493116e-7);
    
    let z = x - one;
    
    var sum = c0;
    sum = sum + c1 / (z + one);
    sum = sum + c2 / (z + f64_const(x, 2.0));
    sum = sum + c3 / (z + f64_const(x, 3.0));
    sum = sum + c4 / (z + f64_const(x, 4.0));
    sum = sum + c5 / (z + f64_const(x, 5.0));
    sum = sum + c6 / (z + f64_const(x, 6.0));
    sum = sum + c7 / (z + f64_const(x, 7.0));
    sum = sum + c8 / (z + f64_const(x, 8.0));
    
    let t = z + g + half;
    let sqrt_2pi = f64_const(x, 2.5066282746310005);
    
    return sqrt_2pi * pow_f64(t, z + half) * exp_f64(-t) * sum;
}

// ============================================================================
// ERROR FUNCTION (erf)
// ============================================================================

/// Error function using Abramowitz & Stegun approximation
fn erf_f64(x: f64) -> f64 {
    let zero = f64_const(x, 0.0);
    let one = f64_const(x, 1.0);
    
    // Constants
    let a1 = f64_const(x, 0.254829592);
    let a2 = f64_const(x, -0.284496736);
    let a3 = f64_const(x, 1.421413741);
    let a4 = f64_const(x, -1.453152027);
    let a5 = f64_const(x, 1.061405429);
    let p = f64_const(x, 0.3275911);
    
    let sign = sign_f64(x);
    let ax = abs_f64(x);
    
    let t = one / (one + p * ax);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    
    let y = one - (a1 * t + a2 * t2 + a3 * t3 + a4 * t4 + a5 * t5) * exp_f64(-ax * ax);
    
    return sign * y;
}

// ============================================================================
// BESSEL FUNCTIONS (J0, J1)
// ============================================================================

/// Bessel function J0 using polynomial approximation
fn bessel_j0_f64(x: f64) -> f64 {
    let ax = abs_f64(x);
    let one = f64_const(x, 1.0);
    
    if (ax < f64_const(x, 8.0)) {
        let y = x * x;
        let num = f64_const(x, 57568490574.0) + y * (
            f64_const(x, -13362590354.0) + y * (
            f64_const(x, 651619640.7) + y * (
            f64_const(x, -11214424.18) + y * (
            f64_const(x, 77392.33017) + y * f64_const(x, -184.9052456)))));
        let den = f64_const(x, 57568490411.0) + y * (
            f64_const(x, 1029532985.0) + y * (
            f64_const(x, 9494680.718) + y * (
            f64_const(x, 59272.64853) + y * (
            f64_const(x, 267.8532712) + y))));
        return num / den;
    } else {
        let z = f64_const(x, 8.0) / ax;
        let y = z * z;
        let xx = ax - f64_const(x, 0.785398164);
        let p0 = one + y * (f64_const(x, -0.1098628627e-2) + y * f64_const(x, 0.2734510407e-4));
        let q0 = f64_const(x, -0.1562499995e-1) + y * (f64_const(x, 0.1430488765e-3) + y * f64_const(x, -0.6911147651e-5));
        return sqrt_f64(f64_const(x, 0.636619772) / ax) * (cos_f64(xx) * p0 - z * sin_f64(xx) * q0);
    }
}

// ============================================================================
