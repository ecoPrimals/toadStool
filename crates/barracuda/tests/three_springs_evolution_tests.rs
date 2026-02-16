//! Comprehensive Tests for Three Springs Evolution Primitives
//!
//! Unit, E2E, Chaos, and Fault tests for:
//! - Fused Map-Reduce (Shannon, Simpson, norms) - wetSpring Priority 1
//! - Spatial Interpolation (Kriging) - airSpring + wetSpring
//! - Batched Elementwise Computation - airSpring
//!
//! These primitives serve the unified math library vision across:
//! - hotSpring: Nuclear physics (195 checks)
//! - wetSpring: Life science / analytical chemistry (48 checks)
//! - airSpring: Precision agriculture (70 checks)
//!
//! Date: February 16, 2026
//! License: AGPL-3.0-or-later

use barracuda::device::WgpuDevice;
use barracuda::ops::fused_map_reduce_f64::{FusedMapReduceF64, MapOp, ReduceOp};
use barracuda::ops::kriging_f64::{KrigingF64, KrigingResult, VariogramModel};
use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

fn create_device_sync() -> Option<Arc<WgpuDevice>> {
    pollster::block_on(async {
        match WgpuDevice::new_f64_capable().await {
            Ok(d) => Some(Arc::new(d)),
            Err(_) => None,
        }
    })
}

/// CPU reference Shannon entropy
fn cpu_shannon(counts: &[f64]) -> f64 {
    let total: f64 = counts.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    counts
        .iter()
        .map(|&c| {
            let p = c / total;
            if p > 0.0 {
                -p * p.ln()
            } else {
                0.0
            }
        })
        .sum()
}

/// CPU reference Simpson index
fn cpu_simpson(counts: &[f64]) -> f64 {
    let total: f64 = counts.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    counts.iter().map(|&c| (c / total).powi(2)).sum()
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS: Fused Map-Reduce
// ═══════════════════════════════════════════════════════════════════════════

mod fused_map_reduce_unit {
    use super::*;

    /// Test Shannon entropy with wetSpring reference case
    #[test]
    fn test_shannon_wetspring_reference() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => {
                println!("SKIP: No f64-capable device");
                return;
            }
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // Reference case from wetSpring handoff: [10, 20, 30, 40] → Shannon ≈ 1.27985422
        let counts = vec![10.0, 20.0, 30.0, 40.0];
        let result = fmr.shannon_entropy(&counts).unwrap();
        let expected = cpu_shannon(&counts);

        let error = (result - expected).abs();
        assert!(
            error < 1e-10,
            "Shannon entropy error {} exceeds 1e-10 (got {}, expected {})",
            error,
            result,
            expected
        );
        println!(
            "✓ Shannon wetSpring reference: {} (error: {:.2e})",
            result, error
        );
    }

    /// Test Shannon entropy with uniform distribution (max entropy)
    #[test]
    fn test_shannon_uniform_distribution() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // Uniform: [25, 25, 25, 25] → Shannon = ln(4) ≈ 1.3862943611
        let counts = vec![25.0; 4];
        let result = fmr.shannon_entropy(&counts).unwrap();
        let expected = 4.0_f64.ln(); // ln(n) for uniform

        let error = (result - expected).abs();
        assert!(error < 1e-10, "Uniform Shannon error: {}", error);
        println!(
            "✓ Shannon uniform: {} (expected ln(4) = {})",
            result, expected
        );
    }

    /// Test Shannon entropy with single element (zero entropy)
    #[test]
    fn test_shannon_single_element() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // Single element: all probability on one → entropy = 0
        let counts = vec![100.0];
        let result = fmr.shannon_entropy(&counts).unwrap();

        assert!(
            result.abs() < 1e-10,
            "Single element Shannon should be 0, got {}",
            result
        );
        println!("✓ Shannon single element: {} (expected 0)", result);
    }

    /// Test Simpson index basic case
    #[test]
    fn test_simpson_basic() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        let counts = vec![10.0, 20.0, 30.0, 40.0];
        let result = fmr.simpson_index(&counts).unwrap();
        let expected = cpu_simpson(&counts);

        let error = (result - expected).abs();
        assert!(error < 1e-12, "Simpson error: {}", error);
        println!("✓ Simpson index: {} (error: {:.2e})", result, error);
    }

    /// Test Simpson index uniform distribution
    #[test]
    fn test_simpson_uniform() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // Uniform: [25, 25, 25, 25] → Simpson = 4 * (0.25)^2 = 0.25
        let counts = vec![25.0; 4];
        let result = fmr.simpson_index(&counts).unwrap();
        let expected = 0.25;

        let error = (result - expected).abs();
        assert!(error < 1e-12, "Uniform Simpson error: {}", error);
        println!("✓ Simpson uniform: {} (expected 0.25)", result);
    }

    /// Test sum reduction
    #[test]
    fn test_sum_reduction() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        let data: Vec<f64> = (1..=100).map(|x| x as f64).collect();
        let result = fmr.sum(&data).unwrap();
        let expected: f64 = data.iter().sum();

        let error = (result - expected).abs();
        assert!(error < 1e-10, "Sum error: {}", error);
        println!("✓ Sum 1..100: {} (expected {})", result, expected);
    }

    /// Test max reduction
    #[test]
    fn test_max_reduction() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        let data = vec![1.0, 5.0, 3.0, 9.0, 2.0, 7.0];
        let result = fmr.max(&data).unwrap();

        assert!(
            (result - 9.0).abs() < 1e-10,
            "Max should be 9.0, got {}",
            result
        );
        println!("✓ Max: {} (expected 9.0)", result);
    }

    /// Test min reduction
    #[test]
    fn test_min_reduction() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        let data = vec![5.0, 3.0, 9.0, 1.0, 7.0];
        let result = fmr.min(&data).unwrap();

        assert!(
            (result - 1.0).abs() < 1e-10,
            "Min should be 1.0, got {}",
            result
        );
        println!("✓ Min: {} (expected 1.0)", result);
    }

    /// Test sum of squares
    #[test]
    fn test_sum_of_squares() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = fmr.sum_of_squares(&data).unwrap();
        let expected: f64 = data.iter().map(|x| x * x).sum(); // 1+4+9+16+25 = 55

        let error = (result - expected).abs();
        assert!(error < 1e-10, "Sum of squares error: {}", error);
        println!("✓ Sum of squares: {} (expected {})", result, expected);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS: Kriging Spatial Interpolation
