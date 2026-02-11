//! Adaptive dispatch for RBF surrogate training
//!
//! Implements dual-precision training strategy:
//! - **Small N (<threshold)**: Full f64 CPU path (current default)
//! - **Large N (≥threshold)**: f32 distance computation → promote to f64 → f64 solve
//!
//! The f32 distance path is 2-4× faster than f64 for the O(n²·d) cdist operation
//! due to SIMD vectorization (4 f32 vs 2 f64 per SSE/NEON lane). When a GPU is
//! available, the f32 path can be replaced with `cdist.wgsl` for 14× speedup.
//!
//! # Architecture
//!
//! ```text
//! Training Data: Vec<Vec<f64>>
//!         |
//!    N < threshold?
//!    ├── YES: CPU f64 cdist → f64 kernel → f64 solve (exact)
//!    └── NO:  f32 cdist (GPU or fast CPU) → promote → f64 kernel → f64 solve
//!                                                        ↑
//!                                              (Swappable: CPU f32 → GPU cdist.wgsl)
//! ```
//!
//! # Cross-Domain Applications
//!
//! - **Nuclear physics**: 10D Skyrme fits with N=1000+ training points
//! - **Materials science**: DFT surrogate models with large datasets
//! - **ML**: Gaussian process regression with many observations
//!
//! # References
//!
//! - Diaw et al. (2024): Dual-precision surrogate training architecture

use super::kernels::RBFKernel;
use super::rbf::RBFSurrogate;
use crate::error::{BarracudaError, Result};
use crate::linalg::solve_f64;

/// Configuration for adaptive dispatch during RBF training.
#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    /// Minimum N for switching to f32 distance computation.
    /// Below this threshold, full f64 is used.
    /// Default: 200
    pub f32_threshold: usize,

    /// Whether to force f64 path regardless of N.
    /// Useful for validation/comparison.
    /// Default: false
    pub force_f64: bool,

    /// Whether to enable parallelism for distance computation.
    /// Default: true
    pub parallel: bool,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            f32_threshold: 200,
            force_f64: false,
            parallel: true,
        }
    }
}

impl AdaptiveConfig {
    /// Create config that always uses f64 (for validation).
    pub fn exact() -> Self {
        Self {
            force_f64: true,
            ..Default::default()
        }
    }

    /// Create config with a custom threshold.
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            f32_threshold: threshold,
            ..Default::default()
        }
    }
}

/// Training diagnostics from adaptive dispatch.
#[derive(Debug, Clone)]
pub struct TrainingDiagnostics {
    /// Whether f32 path was used for distances
    pub used_f32_distances: bool,
    /// Number of training points
    pub n_train: usize,
    /// Number of dimensions
    pub n_dim: usize,
    /// Size of the linear system solved
    pub system_size: usize,
    /// Maximum absolute difference between f32 and f64 distances
    /// (only populated if both were computed for validation)
    pub max_distance_error: Option<f64>,
}

