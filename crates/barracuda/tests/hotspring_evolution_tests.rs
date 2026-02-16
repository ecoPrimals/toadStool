//! Comprehensive Tests for hotSpring Evolution Primitives
//!
//! Unit, E2E, Chaos, and Fault tests for:
//! - Mixing module (LinearMixer, BroydenMixer)
//! - Grid module (Gradient1D, Gradient2D, Laplacian, Cylindrical)
//! - Special functions (Hermite, Laguerre f64)
//!
//! These primitives were absorbed from hotSpring and validated by
//! 169/169 nuclear EOS acceptance checks on consumer GPU.

use barracuda::device::WgpuDevice;
use barracuda::error::BarracudaError;
use barracuda::ops::grid::{
    CylindricalGradient, CylindricalLaplacian, Gradient1D, Gradient2D, Laplacian2D,
};
use barracuda::ops::mixing::{BroydenMixer, LinearMixer, MixingParams};
use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS: Mixing Module
// ═══════════════════════════════════════════════════════════════════════════

mod mixing_unit {
    use super::*;

    /// Test linear mixing with α=0.5 (simple average)
    #[tokio::test]
    async fn test_linear_mixer_alpha_half() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return, // Skip if no GPU
        };

        let params = MixingParams {
            alpha: 0.5,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 1024, params).unwrap();

        let x_old = vec![0.0; 1024];
        let x_computed = vec![2.0; 1024];

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // Expected: 0.5 * 0.0 + 0.5 * 2.0 = 1.0
        for (i, val) in result.iter().enumerate() {
            assert!(
                (val - 1.0).abs() < 1e-10,
                "At index {}: expected 1.0, got {}",
                i,
                val
            );
        }
    }

    /// Test linear mixing with α=0.3 (conservative mixing)
    #[tokio::test]
    async fn test_linear_mixer_alpha_conservative() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.3,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 512, params).unwrap();

        let x_old = vec![10.0; 512];
        let x_computed = vec![20.0; 512];

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // Expected: 0.7 * 10.0 + 0.3 * 20.0 = 7.0 + 6.0 = 13.0
        for (i, val) in result.iter().enumerate() {
            assert!(
                (val - 13.0).abs() < 1e-10,
                "At index {}: expected 13.0, got {}",
                i,
                val
            );
        }
    }

    /// Test linear mixing with α=1.0 (full replacement)
    #[tokio::test]
    async fn test_linear_mixer_alpha_one() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 1.0,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 256, params).unwrap();

        let x_old = vec![1.0; 256];
        let x_computed = vec![5.0; 256];

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // Expected: 0.0 * 1.0 + 1.0 * 5.0 = 5.0
        for (i, val) in result.iter().enumerate() {
            assert!(
                (val - 5.0).abs() < 1e-10,
                "At index {}: expected 5.0, got {}",
                i,
                val
            );
        }
    }

    /// Test linear mixing with α=0.0 (no change)
    #[tokio::test]
    async fn test_linear_mixer_alpha_zero() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.0,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 128, params).unwrap();

        let x_old = vec![3.14159; 128];
        let x_computed = vec![99.0; 128];

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // Expected: 1.0 * 3.14159 + 0.0 * 99.0 = 3.14159
        for (i, val) in result.iter().enumerate() {
            assert!(
                (val - 3.14159).abs() < 1e-10,
                "At index {}: expected 3.14159, got {}",
                i,
                val
            );
        }
    }

    /// Test linear mixing with varying values
    #[tokio::test]
    async fn test_linear_mixer_varying_values() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.4,
            ..Default::default()
        };
        let n = 1000;
        let mixer = LinearMixer::new(device, n, params).unwrap();

        let x_old: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let x_computed: Vec<f64> = (0..n).map(|i| (i * 2) as f64).collect();

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // Expected: 0.6 * i + 0.4 * 2i = 0.6i + 0.8i = 1.4i
        for (i, val) in result.iter().enumerate() {
            let expected = 1.4 * i as f64;
            assert!(
                (val - expected).abs() < 1e-9,
                "At index {}: expected {}, got {}",
                i,
                expected,
                val
            );
        }
    }

    /// Test Broyden mixer initialization
    #[tokio::test]
    async fn test_broyden_mixer_creation() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.4,
            n_warmup: 3,
            ..Default::default()
        };
        let mixer = BroydenMixer::new(device, 1024, 5, params);
        assert!(mixer.is_ok(), "BroydenMixer creation should succeed");
    }

    /// Test Broyden mixer warmup phase (uses linear mixing)
    #[tokio::test]
    async fn test_broyden_mixer_warmup() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.5,
            n_warmup: 5,
            ..Default::default()
        };
        let mut mixer = BroydenMixer::new(device, 100, 5, params).unwrap();

        let x_old = vec![1.0; 100];
        let x_computed = vec![2.0; 100];

        // First few iterations should use linear mixing
        for _ in 0..3 {
            let result = mixer.mix(&x_old, &x_computed).await.unwrap();
            // During warmup, linear mixing: 0.5 * 1.0 + 0.5 * 2.0 = 1.5
            for val in &result {
                assert!((val - 1.5).abs() < 1e-10);
            }
        }
    }

    /// Test Broyden mixer reset
    #[tokio::test]
    async fn test_broyden_mixer_reset() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams::default();
        let mut mixer = BroydenMixer::new(device, 50, 3, params).unwrap();

        let x_old = vec![1.0; 50];
        let x_computed = vec![2.0; 50];

        // Mix a few times
        for _ in 0..5 {
            let _ = mixer.mix(&x_old, &x_computed).await;
        }

        // Reset and verify it behaves like fresh
        mixer.reset();
        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // After reset, should be back to iteration 1 (warmup)
        // Linear mixing with default alpha=0.4: 0.6 * 1.0 + 0.4 * 2.0 = 1.4
        for val in &result {
            assert!((val - 1.4).abs() < 1e-10);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS: Grid Module
// ═══════════════════════════════════════════════════════════════════════════

mod grid_unit {
    use super::*;

    /// Test 1D gradient of linear function f(x) = x → df/dx = 1
    #[tokio::test]
    async fn test_gradient_1d_linear() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let n = 100;
        let dx = 0.1;
        let grad = Gradient1D::new(device, n, dx).unwrap();

        // f(x) = x (linear)
        let input: Vec<f64> = (0..n).map(|i| i as f64 * dx).collect();
        let result = grad.compute(&input).await.unwrap();

        // Interior points should have gradient ≈ 1.0
        for i in 1..n - 1 {
            assert!(
                (result[i] - 1.0).abs() < 1e-10,
                "At i={}: expected 1.0, got {}",
                i,
                result[i]
            );
        }
    }

    /// Test 1D gradient of quadratic function f(x) = x² → df/dx = 2x
    #[tokio::test]
    async fn test_gradient_1d_quadratic() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let n = 100;
        let dx = 0.1;
        let grad = Gradient1D::new(device, n, dx).unwrap();

        // f(x) = x²
        let input: Vec<f64> = (0..n).map(|i| (i as f64 * dx).powi(2)).collect();
        let result = grad.compute(&input).await.unwrap();

        // Interior points: df/dx = 2x with O(dx²) error
        for i in 1..n - 1 {
            let x = i as f64 * dx;
            let expected = 2.0 * x;
            let error = (result[i] - expected).abs();
            assert!(
                error < 0.02,
                "At i={}: expected {}, got {}, error={}",
                i,
                expected,
                result[i],
                error
            );
        }
    }

    /// Test 1D gradient of cubic function f(x) = x³ → df/dx = 3x²
    #[tokio::test]
    async fn test_gradient_1d_cubic() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let n = 200;
        let dx = 0.05;
        let grad = Gradient1D::new(device, n, dx).unwrap();

        // f(x) = x³
        let input: Vec<f64> = (0..n).map(|i| (i as f64 * dx).powi(3)).collect();
        let result = grad.compute(&input).await.unwrap();

        // Interior points: df/dx = 3x²
        for i in 2..n - 2 {
            let x = i as f64 * dx;
            let expected = 3.0 * x * x;
            let rel_error = if expected.abs() > 1e-10 {
                (result[i] - expected).abs() / expected.abs()
            } else {
                (result[i] - expected).abs()
            };
            assert!(
                rel_error < 0.05,
                "At i={} (x={}): expected {}, got {}, rel_error={}",
                i,
                x,
                expected,
                result[i],
                rel_error
            );
        }
    }

    /// Test 1D gradient of sine function f(x) = sin(x) → df/dx = cos(x)
    #[tokio::test]
    async fn test_gradient_1d_sine() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let n = 200;
        let dx = 0.05;
        let grad = Gradient1D::new(device, n, dx).unwrap();

        // f(x) = sin(x)
        let input: Vec<f64> = (0..n).map(|i| (i as f64 * dx).sin()).collect();
        let result = grad.compute(&input).await.unwrap();

        // Interior points: df/dx = cos(x)
        for i in 5..n - 5 {
            let x = i as f64 * dx;
            let expected = x.cos();
            let error = (result[i] - expected).abs();
            // Central difference error is O(dx²) ≈ 0.0025
            assert!(
                error < 0.01,
                "At i={} (x={}): expected {}, got {}, error={}",
                i,
                x,
                expected,
                result[i],
                error
            );
        }
    }

    /// Test 2D gradient struct creation
    #[tokio::test]
    async fn test_gradient_2d_creation() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let grad = Gradient2D::new(device, 64, 64, 0.1, 0.1);
        assert!(grad.is_ok());
        assert_eq!(grad.unwrap().shape(), (64, 64));
    }

    /// Test 2D Laplacian struct creation
    #[tokio::test]
    async fn test_laplacian_2d_creation() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let lap = Laplacian2D::new(device, 100, 100, 0.05, 0.05);
        assert!(lap.is_ok());
        assert_eq!(lap.unwrap().shape(), (100, 100));
    }

    /// Test cylindrical gradient struct creation
    #[tokio::test]
    async fn test_cylindrical_gradient_creation() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let cyl = CylindricalGradient::new(device, 50, 100, 0.1, 0.1, -5.0);
        assert!(cyl.is_ok());
        assert_eq!(cyl.unwrap().shape(), (50, 100));
    }

    /// Test cylindrical Laplacian struct creation
    #[tokio::test]
    async fn test_cylindrical_laplacian_creation() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let cyl = CylindricalLaplacian::new(device, 50, 100, 0.1, 0.1, -5.0);
        assert!(cyl.is_ok());
        assert_eq!(cyl.unwrap().shape(), (50, 100));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// E2E TESTS: SCF Convergence Simulation
