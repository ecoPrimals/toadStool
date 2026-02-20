// complex_f64.wgsl — f64 complex arithmetic library
//
// Prepend this to any WGSL shader that requires complex-number operations.
// All functions operate on vec2<f64> where .x = real part, .y = imaginary part.
//
// Naming convention: c64_*
//   c64_new(re, im)  →  vec2<f64>(re, im)
//
// NVK / Mesa NAK note:
//   c64_exp uses exp(), cos(), sin() builtins.  On nouveau (NVK) these require
//   the exp/log workaround.  Always compile complex-using shaders through
//   ShaderTemplate::for_driver_profile() with the exp/log flag set.
//
// hotSpring absorption: lattice/complex_f64.rs (v0.5.16, Feb 2026)
// CPU-validated against paper reference implementations.

// ── Constructors ──────────────────────────────────────────────────────────────

fn c64_new(re: f64, im: f64) -> vec2<f64> { return vec2<f64>(re, im); }
fn c64_zero() -> vec2<f64>               { return vec2<f64>(0.0, 0.0); }
fn c64_one()  -> vec2<f64>               { return vec2<f64>(1.0, 0.0); }
fn c64_i()    -> vec2<f64>               { return vec2<f64>(0.0, 1.0); }

// ── Basic arithmetic ──────────────────────────────────────────────────────────

fn c64_add(a: vec2<f64>, b: vec2<f64>) -> vec2<f64> {
    return vec2<f64>(a.x + b.x, a.y + b.y);
}

fn c64_sub(a: vec2<f64>, b: vec2<f64>) -> vec2<f64> {
    return vec2<f64>(a.x - b.x, a.y - b.y);
}

fn c64_mul(a: vec2<f64>, b: vec2<f64>) -> vec2<f64> {
    // (a.x + i·a.y)(b.x + i·b.y) = (a.x·b.x − a.y·b.y) + i(a.x·b.y + a.y·b.x)
    return vec2<f64>(
        a.x * b.x - a.y * b.y,
        a.x * b.y + a.y * b.x,
    );
}

fn c64_conj(a: vec2<f64>) -> vec2<f64> {
    return vec2<f64>(a.x, -a.y);
}

fn c64_scale(a: vec2<f64>, s: f64) -> vec2<f64> {
    return vec2<f64>(a.x * s, a.y * s);
}

// ── Norm and inverse ──────────────────────────────────────────────────────────

fn c64_abs_sq(a: vec2<f64>) -> f64 {
    return a.x * a.x + a.y * a.y;
}

fn c64_abs(a: vec2<f64>) -> f64 {
    return sqrt(c64_abs_sq(a));
}

fn c64_inv(a: vec2<f64>) -> vec2<f64> {
    let denom = c64_abs_sq(a);
    return vec2<f64>(a.x / denom, -a.y / denom);
}

fn c64_div(a: vec2<f64>, b: vec2<f64>) -> vec2<f64> {
    return c64_mul(a, c64_inv(b));
}

// ── Exponential ───────────────────────────────────────────────────────────────
// e^(x + iy) = e^x · (cos y + i sin y)
// Requires NVK exp/log workaround when compiled for nouveau.

fn c64_exp(a: vec2<f64>) -> vec2<f64> {
    let mag = exp(a.x);
    return vec2<f64>(mag * cos(a.y), mag * sin(a.y));
}

// ── Phase factor ──────────────────────────────────────────────────────────────
// e^(i·theta) = cos(theta) + i·sin(theta)

fn c64_phase(theta: f64) -> vec2<f64> {
    return vec2<f64>(cos(theta), sin(theta));
}