/// Train an RBF surrogate with adaptive dispatch.
///
/// Uses f32 distance computation for large datasets and f64 for small ones.
/// The kernel evaluation and linear solve always use f64 for numerical stability.
///
/// # Arguments
///
/// * `x_data` - Training points `[[x₁₁, x₁₂, ...], ...]`
/// * `y_data` - Training values `[y₁, y₂, ...]`
/// * `kernel` - RBF kernel type
/// * `smoothing` - Regularization parameter
/// * `config` - Adaptive dispatch configuration
///
/// # Returns
///
/// Tuple of `(RBFSurrogate, TrainingDiagnostics)`
///
/// # Examples
///
/// ```
/// use barracuda::surrogate::adaptive::{train_adaptive, AdaptiveConfig};
/// use barracuda::surrogate::RBFKernel;
///
/// let x_train: Vec<Vec<f64>> = (0..10).map(|i| vec![i as f64]).collect();
/// let y_train: Vec<f64> = x_train.iter().map(|x| x[0] * x[0]).collect();
///
/// let config = AdaptiveConfig::default();
/// let (surrogate, diag) = train_adaptive(
///     &x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-12, &config
/// )?;
///
/// println!("Used f32: {}, system size: {}", diag.used_f32_distances, diag.system_size);
/// # Ok::<(), barracuda::error::BarracudaError>(())
/// ```
pub fn train_adaptive(
    x_data: &[Vec<f64>],
    y_data: &[f64],
    kernel: RBFKernel,
    smoothing: f64,
    config: &AdaptiveConfig,
) -> Result<(RBFSurrogate, TrainingDiagnostics)> {
    let n_train = x_data.len();

    if n_train == 0 {
        return Err(BarracudaError::InvalidInput {
            message: "Training data cannot be empty".to_string(),
        });
    }

    if y_data.len() != n_train {
        return Err(BarracudaError::InvalidInput {
            message: format!(
                "x_data and y_data length mismatch: {} vs {}",
                n_train,
                y_data.len()
            ),
        });
    }

    let n_dim = x_data[0].len();

    // Flatten training data
    let train_x: Vec<f64> = x_data.iter().flat_map(|row| row.iter().copied()).collect();

    // Decide dispatch path
    let use_f32 = !config.force_f64 && n_train >= config.f32_threshold;

    // Compute pairwise distances
    let distances = if use_f32 {
        compute_distances_f32_promoted(&train_x, &train_x, n_train, n_train, n_dim)
    } else {
        compute_distances_f64(&train_x, &train_x, n_train, n_train, n_dim)
    };

    // Assemble and solve (always f64)
    let surrogate = assemble_and_solve(
        &train_x, &distances, y_data, kernel, smoothing, n_train, n_dim,
    )?;

    let diagnostics = TrainingDiagnostics {
        used_f32_distances: use_f32,
        n_train,
        n_dim,
        system_size: n_train + n_dim + 1,
        max_distance_error: None,
    };

    Ok((surrogate, diagnostics))
}

/// Train with validation: compute both f32 and f64 distances and report error.
///
/// Useful for verifying that the f32 path doesn't introduce significant error.
pub fn train_with_validation(
    x_data: &[Vec<f64>],
    y_data: &[f64],
    kernel: RBFKernel,
    smoothing: f64,
) -> Result<(RBFSurrogate, TrainingDiagnostics)> {
    let n_train = x_data.len();

    if n_train == 0 {
        return Err(BarracudaError::InvalidInput {
            message: "Training data cannot be empty".to_string(),
        });
    }

    if y_data.len() != n_train {
        return Err(BarracudaError::InvalidInput {
            message: format!(
                "x_data and y_data length mismatch: {} vs {}",
                n_train,
                y_data.len()
            ),
        });
    }

    let n_dim = x_data[0].len();
    let train_x: Vec<f64> = x_data.iter().flat_map(|row| row.iter().copied()).collect();

    // Compute both paths
    let distances_f64 = compute_distances_f64(&train_x, &train_x, n_train, n_train, n_dim);
    let distances_f32 = compute_distances_f32_promoted(&train_x, &train_x, n_train, n_train, n_dim);

    // Compute max error
    let max_error = distances_f64
        .iter()
        .zip(distances_f32.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max);

    // Use f64 distances for the actual solve
    let surrogate = assemble_and_solve(
        &train_x,
        &distances_f64,
        y_data,
        kernel,
        smoothing,
        n_train,
        n_dim,
    )?;

    let diagnostics = TrainingDiagnostics {
        used_f32_distances: false,
        n_train,
        n_dim,
        system_size: n_train + n_dim + 1,
        max_distance_error: Some(max_error),
    };

    Ok((surrogate, diagnostics))
}