// ═══════════════════════════════════════════════════════════════════════════

mod e2e_scf {
    use super::*;

    /// Simulate a simple SCF-like fixed-point iteration
    /// f(x) = 0.9*x + 0.1  converges to x=1
    #[tokio::test]
    async fn test_linear_mixing_scf_convergence() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.5,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 1, params).unwrap();

        let mut x = vec![0.0]; // Start from 0
        let target = 1.0;

        for iteration in 0..50 {
            // SCF "output": f(x) = 0.9*x + 0.1 → fixed point at x=1
            let f_x: Vec<f64> = x.iter().map(|&xi| 0.9 * xi + 0.1).collect();

            // Mix
            x = mixer.mix(&x, &f_x).await.unwrap();

            // Check convergence
            let error = (x[0] - target).abs();
            if error < 1e-10 {
                println!("Converged in {} iterations", iteration + 1);
                return;
            }
        }

        // Should have converged
        let final_error = (x[0] - target).abs();
        assert!(
            final_error < 1e-6,
            "SCF did not converge: x={}, error={}",
            x[0],
            final_error
        );
    }

    /// Simulate SCF with multiple degrees of freedom
    #[tokio::test]
    async fn test_linear_mixing_scf_multidim() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.4,
            ..Default::default()
        };
        let n = 10;
        let mixer = LinearMixer::new(device, n, params).unwrap();

        // Start from zeros
        let mut x = vec![0.0; n];

        // Target: x[i] = i + 1
        let target: Vec<f64> = (1..=n).map(|i| i as f64).collect();

        for iteration in 0..100 {
            // SCF "output": move toward target with contraction
            let f_x: Vec<f64> = x
                .iter()
                .zip(&target)
                .map(|(&xi, &ti)| 0.8 * xi + 0.2 * ti)
                .collect();

            // Mix
            x = mixer.mix(&x, &f_x).await.unwrap();

            // Check convergence
            let max_error: f64 = x
                .iter()
                .zip(&target)
                .map(|(&xi, &ti)| (xi - ti).abs())
                .fold(0.0_f64, f64::max);

            if max_error < 1e-10 {
                println!("Multi-dim SCF converged in {} iterations", iteration + 1);
                return;
            }
        }

        // Verify final state
        let max_error: f64 = x
            .iter()
            .zip(&target)
            .map(|(&xi, &ti)| (xi - ti).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            max_error < 1e-4,
            "Multi-dim SCF did not converge: max_error={}",
            max_error
        );
    }

    /// Test Broyden mixer in SCF-like scenario
    #[tokio::test]
    async fn test_broyden_scf_convergence() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.3,
            n_warmup: 3,
            ..Default::default()
        };
        let mut mixer = BroydenMixer::new(device, 5, 3, params).unwrap();

        let mut x = vec![0.0; 5];
        let target = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        for iteration in 0..50 {
            // Simple contraction toward target
            let f_x: Vec<f64> = x
                .iter()
                .zip(&target)
                .map(|(&xi, &ti)| 0.7 * xi + 0.3 * ti)
                .collect();

            x = mixer.mix(&x, &f_x).await.unwrap();

            let max_error: f64 = x
                .iter()
                .zip(&target)
                .map(|(&xi, &ti)| (xi - ti).abs())
                .fold(0.0_f64, f64::max);

            if max_error < 1e-8 {
                println!("Broyden SCF converged in {} iterations", iteration + 1);
                return;
            }
        }

        let max_error: f64 = x
            .iter()
            .zip(&target)
            .map(|(&xi, &ti)| (xi - ti).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            max_error < 1e-3,
            "Broyden SCF did not converge: max_error={}",
            max_error
        );
    }

    /// E2E: Gradient → Mixing loop (like in a PDE solver)
    #[tokio::test]
    async fn test_gradient_mixing_pipeline() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let n = 50;
        let dx = 0.1;

        let grad = Gradient1D::new(device.clone(), n, dx).unwrap();
        let mixer_params = MixingParams {
            alpha: 0.3,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, n, mixer_params).unwrap();

        // Initial field: f(x) = x²
        let mut field: Vec<f64> = (0..n).map(|i| (i as f64 * dx).powi(2)).collect();

        // Iterate: compute gradient, then mix with target
        let target: Vec<f64> = (0..n).map(|i| (i as f64 * dx).powi(2) * 0.5).collect();

        for _iter in 0..5 {
            // Compute gradient
            let _gradient = grad.compute(&field).await.unwrap();

            // Mix field toward target
            field = mixer.mix(&field, &target).await.unwrap();
        }

        // Field should have moved toward target
        let diff: f64 = field
            .iter()
            .zip(&target)
            .map(|(f, t)| (f - t).abs())
            .sum::<f64>()
            / n as f64;
        assert!(
            diff < 1.0,
            "Gradient-mixing pipeline did not evolve: avg_diff={}",
            diff
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CHAOS TESTS: Random/Adversarial Inputs
// ═══════════════════════════════════════════════════════════════════════════

mod chaos {
    use super::*;

    /// Test mixing with very large values
    #[tokio::test]
    async fn test_mixer_large_values() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.5,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 100, params).unwrap();

        let x_old = vec![1e100; 100];
        let x_computed = vec![2e100; 100];

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // Expected: 0.5 * 1e100 + 0.5 * 2e100 = 1.5e100
        for val in &result {
            assert!(
                (val - 1.5e100).abs() / 1.5e100 < 1e-10,
                "Large value mixing failed: got {}",
                val
            );
        }
    }

    /// Test mixing with very small values
    #[tokio::test]
    async fn test_mixer_small_values() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.5,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 100, params).unwrap();

        let x_old = vec![1e-200; 100];
        let x_computed = vec![2e-200; 100];

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // Expected: 1.5e-200
        for val in &result {
            assert!(
                (val - 1.5e-200).abs() < 1e-210,
                "Small value mixing failed: got {}",
                val
            );
        }
    }

    /// Test mixing with alternating signs
    #[tokio::test]
    async fn test_mixer_alternating_signs() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.5,
            ..Default::default()
        };
        let n = 100;
        let mixer = LinearMixer::new(device, n, params).unwrap();

        let x_old: Vec<f64> = (0..n)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let x_computed: Vec<f64> = (0..n)
            .map(|i| if i % 2 == 0 { -1.0 } else { 1.0 })
            .collect();

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // Expected: all zeros
        for (i, val) in result.iter().enumerate() {
            assert!(
                val.abs() < 1e-10,
                "At index {}: expected 0.0, got {}",
                i,
                val
            );
        }
    }

    /// Test gradient with constant function (should be zero)
    #[tokio::test]
    async fn test_gradient_constant() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let n = 100;
        let dx = 0.1;
        let grad = Gradient1D::new(device, n, dx).unwrap();

        let input = vec![42.0; n]; // Constant
        let result = grad.compute(&input).await.unwrap();

        // Gradient of constant should be zero
        for (i, val) in result.iter().enumerate() {
            assert!(
                val.abs() < 1e-10,
                "At index {}: expected 0.0, got {}",
                i,
                val
            );
        }
    }

    /// Test gradient with oscillating function
    #[tokio::test]
    async fn test_gradient_oscillating() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let n = 200;
        let dx = 0.05;
        let grad = Gradient1D::new(device, n, dx).unwrap();

        // High-frequency oscillation: sin(20x)
        let input: Vec<f64> = (0..n).map(|i| (20.0 * i as f64 * dx).sin()).collect();
        let result = grad.compute(&input).await.unwrap();

        // Gradient should be 20*cos(20x), but FD has larger error for high frequency
        // Just check it's not blowing up
        for val in &result {
            assert!(
                val.abs() < 25.0, // 20 * max(cos) + some error
                "Oscillating gradient blew up: {}",
                val
            );
        }
    }

    /// Test gradient with spike (step function)
    #[tokio::test]
    async fn test_gradient_spike() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let n = 100;
        let dx = 0.1;
        let grad = Gradient1D::new(device, n, dx).unwrap();

        // Step function: 0 for i<50, 1 for i>=50
        let input: Vec<f64> = (0..n).map(|i| if i < 50 { 0.0 } else { 1.0 }).collect();
        let result = grad.compute(&input).await.unwrap();

        // Should have a spike near i=50
        // The gradient at the jump should be ~1/(2*dx) = 5
        assert!(
            result[50].abs() > 1.0,
            "Expected spike at step, got {}",
            result[50]
        );
    }

    /// Test mixing with pseudo-random values
    #[tokio::test]
    async fn test_mixer_pseudorandom() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.5,
            ..Default::default()
        };
        let n = 1000;
        let mixer = LinearMixer::new(device, n, params).unwrap();

        // Pseudo-random via LCG (deterministic for reproducibility)
        let mut seed = 12345u64;
        let lcg = |s: &mut u64| {
            *s = s.wrapping_mul(1103515245).wrapping_add(12345) % (1u64 << 31);
            (*s as f64 / (1u64 << 31) as f64) * 2.0 - 1.0 // [-1, 1]
        };

        let x_old: Vec<f64> = (0..n).map(|_| lcg(&mut seed)).collect();
        let x_computed: Vec<f64> = (0..n).map(|_| lcg(&mut seed)).collect();

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // Verify mixing formula holds for all elements
        seed = 12345u64;
        let x_old_check: Vec<f64> = (0..n).map(|_| lcg(&mut seed)).collect();
        let x_computed_check: Vec<f64> = (0..n).map(|_| lcg(&mut seed)).collect();

        for i in 0..n {
            let expected = 0.5 * x_old_check[i] + 0.5 * x_computed_check[i];
            assert!(
                (result[i] - expected).abs() < 1e-10,
                "At index {}: expected {}, got {}",
                i,
                expected,
                result[i]
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// FAULT TESTS: Error Handling
// ═══════════════════════════════════════════════════════════════════════════

mod fault {
    use super::*;

    /// Test mixing with dimension mismatch (x_old wrong size)
    #[tokio::test]
    async fn test_mixer_dimension_mismatch_old() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams::default();
        let mixer = LinearMixer::new(device, 100, params).unwrap();

        let x_old = vec![1.0; 50]; // Wrong size!
        let x_computed = vec![2.0; 100];

        let result = mixer.mix(&x_old, &x_computed).await;
        assert!(result.is_err(), "Should fail on dimension mismatch");

        match result.unwrap_err() {
            BarracudaError::InvalidInput { message } => {
                assert!(message.contains("mismatch") || message.contains("dimension"));
            }
            other => panic!("Expected InvalidInput error, got {:?}", other),
        }
    }

    /// Test mixing with dimension mismatch (x_computed wrong size)
    #[tokio::test]
    async fn test_mixer_dimension_mismatch_computed() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams::default();
        let mixer = LinearMixer::new(device, 100, params).unwrap();

        let x_old = vec![1.0; 100];
        let x_computed = vec![2.0; 200]; // Wrong size!

        let result = mixer.mix(&x_old, &x_computed).await;
        assert!(result.is_err(), "Should fail on dimension mismatch");
    }

    /// Test gradient with input size mismatch
    #[tokio::test]
    async fn test_gradient_size_mismatch() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let grad = Gradient1D::new(device, 100, 0.1).unwrap();

        let input = vec![1.0; 50]; // Wrong size!

        let result = grad.compute(&input).await;
        assert!(result.is_err(), "Should fail on size mismatch");

        match result.unwrap_err() {
            BarracudaError::InvalidInput { message } => {
                assert!(message.contains("mismatch") || message.contains("size"));
            }
            other => panic!("Expected InvalidInput error, got {:?}", other),
        }
    }

    /// Test creating mixer with zero dimension
    #[tokio::test]
    async fn test_mixer_zero_dimension() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams::default();
        // Note: This might succeed at creation but fail at execution
        // The exact behavior depends on implementation
        let mixer = LinearMixer::new(device, 0, params);

        if let Ok(m) = mixer {
            // Try to use it with empty vectors
            let result = m.mix(&[], &[]).await;
            // Either should succeed with empty output or fail gracefully
            if let Ok(output) = result {
                assert!(output.is_empty());
            }
            // Failure is also acceptable
        }
        // Creation failure is also acceptable
    }

    /// Test gradient with empty input
    #[tokio::test]
    async fn test_gradient_empty_input() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let grad = Gradient1D::new(device, 0, 0.1);

        if let Ok(g) = grad {
            let result = g.compute(&[]).await;
            if let Ok(output) = result {
                assert!(output.is_empty());
            }
        }
    }

    /// Test mixing with NaN values (should propagate NaN)
    #[tokio::test]
    async fn test_mixer_nan_propagation() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.5,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 10, params).unwrap();

        let x_old = vec![1.0; 10];
        let mut x_computed = vec![2.0; 10];
        x_computed[5] = f64::NAN;

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // NaN should propagate
        assert!(result[5].is_nan(), "NaN should propagate through mixing");

        // Other elements should be fine
        for i in [0, 1, 2, 3, 4, 6, 7, 8, 9] {
            assert!(
                !result[i].is_nan(),
                "Non-NaN input should produce non-NaN output"
            );
        }
    }

    /// Test mixing with infinity values
    #[tokio::test]
    async fn test_mixer_infinity() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let params = MixingParams {
            alpha: 0.5,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 5, params).unwrap();

        let x_old = vec![1.0, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0];
        let x_computed = vec![2.0, 3.0, 4.0, f64::INFINITY, f64::NEG_INFINITY];

        let result = mixer.mix(&x_old, &x_computed).await.unwrap();

        // Check that infinity propagates correctly
        assert!(result[1].is_infinite(), "Infinity should propagate");
        assert!(result[2].is_infinite(), "Neg infinity should propagate");
        assert!(result[3].is_infinite(), "Infinity should propagate");
        assert!(result[4].is_infinite(), "Neg infinity should propagate");
    }

    /// Test gradient with NaN in input
    #[tokio::test]
    async fn test_gradient_nan_handling() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return,
        };

        let grad = Gradient1D::new(device, 20, 0.1).unwrap();

        let mut input: Vec<f64> = (0..20).map(|i| i as f64).collect();
        input[10] = f64::NAN;

        let result = grad.compute(&input).await.unwrap();

        // NaN should propagate to adjacent points (central difference)
        assert!(result[9].is_nan() || result[10].is_nan() || result[11].is_nan());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SPECIAL FUNCTION TESTS (CPU Reference)
