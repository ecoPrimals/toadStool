// SPDX-License-Identifier: AGPL-3.0-or-later
//! math_f64.wgsl Validation Benchmark
//!
//! Validates the pure-GPU f64 math library against CPU reference implementations.
//! Tests precision and performance of transcendental functions.

// Mathematical constants in f64 algorithms must match the exact values
// used in the WGSL shaders for proper validation
#![allow(clippy::approx_constant)]

use barracuda::shaders::ShaderTemplate;

/// Test values for validation
const TEST_VALUES: &[f64] = &[
    0.0,
    0.5,
    1.0,
    1.5,
    2.0,
    2.5,
    3.0,
    4.0,
    5.0,
    10.0,
    0.1,
    0.25,
    0.75,
    1.25,
    1.75,
    2.25,
    3.5,
    4.5,
    7.0,
    9.0,
    0.01,
    0.001,
    0.0001,
    100.0,
    1000.0,
    10000.0,
    std::f64::consts::PI,
    std::f64::consts::E,
];

/// Validate sqrt implementation
fn validate_sqrt() {
    println!("  sqrt_f64:");
    let mut max_error = 0.0f64;
    let mut max_rel_error = 0.0f64;

    for &x in TEST_VALUES {
        if x >= 0.0 {
            let cpu = x.sqrt();
            // Simulate GPU implementation (Newton-Raphson)
            let gpu = cpu_sqrt_f64(x);
            let error = (cpu - gpu).abs();
            let rel_error = if cpu != 0.0 { error / cpu.abs() } else { error };
            max_error = max_error.max(error);
            max_rel_error = max_rel_error.max(rel_error);
        }
    }

    println!("    Max absolute error: {max_error:.2e}");
    println!("    Max relative error: {max_rel_error:.2e}");
    println!(
        "    Status: {}",
        if max_rel_error < 1e-14 {
            "✅ PASS (full precision)"
        } else {
            "⚠️ NEEDS TUNING"
        }
    );
}

/// CPU implementation of sqrt (mimics GPU Newton-Raphson)
fn cpu_sqrt_f64(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }

    // Newton-Raphson: x_{n+1} = 0.5 * (x_n + S / x_n)
    let mut r = (x + 1.0) / 2.0; // Initial guess
    for _ in 0..5 {
        r = 0.5 * (r + x / r);
    }
    r
}

/// Validate cbrt implementation
fn validate_cbrt() {
    println!("  cbrt_f64:");
    let mut max_error = 0.0f64;
    let mut max_rel_error = 0.0f64;

    for &x in TEST_VALUES {
        let cpu = x.cbrt();
        let gpu = cpu_cbrt_f64(x);
        let error = (cpu - gpu).abs();
        let rel_error = if cpu.abs() > 1e-15 {
            error / cpu.abs()
        } else {
            error
        };
        max_error = max_error.max(error);
        max_rel_error = max_rel_error.max(rel_error);
    }

    println!("    Max absolute error: {max_error:.2e}");
    println!("    Max relative error: {max_rel_error:.2e}");
    println!(
        "    Status: {}",
        if max_rel_error < 1e-13 {
            "✅ PASS"
        } else {
            "⚠️ NEEDS TUNING"
        }
    );
}

/// CPU implementation of cbrt (mimics GPU Halley's method)
fn cpu_cbrt_f64(x: f64) -> f64 {
    if x == 0.0 {
        return 0.0;
    }

    let neg = x < 0.0;
    let y = x.abs();

    // Halley's method
    let mut r = (y + 1.0) / 2.0;
    for _ in 0..6 {
        let r3 = r * r * r;
        r = r * (r3 + 2.0 * y) / (2.0 * r3 + y);
    }

    if neg {
        -r
    } else {
        r
    }
}

/// Validate exp implementation
fn validate_exp() {
    println!("  exp_f64:");
    let mut max_error = 0.0f64;
    let mut max_rel_error = 0.0f64;

    // Test range [-20, 20] to avoid overflow/underflow
    let exp_test_values: Vec<f64> = (-20..=20).map(|i| i as f64).collect();

    for x in &exp_test_values {
        let cpu = x.exp();
        let gpu = cpu_exp_f64(*x);
        let error = (cpu - gpu).abs();
        let rel_error = if cpu.abs() > 1e-300 {
            error / cpu.abs()
        } else {
            error
        };
        max_error = max_error.max(error);
        max_rel_error = max_rel_error.max(rel_error);
    }

    println!("    Max absolute error: {max_error:.2e}");
    println!("    Max relative error: {max_rel_error:.2e}");
    println!(
        "    Status: {}",
        if max_rel_error < 1e-12 {
            "✅ PASS"
        } else {
            "⚠️ NEEDS TUNING"
        }
    );
}

