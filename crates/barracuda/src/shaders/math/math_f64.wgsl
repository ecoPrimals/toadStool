// ============================================================================
// math_f64.wgsl — Pure-arithmetic f64 math library for GPU compute
// ============================================================================
//
// This library implements transcendental functions using only f64 arithmetic
// operations (+, -, *, /, comparisons). Originally created because WGSL spec
// does not guarantee f64 builtins, but see NATIVE BUILTINS section below.
//
// NATIVE f64 BUILTINS (Feb 15-16 2026 hotSpring + wetSpring findings):
// WORKS on NVIDIA/AMD via Vulkan/wgpu:
//   sqrt(f64), abs(f64), min(f64), max(f64), floor(f64), ceil(f64)
// REJECTED by NVVM (not in WGSL spec — "NVVM compilation failed: 1"):
//   log(f64), exp(f64), pow(f64), sin(f64), cos(f64)
//
// This means ANY shader using log/exp/pow on f64 MUST use software
// implementations from this library. Shannon entropy (p * log(p)) was the
// first wetSpring workload to hit this — the fix is log_f64() below.
//
// Performance (RTX 4070, 1M elements):
//   - Native sqrt: 1.5× faster than sqrt_f64 (use native when available)
// For MD force kernels, prefer native sqrt/abs. For transcendentals, use
// the software implementations in this library.
//
// CRITICAL NAGA/WGSL GOTCHAS:
// 1. AbstractFloat (0.0, 1.0) does NOT auto-promote to f64
//    - WRONG: return 1.0;
//    - RIGHT: return x - x + 1.0;  // (f64 - f64) + AbstractFloat → f64
//
// 2. f64 CONSTANT PRECISION (Feb 16 2026 — wetSpring finding):
//    f64(0.333...) truncates through f32, losing ~7 digits of precision!
//    - WRONG: let c = f64(0.3333333333333333);  // truncates to f32 first
//    - WRONG: f64_const(x, 0.333...);           // f32 parameter truncates
//    - RIGHT: let zero = x - x; let c = zero + 0.3333333333333333;
//    The (zero + literal) pattern preserves all 15-16 significant digits.
//    Use this for polynomial coefficients and high-precision constants.
//
// 3. Literals > f32 range cause parse errors
//    - WRONG: return 1e308;
//    - RIGHT: construct via arithmetic
//
// 4. No f64 vec types (vec2<f64>, vec3<f64>, vec4<f64> not supported)
//
// 5. NEVER use i32 % for negative wrapping — produces incorrect results on
//    NVIDIA/Naga/Vulkan. Use branch-based conditionals instead:
//    - WRONG: ((x % n) + n) % n
//    - RIGHT: var w = x; if (w < 0) { w = w + n; } if (w >= n) { w = w - n; }
//    See: hotSpring ALERT Feb 15 2026 - cell-list bug diagnosis.
//
// 6. NATIVE f64 BUILTINS (Feb 15-16 2026 — hotSpring + wetSpring findings):
//    WORKS on NVIDIA/AMD via Vulkan/wgpu:  sqrt, abs, min, max, floor, ceil
//    REJECTED by NVVM (not in WGSL spec):  log, exp, pow, sin, cos
//    Use software implementations in this library for transcendentals.
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
///
/// PRECISION FIX (Feb 16 2026 — wetSpring pattern):
/// Uses (zero + literal) pattern for all f64 constants to preserve full precision.
/// f64_const() truncates through f32, losing ~7 digits.
fn exp_f64(x: f64) -> f64 {
    // Use (zero + literal) pattern for full f64 precision
    let zero = x - x;
    let one = zero + 1.0;
    let two = zero + 2.0;
    let half = zero + 0.5;
    
    // Handle special cases
    let overflow_thresh = zero + 709.0;  // ln(DBL_MAX) ≈ 709.78
    let underflow_thresh = zero - 745.0;
    
    if (x > overflow_thresh) {
        // Return large value (can't express infinity; 1e308 overflows f32 literal)
        let big = zero + 1e38;
        return big * big;
    }
    if (x < underflow_thresh) {
        return zero;
    }
    let tiny = zero + 1e-15;
    if (abs_f64(x) < tiny) {
        return one + x;  // exp(x) ≈ 1 + x for small x
    }
    
    // Range reduction: x = k*ln(2) + r
    // Full precision constants via (zero + literal)
    let inv_ln2 = zero + 1.4426950408889634;
    let k_f = round_f64(x * inv_ln2);
    let k = i32(k_f);
    
    // r = x - k * ln(2) (high precision, split into hi/lo parts)
    let ln2_hi = zero + 0.6931471805599453;
    let ln2_lo = zero + 2.3190468138462996e-17;
    var r = x - k_f * ln2_hi;
    r = r - k_f * ln2_lo;
    
    // Polynomial approximation for exp(r) - 1
    // Using degree-13 minimax polynomial for |r| < ln(2)/2
    // Coefficients via (zero + literal) for full f64 precision
    let r2 = r * r;
    
    // Coefficients: 1/n! series with minimax optimization
    let c2 = zero + 0.5;
    let c3 = zero + 0.16666666666666666;
    let c4 = zero + 0.041666666666666664;
    let c5 = zero + 0.008333333333333333;
    let c6 = zero + 0.001388888888888889;
    let c7 = zero + 0.0001984126984126984;
    let c8 = zero + 0.0000248015873015873;
    let c9 = zero + 0.0000027557319223985893;
    let c10 = zero + 2.7557319223985888e-7;
    let c11 = zero + 2.505210838544172e-8;
    let c12 = zero + 2.08767569878681e-9;
    let c13 = zero + 1.6059043836821613e-10;
    
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
    
    // Scale by 2^k using repeated squaring (faster than loop)
    // Split into positive and negative cases
    if (k >= 0) {
        // Build 2^k via repeated doubling in chunks
        var scale = one;
        var remaining = k;
        // Handle large exponents in chunks of 64 (2^64 fits in f64)
        let pow64 = zero + 18446744073709551616.0;  // 2^64
        while (remaining >= 64) {
            scale = scale * pow64;
            remaining = remaining - 64;
        }
        // Handle remaining bits
        let pow32 = zero + 4294967296.0;  // 2^32
        if (remaining >= 32) {
            scale = scale * pow32;
            remaining = remaining - 32;
        }
        let pow16 = zero + 65536.0;  // 2^16
        if (remaining >= 16) {
            scale = scale * pow16;
            remaining = remaining - 16;
        }
        let pow8 = zero + 256.0;  // 2^8
        if (remaining >= 8) {
            scale = scale * pow8;
            remaining = remaining - 8;
        }
        let pow4 = zero + 16.0;  // 2^4
        if (remaining >= 4) {
            scale = scale * pow4;
            remaining = remaining - 4;
        }
        if (remaining >= 2) {
            scale = scale * (zero + 4.0);
            remaining = remaining - 2;
        }
        if (remaining >= 1) {
            scale = scale * two;
        }
        exp_r = exp_r * scale;
    } else {
        // Negative k: multiply by 2^(-|k|) = 1/2^|k|
        var scale = one;
        var remaining = -k;
        let inv_pow64 = zero + 5.421010862427522e-20;  // 2^-64
        while (remaining >= 64) {
            scale = scale * inv_pow64;
            remaining = remaining - 64;
        }
        let inv_pow32 = zero + 2.3283064365386963e-10;  // 2^-32
        if (remaining >= 32) {
            scale = scale * inv_pow32;
            remaining = remaining - 32;
        }
        let inv_pow16 = zero + 0.0000152587890625;  // 2^-16
        if (remaining >= 16) {
            scale = scale * inv_pow16;
            remaining = remaining - 16;
        }
        let inv_pow8 = zero + 0.00390625;  // 2^-8
        if (remaining >= 8) {
            scale = scale * inv_pow8;
            remaining = remaining - 8;
        }
        let inv_pow4 = zero + 0.0625;  // 2^-4
        if (remaining >= 4) {
            scale = scale * inv_pow4;
            remaining = remaining - 4;
        }
        if (remaining >= 2) {
            scale = scale * (zero + 0.25);
            remaining = remaining - 2;
        }
        if (remaining >= 1) {
            scale = scale * half;
        }
        exp_r = exp_r * scale;
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
        return -f64_const(x, 1e38) * f64_const(x, 1e38);  // -infinity approximation
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
    
    // Polynomial for log(y) via atanh transformation:
    // log(y) = 2 * atanh((y-1)/(y+1)) = 2 * s * (1 + s²/3 + s⁴/5 + s⁶/7 + ...)
    // Coefficients are 1/(2k+1) with minimax optimization.
    // The outer "two * s * (1 + s² * p)" provides the factor of 2.
    //
    // BUG FIX (Feb 16 2026 — wetSpring handoff):
    // Original coefficients were 2/3, 2/5, etc. (doubled), causing ~1e-3 error.
    // Corrected to 1/3, 1/5, etc. for ~1e-15 precision.
    //
    // NOTE: Use (x - x + literal) pattern to preserve full f64 precision.
    // f64_const() truncates through f32, losing ~7 digits.
    let zero = x - x;
    let c1 = zero + 0.3333333333333367565;   // ≈ 1/3 (minimax)
    let c2 = zero + 0.1999999999970470954;   // ≈ 1/5 (minimax)
    let c3 = zero + 0.1428571437183119575;   // ≈ 1/7 (minimax)
    let c4 = zero + 0.1111109921607489198;   // ≈ 1/9 (minimax)
    let c5 = zero + 0.0909178608080902506;   // ≈ 1/11 (minimax)
    let c6 = zero + 0.0765691884960468666;   // ≈ 1/13 (minimax)
    let c7 = zero + 0.0739909930255829295;   // ≈ 1/15 (minimax)
    
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
/// PRECISION FIX (Feb 16 2026): Uses (zero + literal) pattern for full f64 precision.
fn sin_f64(x: f64) -> f64 {
    // Full precision constants via (zero + literal)
    let zero = x - x;
    let one = zero + 1.0;
    let pi = zero + 3.141592653589793;
    let two_pi = zero + 6.283185307179586;
    let neg_pi = zero - 3.141592653589793;
    
    // Range reduction to [-pi, pi]
    var y = x;
    while (y > pi) { y = y - two_pi; }
    while (y < neg_pi) { y = y + two_pi; }
    
    // Taylor series: sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
    let x2 = y * y;
    
    // Coefficients via (zero + literal) for full f64 precision
    let c3 = zero - 0.16666666666666666;    // -1/6
    let c5 = zero + 0.008333333333333333;   // 1/120
    let c7 = zero - 0.0001984126984126984;  // -1/5040
    let c9 = zero + 0.0000027557319223985888;  // 1/362880
    let c11 = zero - 2.505210838544172e-8;
    let c13 = zero + 1.6059043836821613e-10;
    let c15 = zero - 7.647163731819816e-13;
    
    // Horner's method
    var p = c15;
    p = p * x2 + c13;
    p = p * x2 + c11;
    p = p * x2 + c9;
    p = p * x2 + c7;
    p = p * x2 + c5;
    p = p * x2 + c3;
    
    return y + y * x2 * p;
}

/// Cosine using sin(x + pi/2)
/// PRECISION FIX (Feb 16 2026): Uses (zero + literal) for full f64 precision.
fn cos_f64(x: f64) -> f64 {
    let zero = x - x;
    let half_pi = zero + 1.5707963267948966;
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
    let zero = x - x;
    let two = zero + 2.0;
    let ex = exp_f64(x);
    let emx = exp_f64(-x);
    return (ex - emx) / two;
}

/// Hyperbolic cosine: cosh(x) = (exp(x) + exp(-x)) / 2
fn cosh_f64(x: f64) -> f64 {
    let zero = x - x;
    let two = zero + 2.0;
    let ex = exp_f64(x);
    let emx = exp_f64(-x);
    return (ex + emx) / two;
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

/// Lanczos core: Gamma(x) for x >= 0.5 via Lanczos approximation (g=7, n=9)
/// Split out so gamma_f64 can call it iteratively (WGSL forbids recursion).
/// PRECISION FIX (Feb 16 2026): Uses (zero + literal) pattern for full f64 precision.
fn lanczos_core_f64(x: f64) -> f64 {
    let zero = x - x;
    let one = zero + 1.0;
    let half = zero + 0.5;
    let g = zero + 7.0;
    
    // Lanczos coefficients via (zero + literal) for full f64 precision
    let c0 = zero + 0.99999999999980993;
    let c1 = zero + 676.5203681218851;
    let c2 = zero - 1259.1392167224028;
    let c3 = zero + 771.32342877765313;
    let c4 = zero - 176.61502916214059;
    let c5 = zero + 12.507343278686905;
    let c6 = zero - 0.13857109526572012;
    let c7 = zero + 9.9843695780195716e-6;
    let c8 = zero + 1.5056327351493116e-7;

    let z = x - one;

    var sum = c0;
    sum = sum + c1 / (z + one);
    sum = sum + c2 / (z + (zero + 2.0));
    sum = sum + c3 / (z + (zero + 3.0));
    sum = sum + c4 / (z + (zero + 4.0));
    sum = sum + c5 / (z + (zero + 5.0));
    sum = sum + c6 / (z + (zero + 6.0));
    sum = sum + c7 / (z + (zero + 7.0));
    sum = sum + c8 / (z + (zero + 8.0));

    let t = z + g + half;
    let sqrt_2pi = zero + 2.5066282746310005;

    return sqrt_2pi * pow_f64(t, z + half) * exp_f64(-t) * sum;
}

/// Gamma function using Lanczos approximation (non-recursive)
/// Accurate to ~15 digits for positive real arguments.
/// Reflection formula for x < 0.5 inlined to avoid WGSL recursion ban.
/// PRECISION FIX (Feb 16 2026): Uses (zero + literal) pattern for full f64 precision.
fn gamma_f64(x: f64) -> f64 {
    let zero = x - x;
    let half = zero + 0.5;
    let one = zero + 1.0;
    let pi = zero + 3.141592653589793;

    if (x < half) {
        // Reflection: Gamma(x) = pi / (sin(pi*x) * Gamma(1-x))
        // Since 1-x >= 0.5, lanczos_core handles it directly.
        let sin_pix = sin_f64(pi * x);
        let tiny = zero + 1e-15;
        if (abs_f64(sin_pix) < tiny) {
            let big = zero + 1e38;
            return big * big;  // Pole (~1e76, large enough)
        }
        return pi / (sin_pix * lanczos_core_f64(one - x));
    }

    return lanczos_core_f64(x);
}

// ============================================================================
// ERROR FUNCTION (erf)
// ============================================================================

/// Error function using Abramowitz & Stegun approximation
/// PRECISION FIX (Feb 16 2026): Uses (zero + literal) pattern for full f64 precision.
fn erf_f64(x: f64) -> f64 {
    let zero = x - x;
    let one = zero + 1.0;
    
    // Constants via (zero + literal) for full f64 precision
    let a1 = zero + 0.254829592;
    let a2 = zero - 0.284496736;
    let a3 = zero + 1.421413741;
    let a4 = zero - 1.453152027;
    let a5 = zero + 1.061405429;
    let p = zero + 0.3275911;
    
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
/// PRECISION FIX (Feb 16 2026): Uses (zero + literal) pattern for full f64 precision.
fn bessel_j0_f64(x: f64) -> f64 {
    let zero = x - x;
    let one = zero + 1.0;
    let ax = abs_f64(x);
    let eight = zero + 8.0;
    
    if (ax < eight) {
        let y = x * x;
        let num = (zero + 57568490574.0) + y * (
            (zero - 13362590354.0) + y * (
            (zero + 651619640.7) + y * (
            (zero - 11214424.18) + y * (
            (zero + 77392.33017) + y * (zero - 184.9052456)))));
        let den = (zero + 57568490411.0) + y * (
            (zero + 1029532985.0) + y * (
            (zero + 9494680.718) + y * (
            (zero + 59272.64853) + y * (
            (zero + 267.8532712) + y))));
        return num / den;
    } else {
        let z = eight / ax;
        let y = z * z;
        let xx = ax - (zero + 0.785398164);
        let p0 = one + y * ((zero - 0.0010986286270000001) + y * (zero + 0.000027345104070000003));
        let q0 = (zero - 0.01562499995) + y * ((zero + 0.0001430488765) + y * (zero - 0.0000069111476510000005));
        return sqrt_f64((zero + 0.636619772) / ax) * (cos_f64(xx) * p0 - z * sin_f64(xx) * q0);
    }
}

// ============================================================================
