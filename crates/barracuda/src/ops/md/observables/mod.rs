//! Observable Computation for Molecular Dynamics
//!
//! GPU kernels and CPU utilities for computing physical observables.
//!
//! **GPU Observables**:
//! - Kinetic energy (per-particle, for temperature)
//! - RDF histogram (pair distances with atomicAdd)
//!
//! **CPU Post-Processing** (from GPU snapshots):
//! - VACF (velocity autocorrelation)
//! - SSF (static structure factor)
//! - Energy statistics and drift
//!
//! **Deep Debt Compliance**:
//! - ✅ WGSL shader-first (separate .wgsl files)
//! - ✅ Full f64 precision
//! - ✅ Zero unsafe code

mod kinetic_energy;

pub use kinetic_energy::KineticEnergy;

use std::f64::consts::PI;

/// RDF result: g(r) binned at discrete r values
#[derive(Clone, Debug)]
pub struct Rdf {
    pub r_values: Vec<f64>,
    pub g_values: Vec<f64>,
    pub dr: f64,
}

/// VACF result: C(t) at discrete lag times
#[derive(Clone, Debug)]
pub struct Vacf {
    pub t_values: Vec<f64>,
    pub c_values: Vec<f64>,
    pub diffusion_coeff: f64,
}

/// Energy validation result
#[derive(Clone, Debug)]
pub struct EnergyValidation {
    pub mean_total: f64,
    pub std_total: f64,
    pub drift_pct: f64,
    pub mean_temperature: f64,
    pub std_temperature: f64,
    pub passed: bool,
}

/// Compute RDF from position snapshots (CPU post-process)
///
/// # Arguments
/// * `snapshots` - Position snapshots, each [N*3] flattened
/// * `n` - Number of particles
/// * `box_side` - Box side length in reduced units
/// * `n_bins` - Number of histogram bins
pub fn compute_rdf(
    snapshots: &[Vec<f64>],
    n: usize,
    box_side: f64,
    n_bins: usize,
) -> Rdf {
    let r_max = box_side / 2.0;
    let dr = r_max / n_bins as f64;
    let mut histogram = vec![0u64; n_bins];
    let n_frames = snapshots.len();

    for snap in snapshots {
        for i in 0..n {
            let xi = snap[i * 3];
            let yi = snap[i * 3 + 1];
            let zi = snap[i * 3 + 2];

            for j in (i + 1)..n {
                let mut dx = snap[j * 3] - xi;
                let mut dy = snap[j * 3 + 1] - yi;
                let mut dz = snap[j * 3 + 2] - zi;

                // PBC minimum image
                dx -= box_side * (dx / box_side).round();
                dy -= box_side * (dy / box_side).round();
                dz -= box_side * (dz / box_side).round();

                let r = (dx * dx + dy * dy + dz * dz).sqrt();
                let bin = (r / dr) as usize;
                if bin < n_bins {
                    histogram[bin] += 1;
                }
            }
        }
    }

    // Normalize: g(r) = histogram / (n_frames * N * n_density * 4π r² dr)
    let n_density = 3.0 / (4.0 * PI); // OCP reduced units
    let n_f = n as f64;
    let r_values: Vec<f64> = (0..n_bins).map(|i| (i as f64 + 0.5) * dr).collect();
    let g_values: Vec<f64> = r_values
        .iter()
        .enumerate()
        .map(|(i, &r)| {
            let shell_vol = 4.0 * PI * r * r * dr;
            2.0 * histogram[i] as f64 / (n_frames as f64 * n_f * n_density * shell_vol).max(1e-30)
        })
        .collect();

    Rdf { r_values, g_values, dr }
}