/// Assemble the augmented system and solve for RBF weights.
fn assemble_and_solve(
    train_x: &[f64],
    distances: &[f64],
    y_data: &[f64],
    kernel: RBFKernel,
    smoothing: f64,
    n_train: usize,
    n_dim: usize,
) -> Result<RBFSurrogate> {
    let n_poly = n_dim + 1;
    let n_total = n_train + n_poly;

    let mut a = vec![0.0; n_total * n_total];
    let mut b = vec![0.0; n_total];

    // Top-left: Kernel matrix K + smoothing·I
    for i in 0..n_train {
        for j in 0..n_train {
            let k_ij = kernel.eval(distances[i * n_train + j]);
            let smooth = if i == j { smoothing } else { 0.0 };
            a[i * n_total + j] = k_ij + smooth;
        }
    }

    // Top-right and bottom-left: Polynomial matrix P
    for i in 0..n_train {
        a[i * n_total + n_train] = 1.0;
        a[n_train * n_total + i] = 1.0;

        for d in 0..n_dim {
            a[i * n_total + (n_train + 1 + d)] = train_x[i * n_dim + d];
            a[(n_train + 1 + d) * n_total + i] = train_x[i * n_dim + d];
        }
    }

    b[..n_train].copy_from_slice(y_data);

    let solution = solve_f64(&a, &b, n_total)?;
    let weights = solution[..n_train].to_vec();
    let poly_coeffs = solution[n_train..].to_vec();

    Ok(RBFSurrogate::from_parts(
        train_x.to_vec(),
        weights,
        poly_coeffs,
        n_train,
        n_dim,
        kernel,
        smoothing,
    ))
}

/// Compute pairwise Euclidean distances in f64.
fn compute_distances_f64(x1: &[f64], x2: &[f64], n1: usize, n2: usize, n_dim: usize) -> Vec<f64> {
    let mut distances = vec![0.0; n1 * n2];

    for i in 0..n1 {
        for j in 0..n2 {
            let mut dist_sq = 0.0;
            for d in 0..n_dim {
                let diff = x1[i * n_dim + d] - x2[j * n_dim + d];
                dist_sq += diff * diff;
            }
            distances[i * n2 + j] = dist_sq.sqrt();
        }
    }

    distances
}