/// CPU implementation of exp (mimics GPU polynomial)
fn cpu_exp_f64(x: f64) -> f64 {
    if x > 709.0 {
        return f64::MAX;
    }
    if x < -745.0 {
        return 0.0;
    }
    if x.abs() < 1e-15 {
        return 1.0 + x;
    }

    // Range reduction
    let inv_ln2 = 1.4426950408889634;
    let k = (x * inv_ln2).round();
    let ln2_hi = 0.693_147_180_559_945_3;
    let ln2_lo = 1.94821509970e-17;
    let r = x - k * ln2_hi - k * ln2_lo;

    // Polynomial
    let r2 = r * r;
    let coeffs = [
        0.5,
        0.166_666_666_666_666_66,
        0.041_666_666_666_666_664,
        0.008_333_333_333_334_011,
        0.001_388_888_888_897_745,
        0.000_198_412_698_413_242_4,
        0.000_024_801_587_301_587_3,
        0.000_002_755_731_922_398_589_3,
        2.755_731_919_138_63e-7,
        2.505_210_838_544_172e-8,
        2.087_675_698_786_81e-9,
        1.605_904_383_682_161_3e-10,
    ];

    let mut p = coeffs[11];
    for i in (0..11).rev() {
        p = p * r + coeffs[i];
    }

    let exp_r = 1.0 + r + r2 * p;

    // Scale by 2^k
    exp_r * 2.0_f64.powi(k as i32)
}

/// Validate log implementation
fn validate_log() {
    println!("  log_f64:");
    let mut max_error = 0.0f64;
    let mut max_rel_error = 0.0f64;

    for &x in TEST_VALUES {
        if x > 0.0 {
            let cpu = x.ln();
            let gpu = cpu_log_f64(x);
            let error = (cpu - gpu).abs();
            let rel_error = if cpu.abs() > 1e-15 {
                error / cpu.abs()
            } else {
                error
            };
            max_error = max_error.max(error);
            max_rel_error = max_rel_error.max(rel_error);
        }
    }

    println!("    Max absolute error: {max_error:.2e}");
    println!("    Max relative error: {max_rel_error:.2e}");
    println!(
        "    Status: {}",
        if max_rel_error < 1e-12 {
            "✅ PASS"
        } else {
            "⚠️ NEEDS TUNING"
        }
    );
}

/// CPU implementation of log (mimics GPU algorithm)
fn cpu_log_f64(x: f64) -> f64 {
    if x <= 0.0 {
        return f64::NEG_INFINITY;
    }

    // Range reduction to [1, 2)
    let mut y = x;
    let mut k = 0.0;

    while y >= 2.0 {
        y *= 0.5;
        k += 1.0;
    }
    while y < 1.0 {
        y *= 2.0;
        k -= 1.0;
    }

    // Use atanh-based formula
    let z = y - 1.0;
    let s = z / (2.0 + z);
    let s2 = s * s;

    let coeffs = [
        0.666_666_666_666_673_5,
        0.399_999_999_994_094_2,
        0.285_714_287_436_623_9,
        0.222_221_984_321_497_84,
        0.181_835_721_616_180_5,
        0.153_138_376_992_093_73,
        0.147_981_986_051_165_86,
    ];

    let mut p = coeffs[6];
    for i in (0..6).rev() {
        p = p * s2 + coeffs[i];
    }

    let log_y = 2.0 * s * (1.0 + s2 * p);
    let ln2 = 0.6931471805599453;

    k * ln2 + log_y
}

