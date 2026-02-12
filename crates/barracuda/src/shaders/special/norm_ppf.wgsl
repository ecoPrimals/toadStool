// Inverse Normal CDF (Percent Point Function / Probit)
// Φ⁻¹(p) = x such that Φ(x) = p
//
// Uses Acklam's algorithm - rational approximation with max error ~1.15e-9
// No iteration needed, pure arithmetic - ideal for GPU
//
// Applications: Black-Scholes, Monte Carlo, quantile normalization, probit regression
// Reference: Acklam (2004), https://web.archive.org/web/20151030215612/http://home.online.no/~pjacklam/notes/invnorm/

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

struct Params {
    size: u32,
    mu: f32,      // Mean (0.0 for standard)
    sigma: f32,   // Std dev (1.0 for standard)
}

// Acklam's coefficients for rational approximation
// Split into regions: lower tail, central, upper tail

// Coefficients for central region (|p - 0.5| <= 0.425)
const A1: f32 = -3.969683028665376e+01;
const A2: f32 =  2.209460984245205e+02;
const A3: f32 = -2.759285104469687e+02;
const A4: f32 =  1.383577518672690e+02;
const A5: f32 = -3.066479806614716e+01;
const A6: f32 =  2.506628277459239e+00;

const B1: f32 = -5.447609879822406e+01;
const B2: f32 =  1.615858368580409e+02;
const B3: f32 = -1.556989798598866e+02;
const B4: f32 =  6.680131188771972e+01;
const B5: f32 = -1.328068155288572e+01;

// Coefficients for tail regions
const C1: f32 = -7.784894002430293e-03;
const C2: f32 = -3.223964580411365e-01;
const C3: f32 = -2.400758277161838e+00;
const C4: f32 = -2.549732539343734e+00;
const C5: f32 =  4.374664141464968e+00;
const C6: f32 =  2.938163982698783e+00;

const D1: f32 =  7.784695709041462e-03;
const D2: f32 =  3.224671290700398e-01;
const D3: f32 =  2.445134137142996e+00;
const D4: f32 =  3.754408661907416e+00;

// Breakpoints
const P_LOW: f32 = 0.02425;
const P_HIGH: f32 = 0.97575;  // 1 - P_LOW

// Large value to represent infinity (WGSL has no inf literal)
const INF: f32 = 3.4028235e+38;  // f32::MAX

// Standard normal inverse CDF (probit function)
fn norm_ppf_standard(p: f32) -> f32 {
    // Handle boundaries with large finite values
    if (p <= 0.0) {
        return -INF;
    }
    if (p >= 1.0) {
        return INF;
    }

    var r: f32;
    var x: f32;

    if (p < P_LOW) {
        // Lower tail region
        let q = sqrt(-2.0 * log(p));
        x = (((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6) /
            ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0);
    } else if (p <= P_HIGH) {
        // Central region
        let q = p - 0.5;
        r = q * q;
        x = (((((A1 * r + A2) * r + A3) * r + A4) * r + A5) * r + A6) * q /
            (((((B1 * r + B2) * r + B3) * r + B4) * r + B5) * r + 1.0);
    } else {
        // Upper tail region (symmetric to lower)
        let q = sqrt(-2.0 * log(1.0 - p));
        x = -(((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6) /
            ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0);
    }

    return x;
}

// General normal inverse CDF with μ, σ
fn norm_ppf(p: f32, mu: f32, sigma: f32) -> f32 {
    return mu + sigma * norm_ppf_standard(p);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.size) {
        return;
    }

    let p = input[idx];
    output[idx] = norm_ppf(p, params.mu, params.sigma);
}
