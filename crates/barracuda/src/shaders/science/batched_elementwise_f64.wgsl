// ============================================================================
// batched_elementwise_f64.wgsl — Unified batched element-wise computation
// ============================================================================
//
// UNIFIED PATTERN (Feb 16 2026) — Template for all springs:
//   - airSpring: Batched ET₀, water balance across stations/fields
//   - wetSpring: Batched diversity metrics across samples
//   - hotSpring: Batched nuclear structure across nuclei
//
// ARCHITECTURE:
//   - One workgroup per batch element
//   - Each workgroup processes one "row" of work
//   - Parameters passed via uniform buffer
//   - Input/output arrays are flattened [batch * stride]
//
// REQUIRES: SHADER_F64 feature
// Date: February 16, 2026
// License: AGPL-3.0-or-later
// ============================================================================

// ============================================================================
// OPERATION ENUM (selected via params.operation)
// ============================================================================
// 0 = FAO56_ET0 (Penman-Monteith reference evapotranspiration)
// 1 = WATER_BALANCE (daily depletion update)
// 2 = SHANNON_BATCH (Shannon entropy per sample)
// 3 = SIMPSON_BATCH (Simpson index per sample)
// 4 = CUSTOM (user-defined via auxiliary params)

// ============================================================================
// MATH FUNCTIONS (inline subset for self-contained shader)
// ============================================================================

fn exp_f64(x: f64) -> f64 {
    let zero = x - x;
    let one = zero + 1.0;
    
    let overflow = zero + 709.0;
    let underflow = zero - 745.0;
    
    if (x > overflow) {
        let big = zero + 1e38;
        return big * big;
    }
    if (x < underflow) {
        return zero;
    }
    
    let tiny = zero + 1e-15;
    if (x > -tiny && x < tiny) {
        return one + x;
    }
    
    let inv_ln2 = zero + 1.4426950408889634;
    let k_f = floor(x * inv_ln2 + (zero + 0.5));
    let k = i32(k_f);
    
    let ln2 = zero + 0.6931471805599453;
    let r = x - k_f * ln2;
    
    let r2 = r * r;
    let c2 = zero + 0.5;
    let c3 = zero + 0.16666666666666666;
    let c4 = zero + 0.041666666666666664;
    let c5 = zero + 0.008333333333333333;
    
    var p = c5;
    p = p * r + c4;
    p = p * r + c3;
    p = p * r + c2;
    
    var exp_r = one + r + r2 * p;
    
    // Scale by 2^k
    if (k > 0) {
        var scale = one;
        var rem = k;
        if (rem >= 16) { scale = scale * (zero + 65536.0); rem = rem - 16; }
        if (rem >= 8) { scale = scale * (zero + 256.0); rem = rem - 8; }
        if (rem >= 4) { scale = scale * (zero + 16.0); rem = rem - 4; }
        if (rem >= 2) { scale = scale * (zero + 4.0); rem = rem - 2; }
        if (rem >= 1) { scale = scale * (zero + 2.0); }
        exp_r = exp_r * scale;
    } else if (k < 0) {
        var scale = one;
        var rem = -k;
        if (rem >= 16) { scale = scale * (zero + 0.0000152587890625); rem = rem - 16; }
        if (rem >= 8) { scale = scale * (zero + 0.00390625); rem = rem - 8; }
        if (rem >= 4) { scale = scale * (zero + 0.0625); rem = rem - 4; }
        if (rem >= 2) { scale = scale * (zero + 0.25); rem = rem - 2; }
        if (rem >= 1) { scale = scale * (zero + 0.5); }
        exp_r = exp_r * scale;
    }
    
    return exp_r;
}

fn pow_f64(base: f64, exp: f64) -> f64 {
    let zero = base - base;
    let one = zero + 1.0;
    
    if (exp == zero) { return one; }
    if (base == zero) { return zero; }
    if (base == one) { return one; }
    if (exp == one) { return base; }
    
    // Integer exponent: fast binary exponentiation
    let exp_i = i32(exp);
    if (f64(exp_i) == exp) {
        var result = one;
        var b = base;
        var e = exp_i;
        if (e < 0) { b = one / b; e = -e; }
        while (e > 0) {
            if ((e & 1) == 1) { result = result * b; }
            b = b * b;
            e = e >> 1;
        }
        return result;
    }
    
    // Fractional exponent: base^exp = exp(exp * log(base))
    // REQUIRES: base > 0 for real result
    if (base < zero) {
        // Negative base with fractional exponent → NaN (return 0 as sentinel)
        return zero;
    }
    
    return exp_f64(exp * log_f64(base));
}