// ═══════════════════════════════════════════════════════════════════════════

mod special_functions {
    /// CPU reference implementation of Hermite polynomial
    fn hermite_cpu(n: u32, x: f64) -> f64 {
        if n == 0 {
            return 1.0;
        }
        if n == 1 {
            return 2.0 * x;
        }

        let mut h_prev = 1.0;
        let mut h_curr = 2.0 * x;

        for k in 1..n {
            let h_next = 2.0 * x * h_curr - 2.0 * k as f64 * h_prev;
            h_prev = h_curr;
            h_curr = h_next;
        }

        h_curr
    }

    /// CPU reference implementation of Laguerre polynomial
    fn laguerre_cpu(n: u32, alpha: f64, x: f64) -> f64 {
        if n == 0 {
            return 1.0;
        }
        if n == 1 {
            return 1.0 + alpha - x;
        }

        let mut l_prev = 1.0;
        let mut l_curr = 1.0 + alpha - x;

        for k in 1..n {
            let kf = k as f64;
            let l_next =
                ((2.0 * kf + 1.0 + alpha - x) * l_curr - (kf + alpha) * l_prev) / (kf + 1.0);
            l_prev = l_curr;
            l_curr = l_next;
        }

        l_curr
    }

    /// Test Hermite H_0(x) = 1
    #[test]
    fn test_hermite_n0() {
        for x in [-2.0_f64, -1.0, 0.0, 0.5, 1.0, 2.0, 3.14] {
            assert!((hermite_cpu(0, x) - 1.0).abs() < 1e-15);
        }
    }