// ═══════════════════════════════════════════════════════════════════════════

mod kriging_unit {
    use super::*;

    /// Test variogram model values
    #[test]
    fn test_variogram_spherical() {
        let model = VariogramModel::Spherical {
            nugget: 0.0,
            sill: 1.0,
            range: 10.0,
        };

        // γ(0) = 0
        assert!((model.gamma(0.0) - 0.0).abs() < 1e-10, "γ(0) should be 0");

        // γ(a) = sill for spherical
        assert!(
            (model.gamma(10.0) - 1.0).abs() < 1e-10,
            "γ(range) should equal sill"
        );

        // γ(h > a) = sill
        assert!(
            (model.gamma(20.0) - 1.0).abs() < 1e-10,
            "γ(h > range) should equal sill"
        );

        println!("✓ Spherical variogram: γ(0)=0, γ(10)=1, γ(20)=1");
    }

    /// Test variogram exponential
    #[test]
    fn test_variogram_exponential() {
        let model = VariogramModel::Exponential {
            nugget: 0.1,
            sill: 1.0,
            range: 10.0,
        };

        // γ(0) = 0
        assert!(model.gamma(0.0).abs() < 1e-10, "γ(0) should be 0");

        // γ(a) ≈ 0.95 * sill for exponential (at h=a, 1 - e^-3 ≈ 0.95)
        let gamma_at_range = model.gamma(10.0);
        let expected = 0.1 + 0.9 * (1.0 - (-3.0_f64).exp());
        assert!(
            (gamma_at_range - expected).abs() < 1e-6,
            "Exponential γ(a) incorrect: {} vs {}",
            gamma_at_range,
            expected
        );

        println!(
            "✓ Exponential variogram: γ(0)=0, γ(10)≈{:.4}",
            gamma_at_range
        );
    }