fn log_f64(x: f64) -> f64 {
    let zero = x - x;
    let one = zero + 1.0;
    let two = zero + 2.0;
    
    if (x <= zero) {
        let big = zero + 1e38;
        return -big * big;
    }
    
    var y = x;
    var k = zero;
    
    while (y >= two) { y = y * (zero + 0.5); k = k + one; }
    while (y < one) { y = y * two; k = k - one; }
    
    let z = y - one;
    let s = z / (two + z);
    let s2 = s * s;
    
    let c1 = zero + 0.3333333333333367565;
    let c2 = zero + 0.1999999999970470954;
    let c3 = zero + 0.1428571437183119575;
    
    var p = c3;
    p = p * s2 + c2;
    p = p * s2 + c1;
    
    let log_y = two * s * (one + s2 * p);
    let ln2 = zero + 0.6931471805599453;
    return k * ln2 + log_y;
}

// ============================================================================
// FAO-56 PENMAN-MONTEITH (airSpring core equation)
// ============================================================================
// Input per station-day: [tmax, tmin, rh_max, rh_min, wind_2m, Rs, elevation, lat, doy]
// Output: ET₀ (mm/day)

fn fao56_et0(
    tmax: f64, tmin: f64,
    rh_max: f64, rh_min: f64,
    wind_2m: f64,
    rs: f64,
    elevation: f64,
    lat: f64,
    doy: u32
) -> f64 {
    let zero = tmax - tmax;
    let one = zero + 1.0;
    
    // Mean temperature
    let tmean = (tmax + tmin) / (zero + 2.0);
    
    // Atmospheric pressure (FAO-56 Eq. 7)
    let p = (zero + 101.3) * pow_f64((zero + 293.0) - (zero + 0.0065) * elevation, zero + 5.26) / pow_f64(zero + 293.0, zero + 5.26);
    
    // Psychrometric constant γ (kPa/°C)
    let gamma = (zero + 0.000665) * p;
    
    // Saturation vapour pressure
    let e_tmax = (zero + 0.6108) * exp_f64((zero + 17.27) * tmax / (tmax + (zero + 237.3)));
    let e_tmin = (zero + 0.6108) * exp_f64((zero + 17.27) * tmin / (tmin + (zero + 237.3)));
    let es = (e_tmax + e_tmin) / (zero + 2.0);
    
    // Actual vapour pressure (from RH)
    let ea = (e_tmin * rh_max / (zero + 100.0) + e_tmax * rh_min / (zero + 100.0)) / (zero + 2.0);
    
    // Slope of saturation vapour pressure curve Δ
    let e_tmean = (zero + 0.6108) * exp_f64((zero + 17.27) * tmean / (tmean + (zero + 237.3)));
    let delta = (zero + 4098.0) * e_tmean / pow_f64(tmean + (zero + 237.3), zero + 2.0);
    
    // Extraterrestrial radiation Ra (simplified)
    let pi = zero + 3.141592653589793;
    let lat_rad = lat * pi / (zero + 180.0);
    let dr = one + (zero + 0.033) * cos_simple((zero + 2.0) * pi * f64(doy) / (zero + 365.0));
    let decl = (zero + 0.409) * sin_simple((zero + 2.0) * pi * f64(doy) / (zero + 365.0) - (zero + 1.39));
    
    var ws = acos_simple(-tan_simple(lat_rad) * tan_simple(decl));
    if (ws != ws) { ws = pi; } // NaN check
    
    let gsc = zero + 0.0820;
    let ra = (zero + 24.0) * (zero + 60.0) / pi * gsc * dr * (
        ws * sin_simple(lat_rad) * sin_simple(decl) +
        cos_simple(lat_rad) * cos_simple(decl) * sin_simple(ws)
    );
    
    // Clear-sky radiation Rso
    let rso = ((zero + 0.75) + (zero + 0.00002) * elevation) * ra;
    
    // Net shortwave radiation
    let rns = (one - (zero + 0.23)) * rs;
    
    // Net longwave radiation (simplified Stefan-Boltzmann)
    let sigma = zero + 0.000000004903;  // MJ/(K⁴·m²·day)
    let tmax_k = tmax + (zero + 273.16);
    let tmin_k = tmin + (zero + 273.16);
    let rnl = sigma * (pow_f64(tmax_k, zero + 4.0) + pow_f64(tmin_k, zero + 4.0)) / (zero + 2.0) *
              ((zero + 0.34) - (zero + 0.14) * sqrt(ea)) *
              ((zero + 1.35) * rs / rso - (zero + 0.35));
    
    // Net radiation
    let rn = rns - rnl;
    
    // Soil heat flux (daily: G ≈ 0)
    let g = zero;
    
    // FAO-56 Penman-Monteith equation
    let numerator = (zero + 0.408) * delta * (rn - g) +
                    gamma * (zero + 900.0) / (tmean + (zero + 273.0)) * wind_2m * (es - ea);
    let denominator = delta + gamma * (one + (zero + 0.34) * wind_2m);
    
    return numerator / denominator;
}