    /// Test Hermite H_1(x) = 2x
    #[test]
    fn test_hermite_n1() {
        for x in [-2.0_f64, -1.0, 0.0, 0.5, 1.0, 2.0] {
            let expected = 2.0 * x;
            assert!((hermite_cpu(1, x) - expected).abs() < 1e-14);
        }
    }

    /// Test Hermite H_2(x) = 4x² - 2
    #[test]
    fn test_hermite_n2() {
        for x in [-2.0_f64, -1.0, 0.0, 0.5, 1.0, 2.0] {
            let expected = 4.0 * x * x - 2.0;
            assert!(
                (hermite_cpu(2, x) - expected).abs() < 1e-13,
                "H_2({}) = {} vs expected {}",
                x,
                hermite_cpu(2, x),
                expected
            );
        }
    }

    /// Test Hermite H_3(x) = 8x³ - 12x
    #[test]
    fn test_hermite_n3() {
        for x in [-2.0_f64, -1.0, 0.0, 0.5, 1.0, 2.0] {
            let expected = 8.0 * x.powi(3) - 12.0 * x;
            assert!(
                (hermite_cpu(3, x) - expected).abs() < 1e-12,
                "H_3({}) = {} vs expected {}",
                x,
                hermite_cpu(3, x),
                expected
            );
        }
    }