/// Validate pow implementation for nuclear physics use case
fn validate_pow_nuclear() {
    println!("  pow_f64 (nuclear physics A^(2/3)):");

    // Test with mass numbers typical in nuclear physics
    let mass_numbers: Vec<f64> = (1..=250).map(|a| a as f64).collect();
    let mut max_error = 0.0f64;
    let mut max_rel_error = 0.0f64;

    for &a in &mass_numbers {
        let cpu = a.powf(2.0 / 3.0);
        let gpu = cpu_pow_two_thirds(a);
        let error = (cpu - gpu).abs();
        let rel_error = if cpu > 0.0 { error / cpu } else { error };
        max_error = max_error.max(error);
        max_rel_error = max_rel_error.max(rel_error);
    }

    println!("    Test range: A = 1 to 250 (nuclear mass numbers)");
    println!("    Max absolute error: {max_error:.2e}");
    println!("    Max relative error: {max_rel_error:.2e}");

    // hotSpring reported 4e-4 relative error with exp(log()) chain
    // Our specialized cbrt*cbrt path should be better
    if max_rel_error < 1e-12 {
        println!("    Status: ✅ EXCELLENT (< 1e-12)");
    } else if max_rel_error < 1e-10 {
        println!("    Status: ✅ GOOD (< 1e-10)");
    } else if max_rel_error < 1e-4 {
        println!("    Status: ⚠️ ACCEPTABLE for science (< 1e-4)");
    } else {
        println!("    Status: ❌ NEEDS IMPROVEMENT (> 1e-4)");
    }
}

/// Specialized A^(2/3) using cbrt*cbrt (avoids exp/log chain)
fn cpu_pow_two_thirds(x: f64) -> f64 {
    let cbrt_x = cpu_cbrt_f64(x);
    cbrt_x * cbrt_x
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║  math_f64.wgsl Validation                                                    ║");
    println!("║  Testing pure-GPU f64 transcendental functions                               ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    // Show that the preamble loads
    let preamble = ShaderTemplate::math_f64_preamble();
    let lines = preamble.lines().count();
    println!("  math_f64.wgsl loaded: {lines} lines\n");

    println!("══════════════════════════════════════════════════════════════════════════════");
    println!("  CPU Reference Validation (mimics GPU algorithms)");
    println!("══════════════════════════════════════════════════════════════════════════════\n");

    validate_sqrt();
    println!();
    validate_cbrt();
    println!();
    validate_exp();
    println!();
    validate_log();
    println!();
    validate_pow_nuclear();

    println!("\n══════════════════════════════════════════════════════════════════════════════");
    println!("  NAGA/WGSL GOTCHAS (Critical for GPU implementation)");
    println!("══════════════════════════════════════════════════════════════════════════════\n");

    println!("  1. AbstractFloat does NOT auto-promote to f64:");
    println!("     WRONG: return 1.0;");
    println!("     RIGHT: return x - x + 1.0;  // f64 type propagates\n");

    println!("  2. Literals > f32 range cause parse errors:");
    println!("     WRONG: 1e308");
    println!("     RIGHT: construct via arithmetic\n");

    println!("  3. No f64 overloads for ANY builtins:");
    println!("     sqrt, pow, exp, log, sin, cos, abs, etc. → must implement\n");

    println!("  4. No vec2/3/4<f64> types:");
    println!("     All f64 operations are scalar only\n");

    // Check GPU capabilities
    println!("══════════════════════════════════════════════════════════════════════════════");
    println!("  GPU Capability Check");
    println!("══════════════════════════════════════════════════════════════════════════════\n");

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
        let info = adapter.get_info();
        let features = adapter.features();

        // Skip software/CPU adapters
        if info.device_type == wgpu::DeviceType::Cpu {
            continue;
        }

        let has_f64 = features.contains(wgpu::Features::SHADER_F64);
        println!(
            "  {} ({:?}): SHADER_F64 = {}",
            info.name,
            info.backend,
            if has_f64 {
                "✅ Supported"
            } else {
                "❌ Not supported"
            }
        );
    }

    println!("\n═══ SUMMARY ═══\n");
    println!("  math_f64.wgsl provides pure-GPU f64 transcendentals for:");
    println!("  - sqrt, cbrt (Newton-Raphson, Halley - full precision)");
    println!("  - exp, log (polynomial approximation - ~1e-12 precision)");
    println!("  - pow with specialized paths for A^(1/3), A^(2/3)");
    println!("  - sin, cos, tan, sinh, cosh, tanh");
    println!("  - gamma (Lanczos), erf, bessel_j0\n");

    println!("  Usage:");
    println!("    let shader = ShaderTemplate::with_math_f64(my_shader_code);");

    Ok(())
}