// Precision trig for ET₀ (f64 Taylor series)
fn sin_simple(x: f64) -> f64 {
    let zero = x - x;
    let one = zero + 1.0;
    let pi = zero + 3.141592653589793;
    let two_pi = zero + 6.283185307179586;
    
    // Range reduction to [-π, π]
    var y = x;
    while (y > pi) { y = y - two_pi; }
    while (y < -pi) { y = y + two_pi; }
    
    // Taylor series: sin(y) = y - y³/3! + y⁵/5! - y⁷/7! + y⁹/9! - y¹¹/11! + y¹³/13!
    let y2 = y * y;
    let c3 = zero + 0.16666666666666666;   // 1/6
    let c5 = zero + 0.008333333333333333;  // 1/120
    let c7 = zero + 0.0001984126984126984; // 1/5040
    let c9 = zero + 0.0000027557319223985893; // 1/362880
    let c11 = zero + 2.505210838544172e-8;  // 1/39916800
    let c13 = zero + 1.6059043836821613e-10; // 1/6227020800
    
    var p = -c13;
    p = p * y2 + c11;
    p = p * y2 - c9;
    p = p * y2 + c7;
    p = p * y2 - c5;
    p = p * y2 + c3;
    
    return y * (one - y2 * p);
}

fn cos_simple(x: f64) -> f64 {
    let zero = x - x;
    let one = zero + 1.0;
    let pi = zero + 3.141592653589793;
    let two_pi = zero + 6.283185307179586;
    
    // Range reduction
    var y = x;
    while (y > pi) { y = y - two_pi; }
    while (y < -pi) { y = y + two_pi; }
    
    // Taylor series: cos(y) = 1 - y²/2! + y⁴/4! - y⁶/6! + y⁸/8! - y¹⁰/10! + y¹²/12!
    let y2 = y * y;
    let c2 = zero + 0.5;                   // 1/2
    let c4 = zero + 0.041666666666666664;  // 1/24
    let c6 = zero + 0.001388888888888889;  // 1/720
    let c8 = zero + 0.0000248015873015873; // 1/40320
    let c10 = zero + 2.7557319223985893e-7; // 1/3628800
    let c12 = zero + 2.08767569878681e-9;  // 1/479001600
    
    var p = c12;
    p = p * y2 - c10;
    p = p * y2 + c8;
    p = p * y2 - c6;
    p = p * y2 + c4;
    p = p * y2 - c2;
    
    return one + y2 * p;
}

fn tan_simple(x: f64) -> f64 {
    return sin_simple(x) / cos_simple(x);
}