    /// Test Hermite at x=0 (alternating pattern)
    #[test]
    fn test_hermite_at_zero() {
        // H_n(0) = 0 for odd n
        // H_n(0) = (-1)^(n/2) * n! / (n/2)! for even n
        assert!((hermite_cpu(0, 0.0) - 1.0).abs() < 1e-15); // H_0(0) = 1
        assert!((hermite_cpu(1, 0.0) - 0.0).abs() < 1e-15); // H_1(0) = 0
        assert!((hermite_cpu(2, 0.0) - (-2.0)).abs() < 1e-14); // H_2(0) = -2
        assert!((hermite_cpu(3, 0.0) - 0.0).abs() < 1e-14); // H_3(0) = 0
        assert!((hermite_cpu(4, 0.0) - 12.0).abs() < 1e-13); // H_4(0) = 12
    }

    /// Test Laguerre L_0(x) = 1
    #[test]
    fn test_laguerre_n0() {
        for x in [0.0_f64, 0.5, 1.0, 2.0, 5.0] {
            assert!((laguerre_cpu(0, 0.0, x) - 1.0).abs() < 1e-15);
        }
    }

    /// Test Laguerre L_1(x) = 1 - x
    #[test]
    fn test_laguerre_n1() {
        for x in [0.0_f64, 0.5, 1.0, 2.0, 5.0] {
            let expected = 1.0 - x;
            assert!(
                (laguerre_cpu(1, 0.0, x) - expected).abs() < 1e-14,
                "L_1({}) = {} vs expected {}",
                x,
                laguerre_cpu(1, 0.0, x),
                expected
            );
        }
    }