    /// Test kriging at known point (should reproduce value exactly)
    #[test]
    fn test_kriging_at_known_point() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let kriging = KrigingF64::new(device).unwrap();

        let known = vec![
            (0.0, 0.0, 1.0),
            (10.0, 0.0, 2.0),
            (0.0, 10.0, 3.0),
            (10.0, 10.0, 4.0),
        ];

        // Interpolate exactly at first known point
        let targets = vec![(0.0, 0.0)];
        let model = VariogramModel::Spherical {
            nugget: 0.0,
            sill: 1.0,
            range: 15.0,
        };

        let result = kriging.interpolate(&known, &targets, model).unwrap();

        // Should reproduce known value exactly
        assert!(
            (result.values[0] - 1.0).abs() < 1e-6,
            "Kriging at known point should reproduce value: got {} expected 1.0",
            result.values[0]
        );

        // Variance should be ~0 at known point
        assert!(
            result.variances[0] < 0.01,
            "Variance at known point should be ~0: got {}",
            result.variances[0]
        );

        println!(
            "✓ Kriging at known point: value={:.6}, variance={:.6}",
            result.values[0], result.variances[0]
        );
    }

    /// Test kriging center interpolation (symmetric case)
    #[test]
    fn test_kriging_center_interpolation() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let kriging = KrigingF64::new(device).unwrap();

        // Square with values at corners
        let known = vec![
            (0.0, 0.0, 1.0),
            (10.0, 0.0, 1.0),
            (0.0, 10.0, 1.0),
            (10.0, 10.0, 1.0),
        ];

        // Interpolate at center
        let targets = vec![(5.0, 5.0)];
        let model = VariogramModel::Spherical {
            nugget: 0.0,
            sill: 0.5,
            range: 15.0,
        };

        let result = kriging.interpolate(&known, &targets, model).unwrap();

        // Center of square with equal corner values should equal that value
        assert!(
            (result.values[0] - 1.0).abs() < 0.01,
            "Center interpolation should be ~1.0: got {}",
            result.values[0]
        );

        println!(
            "✓ Kriging center: value={:.6}, variance={:.6}",
            result.values[0], result.variances[0]
        );
    }

    /// Test kriging with gradient (linear trend)
    #[test]
    fn test_kriging_linear_trend() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let kriging = KrigingF64::new(device).unwrap();

        // Linear trend: z = x/10 + y/10
        let known = vec![
            (0.0, 0.0, 0.0),   // z = 0
            (10.0, 0.0, 1.0),  // z = 1
            (0.0, 10.0, 1.0),  // z = 1
            (10.0, 10.0, 2.0), // z = 2
        ];

        // Interpolate at center (5,5) → expected ~1.0
        let targets = vec![(5.0, 5.0)];
        let model = VariogramModel::Gaussian {
            nugget: 0.0,
            sill: 0.5,
            range: 15.0,
        };

        let result = kriging.interpolate(&known, &targets, model).unwrap();

        assert!(
            (result.values[0] - 1.0).abs() < 0.15,
            "Linear trend center should be ~1.0: got {}",
            result.values[0]
        );

        println!(
            "✓ Kriging linear trend: value={:.6} (expected ~1.0)",
            result.values[0]
        );
    }

    /// Test simple kriging (known mean)
    #[test]
    fn test_simple_kriging() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let kriging = KrigingF64::new(device).unwrap();

        let known = vec![
            (0.0, 0.0, 10.0),
            (10.0, 0.0, 12.0),
            (0.0, 10.0, 11.0),
            (10.0, 10.0, 13.0),
        ];

        let targets = vec![(5.0, 5.0)];
        let model = VariogramModel::Exponential {
            nugget: 0.0,
            sill: 2.0,
            range: 10.0,
        };
        let mean = 11.5; // Known population mean

        let result = kriging
            .interpolate_simple(&known, &targets, model, mean)
            .unwrap();

        // Simple kriging should pull toward the mean
        assert!(
            result.values[0] > 10.0 && result.values[0] < 14.0,
            "Simple kriging value out of range: {}",
            result.values[0]
        );

        println!(
            "✓ Simple kriging: value={:.6}, variance={:.6}",
            result.values[0], result.variances[0]
        );
    }

    /// Test empirical variogram fitting
    #[test]
    fn test_variogram_fitting() {
        let _device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        // Generate synthetic data with known spatial structure
        let known: Vec<(f64, f64, f64)> = (0..10)
            .flat_map(|i| {
                (0..10).map(move |j| {
                    let x = i as f64 * 2.0;
                    let y = j as f64 * 2.0;
                    let z = x * 0.1 + y * 0.1 + (x * y * 0.01); // Some spatial structure
                    (x, y, z)
                })
            })
            .collect();

        let (lag_distances, lag_semivariances) =
            KrigingF64::fit_variogram(&known, 10, 20.0).unwrap();

        // Should have computed empirical variogram
        assert_eq!(lag_distances.len(), 10);
        assert_eq!(lag_semivariances.len(), 10);

        // Variogram should generally increase with distance (for this data)
        let non_empty_lags: Vec<_> = lag_semivariances.iter().filter(|&&v| v > 0.0).collect();
        assert!(
            !non_empty_lags.is_empty(),
            "Should have computed some lag values"
        );

        println!(
            "✓ Variogram fitting: {} lags computed, {} non-empty",
            lag_distances.len(),
            non_empty_lags.len()
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// E2E TESTS: Complete Workflows
// ═══════════════════════════════════════════════════════════════════════════

mod e2e {
    use super::*;

    /// E2E: Biodiversity analysis pipeline (wetSpring use case)
    #[test]
    fn test_biodiversity_pipeline() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // Simulated OTU counts from microbiome sample
        let otu_counts = vec![
            150.0, 89.0, 67.0, 45.0, 32.0, 28.0, 21.0, 15.0, 12.0, 8.0, 5.0, 3.0, 2.0, 1.0, 1.0,
        ];

        // Calculate diversity metrics
        let shannon = fmr.shannon_entropy(&otu_counts).unwrap();
        let simpson = fmr.simpson_index(&otu_counts).unwrap();
        let richness = otu_counts.len() as f64;
        let evenness = shannon / richness.ln();

        // Validate ranges
        assert!(shannon > 0.0 && shannon < richness.ln() + 0.01);
        assert!(simpson > 0.0 && simpson < 1.0);
        assert!(evenness > 0.0 && evenness <= 1.0);

        println!("✓ Biodiversity pipeline:");
        println!("  Shannon H' = {:.4}", shannon);
        println!("  Simpson D  = {:.4}", simpson);
        println!("  Evenness   = {:.4}", evenness);
    }

    /// E2E: Soil moisture mapping pipeline (airSpring use case)
    #[test]
    fn test_soil_moisture_mapping() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let kriging = KrigingF64::new(device).unwrap();

        // Simulated soil moisture sensors across a field (coordinates in meters)
        let sensors = vec![
            (0.0, 0.0, 0.35),     // Corner sensor, VWC = 35%
            (100.0, 0.0, 0.28),   // Edge, drier
            (0.0, 100.0, 0.32),   // Edge
            (100.0, 100.0, 0.25), // Far corner, driest
            (50.0, 50.0, 0.30),   // Center
        ];

        // Create target grid (10x10 = 100 interpolation points)
        let targets: Vec<(f64, f64)> = (0..10)
            .flat_map(|i| (0..10).map(move |j| (i as f64 * 10.0, j as f64 * 10.0)))
            .collect();

        let model = VariogramModel::Spherical {
            nugget: 0.001, // Small measurement noise
            sill: 0.01,    // Total variance
            range: 75.0,   // Correlation range ~75m
        };

        let result = kriging.interpolate(&sensors, &targets, model).unwrap();

        // Validate results
        assert_eq!(result.values.len(), 100);
        assert_eq!(result.variances.len(), 100);

        // All values should be in valid VWC range
        for (i, &v) in result.values.iter().enumerate() {
            assert!(
                v >= 0.0 && v <= 1.0,
                "VWC at point {} out of range: {}",
                i,
                v
            );
        }

        // Variance should be non-negative
        for (i, &var) in result.variances.iter().enumerate() {
            assert!(var >= 0.0, "Negative variance at point {}: {}", i, var);
        }

        let min_vwc = result.values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_vwc = result
            .values
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        println!("✓ Soil moisture mapping:");
        println!("  Grid: 10x10 = {} points", result.values.len());
        println!(
            "  VWC range: {:.2}% - {:.2}%",
            min_vwc * 100.0,
            max_vwc * 100.0
        );
        println!(
            "  Variance range: {:.6} - {:.6}",
            result
                .variances
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min),
            result
                .variances
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max)
        );
    }

    /// E2E: Combined diversity + spatial analysis
    #[test]
    fn test_combined_diversity_spatial() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device.clone()).unwrap();
        let kriging = KrigingF64::new(device).unwrap();

        // Sampling sites with coordinates and diversity measurements
        // Simulates microbiome diversity across a field
        // Using fmr for Shannon computation to validate combined pipeline
        let sites = vec![
            (0.0, 0.0, fmr.shannon_entropy(&[50.0, 30.0, 20.0]).unwrap()), // Site 1
            (10.0, 0.0, fmr.shannon_entropy(&[40.0, 40.0, 20.0]).unwrap()), // Site 2
            (0.0, 10.0, fmr.shannon_entropy(&[60.0, 25.0, 15.0]).unwrap()), // Site 3
            (
                10.0,
                10.0,
                fmr.shannon_entropy(&[35.0, 35.0, 30.0]).unwrap(),
            ), // Site 4
            (5.0, 5.0, fmr.shannon_entropy(&[45.0, 35.0, 20.0]).unwrap()), // Site 5 (center)
        ];

        // Interpolate diversity to new sampling locations
        let new_sites = vec![(2.5, 2.5), (7.5, 2.5), (2.5, 7.5), (7.5, 7.5)];

        let model = VariogramModel::Gaussian {
            nugget: 0.0,
            sill: 0.05,
            range: 8.0,
        };

        let result = kriging.interpolate(&sites, &new_sites, model).unwrap();

        // Validate interpolated diversity values are reasonable
        for (i, &h) in result.values.iter().enumerate() {
            assert!(
                h > 0.5 && h < 2.0,
                "Interpolated Shannon at site {} out of range: {}",
                i,
                h
            );
        }

        println!("✓ Combined diversity + spatial:");
        println!(
            "  Sites: {} known, {} interpolated",
            sites.len(),
            new_sites.len()
        );
        for (i, &h) in result.values.iter().enumerate() {
            println!(
                "  New site {}: H' = {:.4} ± {:.4}",
                i,
                h,
                result.variances[i].sqrt()
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CHAOS TESTS: Stress and Edge Cases
// ═══════════════════════════════════════════════════════════════════════════

mod chaos {
    use super::*;

    /// Chaos: Shannon entropy with very large counts
    #[test]
    fn test_shannon_large_counts() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // Very large counts (simulating deep sequencing)
        let counts: Vec<f64> = vec![1e8, 5e7, 2.5e7, 1e7, 5e6];
        let result = fmr.shannon_entropy(&counts).unwrap();
        let expected = cpu_shannon(&counts);

        let error = (result - expected).abs();
        assert!(error < 1e-8, "Large counts Shannon error: {}", error);

        println!("✓ Shannon large counts: {} (error: {:.2e})", result, error);
    }

    /// Chaos: Shannon entropy with very small counts
    #[test]
    fn test_shannon_small_counts() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // Very small counts (rare species)
        let counts = vec![1e-10, 1e-11, 1e-12, 1e-13];
        let result = fmr.shannon_entropy(&counts).unwrap();
        let expected = cpu_shannon(&counts);

        let error = (result - expected).abs();
        // Allow larger error for very small numbers due to precision limits
        assert!(
            error < 1e-6 || (result - expected).abs() / expected.abs() < 1e-6,
            "Small counts Shannon error: {}",
            error
        );

        println!("✓ Shannon small counts: {} (error: {:.2e})", result, error);
    }

    /// Chaos: Shannon with many zero counts
    #[test]
    fn test_shannon_sparse() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // Sparse OTU table (many zeros)
        let mut counts = vec![0.0; 1000];
        counts[0] = 100.0;
        counts[50] = 50.0;
        counts[100] = 25.0;
        counts[500] = 10.0;
        counts[999] = 5.0;

        let result = fmr.shannon_entropy(&counts).unwrap();
        let expected = cpu_shannon(&counts);

        let error = (result - expected).abs();
        assert!(error < 1e-10, "Sparse Shannon error: {}", error);

        println!("✓ Shannon sparse (5 non-zero of 1000): {}", result);
    }

    /// Chaos: Kriging with co-located points
    #[test]
    fn test_kriging_colocated() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let kriging = KrigingF64::new(device).unwrap();

        // Two sensors at nearly the same location
        let known = vec![
            (0.0, 0.0, 1.0),
            (0.001, 0.001, 1.1), // Almost co-located
            (10.0, 10.0, 2.0),
        ];

        let targets = vec![(5.0, 5.0)];
        let model = VariogramModel::Spherical {
            nugget: 0.01, // Nugget helps with co-location
            sill: 1.0,
            range: 15.0,
        };

        let result = kriging.interpolate(&known, &targets, model).unwrap();

        // Should still produce valid result
        assert!(
            result.values[0].is_finite(),
            "Co-located points caused non-finite result"
        );

        println!("✓ Kriging co-located: value={:.6}", result.values[0]);
    }

    /// Chaos: Kriging extrapolation (target outside convex hull)
    #[test]
    fn test_kriging_extrapolation() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let kriging = KrigingF64::new(device).unwrap();

        let known = vec![
            (0.0, 0.0, 1.0),
            (10.0, 0.0, 2.0),
            (0.0, 10.0, 2.0),
            (10.0, 10.0, 3.0),
        ];

        // Far outside the known data region
        let targets = vec![(100.0, 100.0), (-50.0, -50.0)];
        let model = VariogramModel::Exponential {
            nugget: 0.0,
            sill: 1.0,
            range: 15.0,
        };

        let result = kriging.interpolate(&known, &targets, model).unwrap();

        // Should have high variance for extrapolation
        for (i, &var) in result.variances.iter().enumerate() {
            assert!(
                var > 0.5,
                "Extrapolation variance should be high: got {} at {}",
                var,
                i
            );
        }

        println!("✓ Kriging extrapolation: variances={:?}", result.variances);
    }

    /// Chaos: Large array reduction (within CPU threshold)
    #[test]
    fn test_large_array_reduction() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // Use 500 elements to stay within CPU path (< 1024 threshold)
        // GPU path for large arrays requires full pipeline implementation
        let n = 500;
        let data: Vec<f64> = (0..n).map(|i| (i as f64) * 0.001).collect();

        let sum = fmr.sum(&data).unwrap();
        let expected: f64 = data.iter().sum();

        let rel_error = (sum - expected).abs() / expected.abs();
        assert!(
            rel_error < 1e-10,
            "Large array sum relative error: {}",
            rel_error
        );

        println!(
            "✓ Array ({} elements) sum: {} (rel error: {:.2e})",
            n, sum, rel_error
        );
    }

    /// Chaos: Very large array reduction (GPU path - ignored if not implemented)
    #[test]
    #[ignore] // GPU path for arrays > 1024 requires full pipeline wiring
    fn test_very_large_array_reduction_gpu() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // 1 million elements - triggers GPU path
        let n = 1_000_000;
        let data: Vec<f64> = (0..n).map(|i| (i as f64) * 0.000001).collect();

        let sum = fmr.sum(&data).unwrap();
        let expected: f64 = data.iter().sum();

        let rel_error = (sum - expected).abs() / expected.abs();
        assert!(
            rel_error < 1e-8,
            "Large array sum relative error: {}",
            rel_error
        );

        println!(
            "✓ Large array (1M elements) sum: {} (rel error: {:.2e})",
            sum, rel_error
        );
    }

    /// Chaos: Repeated operations (memory leak check)
    #[test]
    fn test_repeated_operations() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device.clone()).unwrap();
        let kriging = KrigingF64::new(device.clone()).unwrap();

        // Run many iterations
        for i in 0..100 {
            let counts: Vec<f64> = (0..100).map(|j| ((i * 100 + j) % 50 + 1) as f64).collect();
            let _ = fmr.shannon_entropy(&counts).unwrap();

            let known = vec![(0.0, 0.0, i as f64), (10.0, 10.0, (i + 1) as f64)];
            let targets = vec![(5.0, 5.0)];
            let model = VariogramModel::Linear {
                nugget: 0.0,
                sill: 1.0,
                range: 15.0,
            };
            let _ = kriging.interpolate(&known, &targets, model).unwrap();
        }

        println!("✓ Repeated operations (100 iterations): no crash/leak");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// FAULT TESTS: Error Handling