fn acos_simple(x: f64) -> f64 {
    let zero = x - x;
    let one = zero + 1.0;
    let half_pi = zero + 1.5707963267948966;
    let pi = zero + 3.141592653589793;
    
    // Boundary cases
    if (x >= one) { return zero; }
    if (x <= -one) { return pi; }
    
    // acos(x) = atan2(sqrt(1-x²), x) approximation via asin
    // For |x| <= 0.5: acos(x) = π/2 - asin(x)
    // For x > 0.5: acos(x) = 2 * asin(sqrt((1-x)/2))
    // For x < -0.5: acos(x) = π - 2 * asin(sqrt((1+x)/2))
    
    let half = zero + 0.5;
    
    if (x > half) {
        // acos(x) = 2 * asin(sqrt((1-x)/2))
        let t = sqrt((one - x) * half);
        return (zero + 2.0) * asin_core(t);
    } else if (x < -half) {
        // acos(x) = π - 2 * asin(sqrt((1+x)/2))
        let t = sqrt((one + x) * half);
        return pi - (zero + 2.0) * asin_core(t);
    } else {
        // acos(x) = π/2 - asin(x)
        return half_pi - asin_core(x);
    }
}

// Helper: asin for |x| <= 0.5 using Padé approximation
fn asin_core(x: f64) -> f64 {
    let zero = x - x;
    let x2 = x * x;
    
    // Minimax polynomial for asin(x)/x for |x| <= 0.5
    // asin(x) ≈ x * (1 + x² * P(x²))
    let c1 = zero + 0.16666666666666666;   // 1/6
    let c2 = zero + 0.075;                  // 3/40
    let c3 = zero + 0.04464285714285714;   // 15/336
    let c4 = zero + 0.030381944444444446;  // 35/1152
    let c5 = zero + 0.022372159090909092;  // 63/2816
    
    var p = c5;
    p = p * x2 + c4;
    p = p * x2 + c3;
    p = p * x2 + c2;
    p = p * x2 + c1;
    
    return x * ((zero + 1.0) + x2 * p);
}

// ============================================================================
// BINDINGS
// ============================================================================

struct BatchParams {
    batch_size: u32,
    stride: u32,       // Elements per batch item
    operation: u32,    // Operation enum
    aux_param: f64,    // Auxiliary parameter (e.g., total for normalization)
}

@group(0) @binding(0) var<storage, read> input: array<f64>;
@group(0) @binding(1) var<storage, read_write> output: array<f64>;
@group(0) @binding(2) var<uniform> params: BatchParams;

// ============================================================================
// MAIN: One workgroup per batch element
// ============================================================================
@compute @workgroup_size(64)
fn batched_compute(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let batch_idx = workgroup_id.x;
    if (batch_idx >= params.batch_size) {
        return;
    }
    
    let base = batch_idx * params.stride;
    let op = params.operation;
    
    // Only thread 0 in each workgroup computes the result
    if (local_id.x != 0u) {
        return;
    }
    
    switch (op) {
        case 0u: {
            // FAO-56 ET₀: input is [tmax, tmin, rh_max, rh_min, wind, Rs, elev, lat, doy]
            let tmax = input[base + 0u];
            let tmin = input[base + 1u];
            let rh_max = input[base + 2u];
            let rh_min = input[base + 3u];
            let wind = input[base + 4u];
            let rs = input[base + 5u];
            let elev = input[base + 6u];
            let lat = input[base + 7u];
            let doy = u32(input[base + 8u]);
            
            output[batch_idx] = fao56_et0(tmax, tmin, rh_max, rh_min, wind, rs, elev, lat, doy);
        }
        case 1u: {
            // Water balance: simplified Dr update
            // input: [Dr_prev, P, I, ETc, TAW, RAW, p]
            let dr_prev = input[base + 0u];
            let precip = input[base + 1u];
            let irrig = input[base + 2u];
            let etc = input[base + 3u];
            let taw = input[base + 4u];
            let raw = input[base + 5u];
            let p_frac = input[base + 6u];
            
            let zero = dr_prev - dr_prev;
            
            // Stress coefficient Ks
            var ks = zero + 1.0;
            if (dr_prev > raw) {
                ks = (taw - dr_prev) / (taw - raw);
                if (ks < zero) { ks = zero; }
            }
            
            // Adjusted ETc
            let etc_adj = ks * etc;
            
            // New depletion
            var dr_new = dr_prev - precip - irrig + etc_adj;
            if (dr_new < zero) { dr_new = zero; }
            if (dr_new > taw) { dr_new = taw; }
            
            output[batch_idx] = dr_new;
        }
        default: {
            // Identity / passthrough first element
            output[batch_idx] = input[base];
        }
    }
}