    /// Test Laguerre L_2(x) = (x² - 4x + 2)/2
    #[test]
    fn test_laguerre_n2() {
        for x in [0.0_f64, 0.5, 1.0, 2.0, 5.0] {
            let expected = (x * x - 4.0 * x + 2.0) / 2.0;
            assert!(
                (laguerre_cpu(2, 0.0, x) - expected).abs() < 1e-13,
                "L_2({}) = {} vs expected {}",
                x,
                laguerre_cpu(2, 0.0, x),
                expected
            );
        }
    }

    /// Test generalized Laguerre L_n^(α)(x) with α=1
    #[test]
    fn test_laguerre_alpha1() {
        // L_0^(1)(x) = 1
        assert!((laguerre_cpu(0, 1.0, 2.0) - 1.0).abs() < 1e-15);

        // L_1^(1)(x) = 2 - x
        for x in [0.0_f64, 1.0, 2.0] {
            let expected = 2.0 - x;
            assert!(
                (laguerre_cpu(1, 1.0, x) - expected).abs() < 1e-14,
                "L_1^(1)({}) = {} vs expected {}",
                x,
                laguerre_cpu(1, 1.0, x),
                expected
            );
        }
    }

    /// Test Laguerre at x=0
    #[test]
    fn test_laguerre_at_zero() {
        // L_n^(α)(0) = binomial(n+α, n) = (n+α)! / (n! α!)
        // For α=0: L_n(0) = 1
        for n in 0..10 {
            assert!(
                (laguerre_cpu(n, 0.0, 0.0) - 1.0).abs() < 1e-10,
                "L_{}(0) should be 1, got {}",
                n,
                laguerre_cpu(n, 0.0, 0.0)
            );
        }
    }

    /// Test higher-order Hermite stability
    #[test]
    fn test_hermite_high_order() {
        // H_10(1) should be a large but finite number
        let h10 = hermite_cpu(10, 1.0);
        assert!(h10.is_finite(), "H_10(1) should be finite");
        assert!(h10.abs() > 1.0, "H_10(1) should be non-trivial");

        // H_20(0.5) - check it's stable
        let h20 = hermite_cpu(20, 0.5);
        assert!(h20.is_finite(), "H_20(0.5) should be finite");
    }

    /// Test higher-order Laguerre stability
    #[test]
    fn test_laguerre_high_order() {
        // L_10(1) with α=0
        let l10 = laguerre_cpu(10, 0.0, 1.0);
        assert!(l10.is_finite(), "L_10(1) should be finite");

        // L_20^(2)(2)
        let l20 = laguerre_cpu(20, 2.0, 2.0);
        assert!(l20.is_finite(), "L_20^(2)(2) should be finite");
    }
}