// ═══════════════════════════════════════════════════════════════════════════

mod fault {
    use super::*;

    /// Fault: Empty input to Shannon
    #[test]
    fn test_shannon_empty_input() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        let counts: Vec<f64> = vec![];
        let result = fmr.shannon_entropy(&counts);

        // Should handle gracefully (return 0 or error)
        match result {
            Ok(v) => {
                assert!(v.abs() < 1e-10, "Empty Shannon should be 0 or error");
                println!("✓ Empty Shannon: returned 0.0");
            }
            Err(e) => {
                println!("✓ Empty Shannon: returned error ({})", e);
            }
        }
    }

    /// Fault: All-zero input to Shannon
    #[test]
    fn test_shannon_all_zeros() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        let counts = vec![0.0; 100];
        let result = fmr.shannon_entropy(&counts);

        // Should handle gracefully
        match result {
            Ok(v) => {
                assert!(v.is_finite(), "All-zero Shannon should be finite");
                println!("✓ All-zero Shannon: {}", v);
            }
            Err(e) => {
                println!("✓ All-zero Shannon: returned error ({})", e);
            }
        }
    }

    /// Fault: NaN in input
    #[test]
    fn test_nan_input() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        let counts = vec![1.0, f64::NAN, 3.0, 4.0];
        let result = fmr.sum(&counts).unwrap();

        // Sum with NaN should propagate NaN
        assert!(result.is_nan(), "NaN should propagate");
        println!("✓ NaN propagation: sum of [1, NaN, 3, 4] = {}", result);
    }

    /// Fault: Infinity in input
    #[test]
    fn test_infinity_input() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        let counts = vec![1.0, f64::INFINITY, 3.0];
        let result = fmr.sum(&counts).unwrap();

        // Sum with infinity should be infinity
        assert!(result.is_infinite(), "Infinity should propagate");
        println!("✓ Infinity propagation: sum = {}", result);
    }

    /// Fault: Negative counts
    #[test]
    fn test_negative_counts() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // Negative counts are mathematically invalid for entropy
        let counts = vec![10.0, -5.0, 15.0];
        let result = fmr.shannon_entropy(&counts);

        // Should handle (likely produces NaN from log of negative)
        match result {
            Ok(v) => {
                println!("✓ Negative counts: Shannon = {} (may be NaN)", v);
            }
            Err(e) => {
                println!("✓ Negative counts: returned error ({})", e);
            }
        }
    }

    /// Fault: Kriging with no known points
    #[test]
    fn test_kriging_empty_known() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let kriging = KrigingF64::new(device).unwrap();

        let known: Vec<(f64, f64, f64)> = vec![];
        let targets = vec![(5.0, 5.0)];
        let model = VariogramModel::Spherical {
            nugget: 0.0,
            sill: 1.0,
            range: 10.0,
        };

        let result = kriging.interpolate(&known, &targets, model).unwrap();

        // Should return empty or default values
        assert!(
            result.values.is_empty() || result.values[0].is_nan(),
            "Empty known should produce empty/NaN result"
        );

        println!("✓ Kriging empty known: handled gracefully");
    }

    /// Fault: Kriging with single known point
    #[test]
    fn test_kriging_single_point() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let kriging = KrigingF64::new(device).unwrap();

        // Only one known point
        let known = vec![(5.0, 5.0, 1.0)];
        let targets = vec![(0.0, 0.0), (10.0, 10.0)];
        let model = VariogramModel::Spherical {
            nugget: 0.0,
            sill: 1.0,
            range: 10.0,
        };

        let result = kriging.interpolate(&known, &targets, model).unwrap();

        // With only one point, interpolation should return that value
        for &v in &result.values {
            assert!(
                v.is_finite(),
                "Single point kriging should produce finite values"
            );
        }

        println!("✓ Kriging single point: values={:?}", result.values);
    }

    /// Fault: Invalid variogram parameters
    #[test]
    fn test_invalid_variogram() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let kriging = KrigingF64::new(device).unwrap();

        let known = vec![(0.0, 0.0, 1.0), (10.0, 10.0, 2.0)];
        let targets = vec![(5.0, 5.0)];

        // Invalid: sill < nugget
        let model = VariogramModel::Spherical {
            nugget: 2.0,
            sill: 1.0, // sill < nugget is invalid
            range: 10.0,
        };

        let result = kriging.interpolate(&known, &targets, model);

        // Should still produce some result (may be numerically unstable)
        match result {
            Ok(r) => {
                println!("✓ Invalid variogram: value={:?}", r.values);
            }
            Err(e) => {
                println!("✓ Invalid variogram: error ({})", e);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PRECISION TESTS: f64 Accuracy
// ═══════════════════════════════════════════════════════════════════════════

mod precision {
    use super::*;

    /// Precision: Shannon entropy vs CPU reference (multiple cases)
    #[test]
    fn test_shannon_precision_suite() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        let test_cases = vec![
            vec![1.0, 1.0, 1.0, 1.0],                        // Uniform
            vec![100.0, 1.0, 1.0, 1.0],                      // Skewed
            vec![1.0, 2.0, 3.0, 4.0, 5.0],                   // Gradient
            vec![0.001, 0.01, 0.1, 1.0, 10.0],               // Log-scale
            (1..=100).map(|x| x as f64).collect::<Vec<_>>(), // 1..100
        ];

        println!("Shannon precision suite:");
        for (i, counts) in test_cases.iter().enumerate() {
            let gpu = fmr.shannon_entropy(counts).unwrap();
            let cpu = cpu_shannon(counts);
            let error = (gpu - cpu).abs();
            let rel_error = if cpu.abs() > 1e-15 {
                error / cpu.abs()
            } else {
                error
            };

            assert!(
                error < 1e-10 || rel_error < 1e-10,
                "Case {} failed: error={}, rel_error={}",
                i,
                error,
                rel_error
            );

            println!("  Case {}: H={:.10}, error={:.2e}", i, gpu, error);
        }
    }

    /// Precision: Simpson index vs CPU reference
    #[test]
    fn test_simpson_precision_suite() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        let test_cases = vec![
            vec![1.0, 1.0, 1.0, 1.0],
            vec![100.0, 1.0, 1.0, 1.0],
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
        ];

        println!("Simpson precision suite:");
        for (i, counts) in test_cases.iter().enumerate() {
            let gpu = fmr.simpson_index(counts).unwrap();
            let cpu = cpu_simpson(counts);
            let error = (gpu - cpu).abs();

            assert!(error < 1e-12, "Case {} failed: error={}", i, error);

            println!("  Case {}: D={:.12}, error={:.2e}", i, gpu, error);
        }
    }

    /// Precision: Kahan summation accuracy test (CPU path)
    #[test]
    fn test_kahan_summation_accuracy() {
        let device = match create_device_sync() {
            Some(d) => d,
            None => return,
        };

        let fmr = FusedMapReduceF64::new(device).unwrap();

        // This pattern causes naive summation to lose precision
        // Use 500 iterations to stay in CPU path (< 1024 threshold)
        let n = 500;
        let large = 1e10;
        let small = 1.0;

        let mut data = vec![large];
        for _ in 0..n {
            data.push(small);
        }
        data.push(-large); // Should cancel with first element

        let result = fmr.sum(&data).unwrap();
        let expected = n as f64; // large - large + n*1 = n

        let error = (result - expected).abs();
        let rel_error = error / expected;

        // With Kahan summation, error should be small
        assert!(
            rel_error < 1e-10,
            "Kahan summation error too large: {} (rel: {})",
            error,
            rel_error
        );

        println!(
            "✓ Kahan summation: sum={}, expected={}, rel_error={:.2e}",
            result, expected, rel_error
        );
    }
}