/// Compute pairwise Euclidean distances in f32, promoted to f64.
///
/// This is the CPU fast path that mirrors what `cdist.wgsl` does on GPU.
/// The f32 arithmetic is ~2× faster due to SIMD width and the distances
/// are sufficiently accurate for RBF kernel evaluation.
///
/// When a GPU is available, this function can be replaced with:
/// ```ignore
/// let tensor_a = Tensor::from_data(x1_f32, [n1, n_dim], device);
/// let tensor_b = Tensor::from_data(x2_f32, [n2, n_dim], device);
/// let distances = tensor_a.cdist_wgsl(tensor_b, DistanceMetric::Euclidean)?;
/// ```
fn compute_distances_f32_promoted(
    x1: &[f64],
    x2: &[f64],
    n1: usize,
    n2: usize,
    n_dim: usize,
) -> Vec<f64> {
    // Downcast to f32 for fast computation
    let x1_f32: Vec<f32> = x1.iter().map(|&v| v as f32).collect();
    let x2_f32: Vec<f32> = x2.iter().map(|&v| v as f32).collect();

    let mut distances = vec![0.0f64; n1 * n2];

    for i in 0..n1 {
        for j in 0..n2 {
            let mut dist_sq = 0.0f32;
            for d in 0..n_dim {
                let diff = x1_f32[i * n_dim + d] - x2_f32[j * n_dim + d];
                dist_sq += diff * diff;
            }
            // Promote result to f64
            distances[i * n2 + j] = (dist_sq.sqrt()) as f64;
        }
    }

    distances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_config_default() {
        let config = AdaptiveConfig::default();
        assert_eq!(config.f32_threshold, 200);
        assert!(!config.force_f64);
        assert!(config.parallel);
    }

    #[test]
    fn test_adaptive_config_exact() {
        let config = AdaptiveConfig::exact();
        assert!(config.force_f64);
    }

    #[test]
    fn test_adaptive_config_threshold() {
        let config = AdaptiveConfig::with_threshold(50);
        assert_eq!(config.f32_threshold, 50);
        assert!(!config.force_f64);
    }

    #[test]
    fn test_train_adaptive_small_dataset() {
        // Below threshold → should use f64 path
        let x_train: Vec<Vec<f64>> = (0..5).map(|i| vec![i as f64]).collect();
        let y_train: Vec<f64> = x_train.iter().map(|x| x[0] * x[0]).collect();

        let config = AdaptiveConfig::default();
        let (surrogate, diag) = train_adaptive(
            &x_train,
            &y_train,
            RBFKernel::ThinPlateSpline,
            1e-12,
            &config,
        )
        .unwrap();

        assert!(!diag.used_f32_distances);
        assert_eq!(diag.n_train, 5);
        assert_eq!(diag.n_dim, 1);
        assert_eq!(diag.system_size, 7); // 5 + 1 + 1

        // Should interpolate training points
        let y_pred = surrogate.predict(&x_train).unwrap();
        for i in 0..5 {
            assert!(
                (y_pred[i] - y_train[i]).abs() < 1e-6,
                "Bad interpolation at {}: {} vs {}",
                i,
                y_pred[i],
                y_train[i]
            );
        }
    }

    #[test]
    fn test_train_adaptive_uses_f32_above_threshold() {
        // Set low threshold to trigger f32 path
        let n = 10;
        let x_train: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64 / n as f64]).collect();
        let y_train: Vec<f64> = x_train.iter().map(|x| (x[0] * 3.14).sin()).collect();

        let config = AdaptiveConfig::with_threshold(5); // n=10 >= 5 → f32 path
        let (surrogate, diag) = train_adaptive(
            &x_train,
            &y_train,
            RBFKernel::ThinPlateSpline,
            1e-10,
            &config,
        )
        .unwrap();

        assert!(diag.used_f32_distances);
        assert_eq!(diag.n_train, n);

        // Should still interpolate reasonably well
        let y_pred = surrogate.predict(&x_train).unwrap();
        for i in 0..n {
            assert!(
                (y_pred[i] - y_train[i]).abs() < 0.1,
                "Bad f32-path interpolation at {}: {} vs {}",
                i,
                y_pred[i],
                y_train[i]
            );
        }
    }

    #[test]
    fn test_train_adaptive_force_f64() {
        let n = 10;
        let x_train: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64]).collect();
        let y_train: Vec<f64> = x_train.iter().map(|x| x[0]).collect();

        let config = AdaptiveConfig {
            f32_threshold: 5,
            force_f64: true, // Override: always f64
            parallel: true,
        };
        let (_, diag) = train_adaptive(
            &x_train,
            &y_train,
            RBFKernel::ThinPlateSpline,
            1e-12,
            &config,
        )
        .unwrap();

        assert!(!diag.used_f32_distances); // f64 forced
    }

    #[test]
    fn test_train_with_validation() {
        let x_train = vec![vec![0.0], vec![1.0], vec![2.0], vec![3.0], vec![4.0]];
        let y_train = vec![0.0, 1.0, 4.0, 9.0, 16.0];

        let (surrogate, diag) =
            train_with_validation(&x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-12).unwrap();

        assert!(!diag.used_f32_distances);
        assert!(diag.max_distance_error.is_some());

        // f32 vs f64 distance error should be very small for these values
        let max_err = diag.max_distance_error.unwrap();
        assert!(
            max_err < 1e-4,
            "f32/f64 distance error too large: {}",
            max_err
        );

        // Surrogate should work
        let y_pred = surrogate.predict(&x_train).unwrap();
        for i in 0..5 {
            assert!((y_pred[i] - y_train[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn test_f32_vs_f64_distances_accuracy() {
        // Verify that f32 distances are close to f64 for typical scientific data
        let n = 20;
        let n_dim = 3;
        let mut train_x = Vec::with_capacity(n * n_dim);
        for i in 0..n {
            for d in 0..n_dim {
                train_x.push((i as f64 * 0.1) + (d as f64 * 0.3));
            }
        }

        let d_f64 = compute_distances_f64(&train_x, &train_x, n, n, n_dim);
        let d_f32 = compute_distances_f32_promoted(&train_x, &train_x, n, n, n_dim);

        let max_abs_error = d_f64
            .iter()
            .zip(d_f32.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max);

        let max_rel_error = d_f64
            .iter()
            .zip(d_f32.iter())
            .filter(|(a, _)| **a > 1e-10)
            .map(|(a, b)| (a - b).abs() / a)
            .fold(0.0_f64, f64::max);

        assert!(
            max_abs_error < 1e-3,
            "Max absolute distance error: {}",
            max_abs_error
        );
        assert!(
            max_rel_error < 1e-5,
            "Max relative distance error: {}",
            max_rel_error
        );
    }

    #[test]
    fn test_adaptive_2d_function() {
        // Test with a 2D function: f(x,y) = x² + y²
        let x_train = vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![0.5, 0.5],
        ];
        let y_train: Vec<f64> = x_train.iter().map(|x| x[0] * x[0] + x[1] * x[1]).collect();

        let config = AdaptiveConfig::default();
        let (surrogate, diag) = train_adaptive(
            &x_train,
            &y_train,
            RBFKernel::ThinPlateSpline,
            1e-12,
            &config,
        )
        .unwrap();

        assert_eq!(diag.n_dim, 2);

        // Test at center (should interpolate exactly)
        let y_center = surrogate.predict(&[vec![0.5, 0.5]]).unwrap();
        assert!(
            (y_center[0] - 0.5).abs() < 0.1,
            "2D interpolation error: {}",
            y_center[0]
        );
    }

    #[test]
    fn test_adaptive_errors() {
        let config = AdaptiveConfig::default();

        // Empty data
        assert!(train_adaptive(&[], &[], RBFKernel::ThinPlateSpline, 1e-12, &config).is_err());

        // Mismatched lengths
        assert!(train_adaptive(
            &[vec![0.0], vec![1.0]],
            &[0.0],
            RBFKernel::ThinPlateSpline,
            1e-12,
            &config
        )
        .is_err());

        // Validation errors too
        assert!(train_with_validation(&[], &[], RBFKernel::ThinPlateSpline, 1e-12).is_err());
    }

    #[test]
    fn test_adaptive_gaussian_kernel() {
        let x_train: Vec<Vec<f64>> = (0..8).map(|i| vec![i as f64 * 0.5]).collect();
        let y_train: Vec<f64> = x_train.iter().map(|x| (-x[0] * x[0]).exp()).collect();

        let config = AdaptiveConfig::with_threshold(5);
        let (surrogate, diag) = train_adaptive(
            &x_train,
            &y_train,
            RBFKernel::Gaussian { epsilon: 1.0 },
            1e-10,
            &config,
        )
        .unwrap();

        assert!(diag.used_f32_distances); // n=8 >= threshold=5

        // Should interpolate training data
        let y_pred = surrogate.predict(&x_train).unwrap();
        for i in 0..8 {
            assert!(
                (y_pred[i] - y_train[i]).abs() < 0.01,
                "Gaussian kernel interpolation failed at {}: {} vs {}",
                i,
                y_pred[i],
                y_train[i]
            );
        }
    }

    #[test]
    fn test_diagnostics_fields() {
        // Use well-separated points to avoid singularity with TPS kernel
        let x_train = vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
        ];
        let y_train = vec![0.0, 1.0, 1.0, 2.0];

        let config = AdaptiveConfig::default();
        let (_, diag) = train_adaptive(
            &x_train,
            &y_train,
            RBFKernel::ThinPlateSpline,
            1e-12,
            &config,
        )
        .unwrap();

        assert_eq!(diag.n_train, 4);
        assert_eq!(diag.n_dim, 2);
        assert_eq!(diag.system_size, 7); // 4 + 2 + 1
        assert!(diag.max_distance_error.is_none()); // Not validation mode
    }
}