/// Compute VACF from velocity snapshots (CPU post-process)
///
/// # Arguments
/// * `vel_snapshots` - Velocity snapshots, each [N*3] flattened
/// * `n` - Number of particles
/// * `dt_dump` - Time between snapshots (reduced units)
/// * `max_lag` - Maximum lag in snapshots
pub fn compute_vacf(
    vel_snapshots: &[Vec<f64>],
    n: usize,
    dt_dump: f64,
    max_lag: usize,
) -> Vacf {
    let n_frames = vel_snapshots.len();
    let n_lag = max_lag.min(n_frames);
    let mut c_values = vec![0.0f64; n_lag];
    let mut counts = vec![0usize; n_lag];

    for t0 in 0..n_frames {
        for lag in 0..n_lag {
            let t1 = t0 + lag;
            if t1 >= n_frames {
                break;
            }
            let v0 = &vel_snapshots[t0];
            let v1 = &vel_snapshots[t1];

            let mut dot_sum = 0.0;
            for i in 0..n {
                dot_sum += v0[i * 3] * v1[i * 3]
                    + v0[i * 3 + 1] * v1[i * 3 + 1]
                    + v0[i * 3 + 2] * v1[i * 3 + 2];
            }
            c_values[lag] += dot_sum / n as f64;
            counts[lag] += 1;
        }
    }

    // Average over time origins
    for i in 0..n_lag {
        if counts[i] > 0 {
            c_values[i] /= counts[i] as f64;
        }
    }

    // Normalize by C(0)
    let c0 = c_values[0].max(1e-30);
    let c_normalized: Vec<f64> = c_values.iter().map(|&c| c / c0).collect();

    // Diffusion coefficient: D* = (1/3) integral_0^inf C(t) dt
    let mut integral = 0.0;
    for i in 1..n_lag {
        integral += 0.5 * (c_values[i - 1] + c_values[i]) * dt_dump;
    }
    let diffusion_coeff = integral / 3.0;

    let t_values: Vec<f64> = (0..n_lag).map(|i| i as f64 * dt_dump).collect();

    Vacf {
        t_values,
        c_values: c_normalized,
        diffusion_coeff,
    }
}

/// Compute static structure factor S(k) from position snapshots
///
/// # Arguments
/// * `snapshots` - Position snapshots, each [N*3] flattened
/// * `n` - Number of particles
/// * `box_side` - Box side length in reduced units
/// * `max_k_harmonics` - Number of k-vectors along each axis
pub fn compute_ssf(
    snapshots: &[Vec<f64>],
    n: usize,
    box_side: f64,
    max_k_harmonics: usize,
) -> Vec<(f64, f64)> {
    let dk = 2.0 * PI / box_side;
    let _n_frames = snapshots.len(); // Used implicitly via iteration
    let mut sk_values: Vec<(f64, f64)> = Vec::new();

    for kn in 1..=max_k_harmonics {
        let k_mag = kn as f64 * dk;
        let mut sk_sum = 0.0;
        let mut count = 0;

        for snap in snapshots {
            // S(k) = <|rho(k)|²> / N along each principal axis
            for axis in 0..3 {
                let mut re = 0.0;
                let mut im = 0.0;
                for j in 0..n {
                    let r_component = snap[j * 3 + axis];
                    let phase = k_mag * r_component;
                    re += phase.cos();
                    im += phase.sin();
                }
                sk_sum += (re * re + im * im) / n as f64;
                count += 1;
            }
        }

        sk_values.push((k_mag, sk_sum / count as f64));
    }

    sk_values
}

/// Validate energy conservation from energy history
///
/// # Arguments
/// * `energies` - Vector of (step, KE, PE, total) tuples
/// * `skip_fraction` - Fraction of initial data to skip (e.g., 0.1)
pub fn validate_energy(
    energies: &[(usize, f64, f64, f64)],
    skip_fraction: f64,
) -> EnergyValidation {
    if energies.is_empty() {
        return EnergyValidation {
            mean_total: 0.0,
            std_total: 0.0,
            drift_pct: 0.0,
            mean_temperature: 0.0,
            std_temperature: 0.0,
            passed: false,
        };
    }

    let skip = ((energies.len() as f64) * skip_fraction) as usize;
    let stable: Vec<_> = energies.iter().skip(skip).collect();

    let totals: Vec<f64> = stable.iter().map(|e| e.3).collect();
    let mean_e: f64 = totals.iter().sum::<f64>() / totals.len() as f64;
    let var_e: f64 = totals
        .iter()
        .map(|e| (e - mean_e).powi(2))
        .sum::<f64>()
        / totals.len() as f64;
    let std_e = var_e.sqrt();

    let e_initial = stable.first().unwrap().3;
    let e_final = stable.last().unwrap().3;
    let drift_pct = if mean_e.abs() > 1e-30 {
        ((e_final - e_initial) / mean_e.abs()).abs() * 100.0
    } else {
        0.0
    };

    let passed = drift_pct < 5.0;

    EnergyValidation {
        mean_total: mean_e,
        std_total: std_e,
        drift_pct,
        mean_temperature: 0.0, // Caller should compute from KE
        std_temperature: 0.0,
        passed,
    }
}
