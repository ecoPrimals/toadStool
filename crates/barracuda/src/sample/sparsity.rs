//! Sparsity-based iterative surrogate sampling
//!
//! Implements the SparsitySampler algorithm from Diaw et al. (2024):
//! an iterative workflow that alternates between optimization (evaluation gathering)
//! and surrogate model training to achieve both exploitation and exploration.
//!
//! # Algorithm
//!
//! ```text
//! 1. Generate initial samples via maximin LHS
//! 2. Evaluate objective at initial samples
//! 3. LOOP until budget exhausted:
//!    a. Train RBF surrogate on ALL evaluations
//!    b. Use surrogate to identify promising regions (minimize predicted value)
//!    c. Run multi-start NM on the SURROGATE to find candidate points
//!    d. Evaluate TRUE objective at candidate points
//!    e. Add evaluations to cache
//! ```
//!
//! This produces space-filling evaluations that are simultaneously:
//! - **Exploitative**: concentrated near optima (from NM convergence)
//! - **Exploratory**: spread across space (from LHS starts + NM initial phases)
//!
//! The key insight from Diaw et al.: training surrogates on ALL evaluations
//! (not just the best) provides dramatically better approximation quality.
//!
//! # Hybrid Evaluation Strategy
//!
//! For large datasets (n > 100), the distance matrix computation becomes expensive
//! (O(n²)). The hybrid mode uses GPU acceleration via `cdist.wgsl` for:
//! - Distance matrix computation (via [`train_adaptive_gpu`])
//! - Batch surrogate prediction
//!
//! Enable with `SparsitySamplerConfig::with_gpu(device)`.
//!
//! # Cross-Domain Applications
//!
//! - **Nuclear physics**: EOS parameter fitting with expensive nuclear simulations
//! - **ML**: Bayesian-style hyperparameter optimization without GP overhead
//! - **Materials science**: Force-field calibration with DFT evaluations
//! - **Engineering**: Design optimization with expensive CFD/FEA simulations
//!
//! # References
//!
//! - Diaw, A. et al. (2024). "Efficient learning of accurate surrogates for
//!   simulations of complex systems." Nature Machine Intelligence.
//! - hotSpring: `control/surrogate/scripts/full_iterative_workflow.py`

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use crate::optimize::eval_record::EvaluationCache;
use crate::optimize::multi_start::SolverResult;
use crate::sample::latin_hypercube;
use crate::surrogate::adaptive::train_adaptive_gpu;
use crate::surrogate::{loo_cv_optimal_smoothing, RBFKernel, RBFSurrogate};
use std::sync::Arc;

/// Penalty filter strategy for surrogate training.
///
/// When training the RBF surrogate, large penalty values from infeasible
/// regions can corrupt the approximation. These filters remove or cap
/// penalty values before training.
///
/// # Reference
///
/// hotSpring validation: `surrogate.rs::filter_training_data()`
#[derive(Debug, Clone, Copy, Default)]
pub enum PenaltyFilter {
    /// No filtering (default)
    #[default]
    None,
    /// Remove all y values exceeding threshold
    Threshold(f64),
    /// Remove top q% outliers (0.0 to 1.0)
    Quantile(f64),
    /// Median + k×MAD (robust outlier detection)
    AdaptiveMAD(f64),
}

/// Configuration for the SparsitySampler.
#[derive(Clone)]
pub struct SparsitySamplerConfig {
    /// Number of initial samples via LHS (default: 10 × n_dims)
    pub n_initial: usize,
    /// Number of NM solvers per iteration (default: 8)
    pub n_solvers: usize,
    /// Max evaluations per NM solver per iteration (default: 50)
    pub max_eval_per_solver: usize,
    /// Number of surrogate refinement iterations (default: 5)
    pub n_iterations: usize,
    /// NM convergence tolerance (default: 1e-6)
    pub tol: f64,
    /// RBF kernel for surrogate (default: ThinPlateSpline)
    pub kernel: RBFKernel,
    /// RBF smoothing parameter (default: 1e-12, but see auto_smoothing)
    pub smoothing: f64,
    /// Enable LOO-CV auto-tuning of smoothing (default: false)
    ///
    /// When enabled, the sampler will run LOO-CV grid search after each
    /// iteration to find the optimal smoothing parameter. This prevents
    /// both overfitting (smoothing too low) and underfitting (smoothing too high).
    ///
    /// # Reference
    ///
    /// hotSpring validation: `surrogate.rs::loo_cv_optimal_smoothing()`
    pub auto_smoothing: bool,
    /// Penalty filter for surrogate training (default: None)
    ///
    /// Filters out penalty values before training the surrogate, preventing
    /// corruption from large infeasible-region penalties.
    pub penalty_filter: PenaltyFilter,
    /// Warm-start seeds (default: empty)
    ///
    /// Pre-computed starting points for optimization (e.g., from L1 layer
    /// for L2 optimization). When non-empty, these seeds are used as
    /// additional starting points alongside LHS samples.
    ///
    /// # Reference
    ///
    /// hotSpring validation: `nuclear_eos_l2_ref.rs` L1-seeded L2 pattern
    pub warm_start_seeds: Vec<Vec<f64>>,
    /// Random seed
    pub seed: u64,
    /// GPU device for hybrid evaluation (None = CPU only)
    pub gpu_device: Option<Arc<WgpuDevice>>,
    /// Minimum dataset size to trigger GPU acceleration (default: 100)
    pub gpu_threshold: usize,
}

impl std::fmt::Debug for SparsitySamplerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SparsitySamplerConfig")
            .field("n_initial", &self.n_initial)
            .field("n_solvers", &self.n_solvers)
            .field("max_eval_per_solver", &self.max_eval_per_solver)
            .field("n_iterations", &self.n_iterations)
            .field("tol", &self.tol)
            .field("kernel", &self.kernel)
            .field("smoothing", &self.smoothing)
            .field("auto_smoothing", &self.auto_smoothing)
            .field("penalty_filter", &self.penalty_filter)
            .field("warm_start_seeds", &self.warm_start_seeds.len())
            .field("seed", &self.seed)
            .field(
                "gpu_device",
                &self.gpu_device.as_ref().map(|_| "Some(WgpuDevice)"),
            )
            .field("gpu_threshold", &self.gpu_threshold)
            .finish()
    }
}

impl SparsitySamplerConfig {
    /// Create a default configuration scaled to the problem dimension.
    ///
    /// Default smoothing is 1e-12 (near-exact interpolation). For rugged
    /// landscapes, enable `auto_smoothing` or set smoothing manually.
    pub fn new(n_dims: usize, seed: u64) -> Self {
        Self {
            n_initial: 10 * n_dims,
            n_solvers: 8,
            max_eval_per_solver: 50,
            n_iterations: 5,
            tol: 1e-6,
            kernel: RBFKernel::ThinPlateSpline,
            smoothing: 1e-12,
            auto_smoothing: false,
            penalty_filter: PenaltyFilter::None,
            warm_start_seeds: Vec::new(),
            seed,
            gpu_device: None,
            gpu_threshold: 100,
        }
    }

    /// Set number of initial LHS samples.
    pub fn with_initial_samples(mut self, n: usize) -> Self {
        self.n_initial = n;
        self
    }

    /// Set number of NM solvers per iteration.
    pub fn with_solvers(mut self, n: usize) -> Self {
        self.n_solvers = n;
        self
    }

    /// Set max evaluations per solver.
    pub fn with_eval_budget(mut self, n: usize) -> Self {
        self.max_eval_per_solver = n;
        self
    }

    /// Set number of refinement iterations.
    pub fn with_iterations(mut self, n: usize) -> Self {
        self.n_iterations = n;
        self
    }

    /// Set RBF kernel type.
    pub fn with_kernel(mut self, kernel: RBFKernel) -> Self {
        self.kernel = kernel;
        self
    }

    /// Enable GPU-accelerated surrogate training.
    ///
    /// When enabled and dataset size exceeds `gpu_threshold`, the distance
    /// matrix computation uses `cdist.wgsl` on the GPU for O(n²) speedup.
    ///
    /// # Arguments
    ///
    /// * `device` - Shared WGPU device handle
    ///
    /// # Example
    ///
    /// ```no_run
    /// use barracuda::device::WgpuDevice;
    /// use barracuda::sample::sparsity::SparsitySamplerConfig;
    /// use std::sync::Arc;
    ///
    /// # async fn example() {
    /// let device = Arc::new(WgpuDevice::new().await.unwrap());
    /// let config = SparsitySamplerConfig::new(5, 42)
    ///     .with_gpu(device)
    ///     .with_gpu_threshold(50);
    /// # }
    /// ```
    pub fn with_gpu(mut self, device: Arc<WgpuDevice>) -> Self {
        self.gpu_device = Some(device);
        self
    }

    /// Set minimum dataset size to trigger GPU acceleration (default: 100).
    ///
    /// Below this threshold, CPU training is faster due to GPU dispatch overhead.
    pub fn with_gpu_threshold(mut self, n: usize) -> Self {
        self.gpu_threshold = n;
        self
    }

    /// Set RBF smoothing parameter explicitly.
    ///
    /// Lower values → more exact interpolation (risk of overfitting).
    /// Higher values → smoother fit (risk of underfitting).
    ///
    /// For rugged landscapes, consider `with_auto_smoothing()` instead.
    pub fn with_smoothing(mut self, smoothing: f64) -> Self {
        self.smoothing = smoothing;
        self
    }

    /// Enable automatic smoothing via LOO-CV grid search.
    ///
    /// After each iteration, runs LOO-CV to find optimal smoothing
    /// that minimizes cross-validation error. This prevents:
    /// - Overfitting when smoothing is too low (default 1e-12)
    /// - Underfitting when smoothing is too high
    ///
    /// # Reference
    ///
    /// hotSpring validation: `surrogate.rs::loo_cv_optimal_smoothing()`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use barracuda::sample::sparsity::SparsitySamplerConfig;
    ///
    /// let config = SparsitySamplerConfig::new(10, 42)
    ///     .with_auto_smoothing(true)
    ///     .with_penalty_filter(barracuda::sample::sparsity::PenaltyFilter::Threshold(12.0));
    /// ```
    pub fn with_auto_smoothing(mut self, enabled: bool) -> Self {
        self.auto_smoothing = enabled;
        self
    }

    /// Set penalty filtering strategy for surrogate training.
    ///
    /// Large penalty values from infeasible regions can corrupt the
    /// surrogate approximation. Filtering removes or caps these values
    /// before training.
    ///
    /// # Options
    ///
    /// - `PenaltyFilter::None` — no filtering (default)
    /// - `PenaltyFilter::Threshold(v)` — remove all y > v
    /// - `PenaltyFilter::Quantile(q)` — remove top q% outliers
    /// - `PenaltyFilter::AdaptiveMAD(k)` — remove y > median + k×MAD
    ///
    /// # Reference
    ///
    /// hotSpring validation: `surrogate.rs::filter_training_data()`
    pub fn with_penalty_filter(mut self, filter: PenaltyFilter) -> Self {
        self.penalty_filter = filter;
        self
    }

    /// Set warm-start seeds from a previous optimization layer.
    ///
    /// When optimizing in layers (e.g., L1 → L2), the best solutions from
    /// the cheaper layer make excellent starting points for the expensive
    /// layer. This ensures NM starts in physically-valid regions rather
    /// than random space.
    ///
    /// Seeds are used alongside (not replacing) the LHS initial samples.
    ///
    /// # Reference
    ///
    /// hotSpring validation: `nuclear_eos_l2_ref.rs` L1-seeded L2 pattern
    ///
    /// # Example
    ///
    /// ```no_run
    /// use barracuda::sample::sparsity::SparsitySamplerConfig;
    ///
    /// // Best solutions from L1 optimization
    /// let l1_best = vec![
    ///     vec![0.1, 0.2, 0.3],
    ///     vec![0.15, 0.25, 0.35],
    /// ];
    ///
    /// let config = SparsitySamplerConfig::new(3, 42)
    ///     .with_warm_start(l1_best);
    /// ```
    pub fn with_warm_start(mut self, seeds: Vec<Vec<f64>>) -> Self {
        self.warm_start_seeds = seeds;
        self
    }

    /// Total evaluation budget (approximate).
    pub fn total_budget(&self) -> usize {
        self.n_initial + self.n_iterations * self.n_solvers * self.max_eval_per_solver
    }

    /// Check if GPU acceleration is configured and applicable.
    pub fn should_use_gpu(&self, dataset_size: usize) -> bool {
        self.gpu_device.is_some() && dataset_size >= self.gpu_threshold
    }
}

/// Result of SparsitySampler optimization.
#[derive(Debug)]
pub struct SparsitySamplerResult {
    /// Best point found
    pub x_best: Vec<f64>,
    /// Best function value
    pub f_best: f64,
    /// All evaluations (for surrogate training)
    pub cache: EvaluationCache,
    /// Final trained surrogate (if training succeeded)
    pub surrogate: Option<RBFSurrogate>,
    /// Results per iteration
    pub iteration_results: Vec<IterationResult>,
}

/// Apply penalty filtering to training data.
///
/// Removes or caps penalty values that would corrupt surrogate training.
///
/// # Arguments
///
/// * `x_data` - Training inputs
/// * `y_data` - Training outputs (may contain penalty values)
/// * `filter` - Filtering strategy
///
/// # Returns
///
/// Filtered (x_data, y_data) with penalties removed/capped.
fn filter_training_data(
    x_data: &[Vec<f64>],
    y_data: &[f64],
    filter: PenaltyFilter,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    match filter {
        PenaltyFilter::None => (x_data.to_vec(), y_data.to_vec()),

        PenaltyFilter::Threshold(threshold) => {
            let (x_filt, y_filt): (Vec<_>, Vec<_>) = x_data
                .iter()
                .zip(y_data.iter())
                .filter(|(_, &y)| y <= threshold)
                .map(|(x, &y)| (x.clone(), y))
                .unzip();
            (x_filt, y_filt)
        }

        PenaltyFilter::Quantile(q) => {
            if y_data.is_empty() || !(0.0..=1.0).contains(&q) {
                return (x_data.to_vec(), y_data.to_vec());
            }
            let mut sorted: Vec<f64> = y_data.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let cutoff_idx = ((1.0 - q) * (sorted.len() as f64)).floor() as usize;
            let cutoff_idx = cutoff_idx.min(sorted.len().saturating_sub(1));
            let threshold = sorted[cutoff_idx];

            let (x_filt, y_filt): (Vec<_>, Vec<_>) = x_data
                .iter()
                .zip(y_data.iter())
                .filter(|(_, &y)| y <= threshold)
                .map(|(x, &y)| (x.clone(), y))
                .unzip();
            (x_filt, y_filt)
        }

        PenaltyFilter::AdaptiveMAD(k) => {
            if y_data.is_empty() {
                return (x_data.to_vec(), y_data.to_vec());
            }
            // Compute median
            let mut sorted: Vec<f64> = y_data.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = if sorted.len() % 2 == 0 {
                (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
            } else {
                sorted[sorted.len() / 2]
            };

            // Compute MAD (median absolute deviation)
            let mut deviations: Vec<f64> = y_data.iter().map(|&y| (y - median).abs()).collect();
            deviations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mad = if deviations.len() % 2 == 0 {
                (deviations[deviations.len() / 2 - 1] + deviations[deviations.len() / 2]) / 2.0
            } else {
                deviations[deviations.len() / 2]
            };

            let threshold = median + k * mad;

            let (x_filt, y_filt): (Vec<_>, Vec<_>) = x_data
                .iter()
                .zip(y_data.iter())
                .filter(|(_, &y)| y <= threshold)
                .map(|(x, &y)| (x.clone(), y))
                .unzip();
            (x_filt, y_filt)
        }
    }
}

/// Diagnostics for a single SparsitySampler iteration.
#[derive(Debug, Clone)]
pub struct IterationResult {
    /// Iteration number (0-indexed)
    pub iteration: usize,
    /// Best f found by NM solvers in this iteration
    pub best_f: f64,
    /// Number of new evaluations in this iteration
    pub n_new_evals: usize,
    /// Total evaluations accumulated
    pub total_evals: usize,
    /// Surrogate training error (leave-one-out or None if not computed)
    pub surrogate_error: Option<f64>,
    /// Whether GPU was used for surrogate training in this iteration
    pub used_gpu: bool,
}

/// Run the SparsitySampler algorithm.
///
/// Alternates between multi-start NM optimization and RBF surrogate training
/// to efficiently explore parameter space with a limited evaluation budget.
///
/// # Arguments
///
/// * `f` - Expensive objective function to minimize
/// * `bounds` - Box bounds `[(min, max), ...]` for each dimension
/// * `config` - Sampler configuration
///
/// # Returns
///
/// [`SparsitySamplerResult`] with the best solution, all evaluations, and the
/// final surrogate model.
///
/// # Examples
///
/// ```
/// use barracuda::sample::sparsity::{sparsity_sampler, SparsitySamplerConfig};
///
/// // Expensive function (simulated)
/// let rosenbrock = |x: &[f64]| {
///     (1.0 - x[0]).powi(2) + 100.0 * (x[1] - x[0].powi(2)).powi(2)
/// };
///
/// let bounds = vec![(-5.0, 5.0), (-5.0, 5.0)];
/// let config = SparsitySamplerConfig::new(2, 42)
///     .with_initial_samples(20)
///     .with_solvers(4)
///     .with_eval_budget(30)
///     .with_iterations(3);
///
/// let result = sparsity_sampler(rosenbrock, &bounds, &config)?;
///
/// println!("Best: f={:.4} at {:?}", result.f_best, result.x_best);
/// println!("Total evaluations: {}", result.cache.len());
/// assert!(result.f_best < 10.0); // Should find a reasonable solution
/// # Ok::<(), barracuda::error::BarracudaError>(())
/// ```
pub fn sparsity_sampler<F>(
    f: F,
    bounds: &[(f64, f64)],
    config: &SparsitySamplerConfig,
) -> Result<SparsitySamplerResult>
where
    F: Fn(&[f64]) -> f64,
{
    if bounds.is_empty() {
        return Err(BarracudaError::InvalidInput {
            message: "bounds must be non-empty".to_string(),
        });
    }

    if config.n_initial < 2 {
        return Err(BarracudaError::InvalidInput {
            message: "n_initial must be >= 2 for surrogate training".to_string(),
        });
    }

    let _n_dims = bounds.len();
    let mut cache = EvaluationCache::with_capacity(config.total_budget());
    let mut iteration_results = Vec::with_capacity(config.n_iterations);
    let mut current_smoothing = config.smoothing;

    // Phase 1: Initial sampling via LHS
    let initial_points = latin_hypercube(config.n_initial, bounds, config.seed)?;

    for point in &initial_points {
        let val = f(point);
        cache.record(point.clone(), val);
    }

    // Evaluate warm-start seeds (L1→L2 seeding pattern)
    for seed in &config.warm_start_seeds {
        if seed.len() == bounds.len() {
            let val = f(seed);
            cache.record(seed.clone(), val);
        }
    }

    // Iterative refinement loop
    let mut last_surrogate = None;

    for iter in 0..config.n_iterations {
        let iter_start_evals = cache.len();

        // Get training data from cache
        let (x_raw, y_raw) = cache.training_data();

        // Apply penalty filtering before surrogate training
        let (x_data, y_data) = filter_training_data(&x_raw, &y_raw, config.penalty_filter);

        // Skip if filtering removed too many points
        if x_data.len() < 2 {
            let nm_result = run_nm_batch(&f, bounds, config, iter, &mut cache)?;
            iteration_results.push(IterationResult {
                iteration: iter,
                best_f: nm_result.f_best,
                n_new_evals: cache.len() - iter_start_evals,
                total_evals: cache.len(),
                surrogate_error: None,
                used_gpu: false,
            });
            continue;
        }

        // Auto-smoothing via LOO-CV grid search (if enabled)
        if config.auto_smoothing {
            if let Ok(result) = loo_cv_optimal_smoothing(&x_data, &y_data, config.kernel, None) {
                current_smoothing = result.smoothing;
            }
        }

        let surrogate = match RBFSurrogate::train(&x_data, &y_data, config.kernel, current_smoothing)
        {
            Ok(s) => s,
            Err(_) => {
                // If surrogate training fails (e.g., singular matrix), fall back
                // to direct multi-start NM on the true objective
                let nm_result = run_nm_batch(&f, bounds, config, iter, &mut cache)?;
                iteration_results.push(IterationResult {
                    iteration: iter,
                    best_f: nm_result.f_best,
                    n_new_evals: cache.len() - iter_start_evals,
                    total_evals: cache.len(),
                    surrogate_error: None,
                    used_gpu: false,
                });
                continue;
            }
        };

        // Compute surrogate quality metric (LOO-CV RMSE if available, else train error)
        let surrogate_error = surrogate.loo_cv_rmse().unwrap_or_else(|_| {
            compute_surrogate_rmse(&surrogate, &x_data, &y_data)
        });

        // Use surrogate to find promising regions:
        // Run multi-start NM on the SURROGATE (cheap evaluations!)
        let surrogate_ref = &surrogate;
        let surrogate_objective = |x: &[f64]| {
            surrogate_ref
                .predict(&[x.to_vec()])
                .map(|v| v[0])
                .unwrap_or(f64::INFINITY)
        };

        let iter_seed = config.seed.wrapping_add((iter as u64 + 1) * 10007);
        let candidate_points = latin_hypercube(config.n_solvers, bounds, iter_seed)?;

        // Run NM from each candidate on the surrogate, then evaluate true objective
        // at the best points found
        let mut iter_best_f = f64::INFINITY;

        for x0 in &candidate_points {
            // Quick NM on surrogate to find promising point
            let (x_star, _, _) = crate::optimize::nelder_mead(
                surrogate_objective,
                x0,
                bounds,
                config.max_eval_per_solver,
                config.tol,
            )?;

            // Evaluate TRUE objective at surrogate-suggested point
            let f_true = f(&x_star);
            cache.record(x_star, f_true);

            if f_true < iter_best_f {
                iter_best_f = f_true;
            }
        }

        // Also sample a few direct points for exploration (prevents surrogate tunnel vision)
        let explore_seed = iter_seed.wrapping_add(99991);
        let n_explore = (config.n_solvers / 4).max(1);
        let explore_points = latin_hypercube(n_explore, bounds, explore_seed)?;
        for point in &explore_points {
            let val = f(point);
            cache.record(point.clone(), val);
            if val < iter_best_f {
                iter_best_f = val;
            }
        }

        iteration_results.push(IterationResult {
            iteration: iter,
            best_f: iter_best_f,
            n_new_evals: cache.len() - iter_start_evals,
            total_evals: cache.len(),
            surrogate_error: Some(surrogate_error),
            used_gpu: false, // CPU-only path
        });

        last_surrogate = Some(surrogate);
    }

    // Extract best overall result
    let (x_best, f_best) = match cache.best() {
        Some(record) => (record.x.clone(), record.f),
        None => {
            return Err(BarracudaError::Internal(
                "No evaluations recorded".to_string(),
            ))
        }
    };

    Ok(SparsitySamplerResult {
        x_best,
        f_best,
        cache,
        surrogate: last_surrogate,
        iteration_results,
    })
}

/// Run the SparsitySampler algorithm with GPU-accelerated surrogate training.
///
/// When the dataset exceeds `config.gpu_threshold`, uses GPU for distance matrix
/// computation via `cdist.wgsl`. Falls back to CPU when GPU unavailable or
/// dataset is small.
///
/// # Requirements
///
/// - Config must have `gpu_device` set via [`SparsitySamplerConfig::with_gpu`]
/// - Async runtime (tokio or similar)
///
/// # Examples
///
/// ```no_run
/// use barracuda::device::WgpuDevice;
/// use barracuda::sample::sparsity::{sparsity_sampler_gpu, SparsitySamplerConfig};
/// use std::sync::Arc;
///
/// # async fn example() -> barracuda::error::Result<()> {
/// let device = Arc::new(WgpuDevice::new().await?);
///
/// let f = |x: &[f64]| x[0].powi(2) + x[1].powi(2);
/// let bounds = vec![(-5.0, 5.0), (-5.0, 5.0)];
///
/// let config = SparsitySamplerConfig::new(2, 42)
///     .with_gpu(device)
///     .with_gpu_threshold(50)
///     .with_initial_samples(100)
///     .with_iterations(5);
///
/// let result = sparsity_sampler_gpu(f, &bounds, &config).await?;
/// println!("Best: f={:.4}", result.f_best);
/// # Ok(())
/// # }
/// ```
pub async fn sparsity_sampler_gpu<F>(
    f: F,
    bounds: &[(f64, f64)],
    config: &SparsitySamplerConfig,
) -> Result<SparsitySamplerResult>
where
    F: Fn(&[f64]) -> f64,
{
    if bounds.is_empty() {
        return Err(BarracudaError::InvalidInput {
            message: "bounds must be non-empty".to_string(),
        });
    }

    if config.n_initial < 2 {
        return Err(BarracudaError::InvalidInput {
            message: "n_initial must be >= 2 for surrogate training".to_string(),
        });
    }

    let _n_dims = bounds.len();
    let mut cache = EvaluationCache::with_capacity(config.total_budget());
    let mut iteration_results = Vec::with_capacity(config.n_iterations);

    // Phase 1: Initial sampling via LHS
    let initial_points = latin_hypercube(config.n_initial, bounds, config.seed)?;

    for point in &initial_points {
        let val = f(point);
        cache.record(point.clone(), val);
    }

    // Iterative refinement loop
    let mut last_surrogate = None;

    for iter in 0..config.n_iterations {
        let iter_start_evals = cache.len();

        // Train surrogate on ALL evaluations so far
        let (x_data, y_data) = cache.training_data();

        // Decide: GPU or CPU path?
        let (surrogate, used_gpu) = if config.should_use_gpu(x_data.len()) {
            // GPU path: use cdist.wgsl for distance computation
            let device = config.gpu_device.as_ref().unwrap().clone();
            match train_adaptive_gpu(&x_data, &y_data, config.kernel, config.smoothing, device)
                .await
            {
                Ok((s, _diag)) => (s, true),
                Err(_) => {
                    // GPU failed, fall back to CPU
                    match RBFSurrogate::train(&x_data, &y_data, config.kernel, config.smoothing) {
                        Ok(s) => (s, false),
                        Err(_) => {
                            // Both failed, fall back to direct NM
                            let nm_result = run_nm_batch(&f, bounds, config, iter, &mut cache)?;
                            iteration_results.push(IterationResult {
                                iteration: iter,
                                best_f: nm_result.f_best,
                                n_new_evals: cache.len() - iter_start_evals,
                                total_evals: cache.len(),
                                surrogate_error: None,
                                used_gpu: false,
                            });
                            continue;
                        }
                    }
                }
            }
        } else {
            // CPU path (dataset too small for GPU benefit)
            match RBFSurrogate::train(&x_data, &y_data, config.kernel, config.smoothing) {
                Ok(s) => (s, false),
                Err(_) => {
                    // Fall back to direct NM
                    let nm_result = run_nm_batch(&f, bounds, config, iter, &mut cache)?;
                    iteration_results.push(IterationResult {
                        iteration: iter,
                        best_f: nm_result.f_best,
                        n_new_evals: cache.len() - iter_start_evals,
                        total_evals: cache.len(),
                        surrogate_error: None,
                        used_gpu: false,
                    });
                    continue;
                }
            }
        };

        // Compute surrogate quality metric
        let surrogate_error = compute_surrogate_rmse(&surrogate, &x_data, &y_data);

        // Use surrogate to find promising regions
        let surrogate_ref = &surrogate;
        let surrogate_objective = |x: &[f64]| {
            surrogate_ref
                .predict(&[x.to_vec()])
                .map(|v| v[0])
                .unwrap_or(f64::INFINITY)
        };

        let iter_seed = config.seed.wrapping_add((iter as u64 + 1) * 10007);
        let candidate_points = latin_hypercube(config.n_solvers, bounds, iter_seed)?;

        let mut iter_best_f = f64::INFINITY;

        for x0 in &candidate_points {
            let (x_star, _, _) = crate::optimize::nelder_mead(
                surrogate_objective,
                x0,
                bounds,
                config.max_eval_per_solver,
                config.tol,
            )?;

            let f_true = f(&x_star);
            cache.record(x_star, f_true);

            if f_true < iter_best_f {
                iter_best_f = f_true;
            }
        }

        // Exploration points
        let explore_seed = iter_seed.wrapping_add(99991);
        let n_explore = (config.n_solvers / 4).max(1);
        let explore_points = latin_hypercube(n_explore, bounds, explore_seed)?;
        for point in &explore_points {
            let val = f(point);
            cache.record(point.clone(), val);
            if val < iter_best_f {
                iter_best_f = val;
            }
        }

        iteration_results.push(IterationResult {
            iteration: iter,
            best_f: iter_best_f,
            n_new_evals: cache.len() - iter_start_evals,
            total_evals: cache.len(),
            surrogate_error: Some(surrogate_error),
            used_gpu,
        });

        last_surrogate = Some(surrogate);
    }

    // Extract best overall result
    let (x_best, f_best) = match cache.best() {
        Some(record) => (record.x.clone(), record.f),
        None => {
            return Err(BarracudaError::Internal(
                "No evaluations recorded".to_string(),
            ))
        }
    };

    Ok(SparsitySamplerResult {
        x_best,
        f_best,
        cache,
        surrogate: last_surrogate,
        iteration_results,
    })
}

/// Run a batch of NM solvers on the true objective (fallback when surrogate fails).
fn run_nm_batch<F>(
    f: &F,
    bounds: &[(f64, f64)],
    config: &SparsitySamplerConfig,
    iter: usize,
    cache: &mut EvaluationCache,
) -> Result<SolverResult>
where
    F: Fn(&[f64]) -> f64,
{
    let seed = config.seed.wrapping_add((iter as u64 + 1) * 10007);
    let points = latin_hypercube(config.n_solvers, bounds, seed)?;

    let mut best_x = vec![0.0; bounds.len()];
    let mut best_f = f64::INFINITY;

    for x0 in &points {
        let (x_star, f_star, _) =
            crate::optimize::nelder_mead(f, x0, bounds, config.max_eval_per_solver, config.tol)?;
        cache.record(x_star.clone(), f_star);
        if f_star < best_f {
            best_f = f_star;
            best_x = x_star;
        }
    }

    Ok(SolverResult {
        x_best: best_x,
        f_best: best_f,
        n_evals: config.n_solvers * config.max_eval_per_solver,
        converged: false,
    })
}

/// Compute RMSE of surrogate predictions at training points.
///
/// This isn't true leave-one-out CV, but gives a quick measure of
/// surrogate quality. Low RMSE indicates good interpolation.
fn compute_surrogate_rmse(surrogate: &RBFSurrogate, x_data: &[Vec<f64>], y_data: &[f64]) -> f64 {
    match surrogate.predict(x_data) {
        Ok(y_pred) => {
            let mse = y_pred
                .iter()
                .zip(y_data.iter())
                .map(|(p, t)| (p - t).powi(2))
                .sum::<f64>()
                / y_data.len() as f64;
            mse.sqrt()
        }
        Err(_) => f64::INFINITY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparsity_sampler_quadratic() {
        let f = |x: &[f64]| (x[0] - 2.0).powi(2) + (x[1] - 3.0).powi(2);
        let bounds = vec![(-10.0, 10.0), (-10.0, 10.0)];

        let config = SparsitySamplerConfig::new(2, 42)
            .with_initial_samples(20)
            .with_solvers(4)
            .with_eval_budget(30)
            .with_iterations(3);

        let result = sparsity_sampler(f, &bounds, &config).unwrap();

        assert!((result.x_best[0] - 2.0).abs() < 2.0);
        assert!((result.x_best[1] - 3.0).abs() < 2.0);
        assert!(result.f_best < 5.0);
        assert!(result.cache.len() > 20); // More than initial samples
        assert_eq!(result.iteration_results.len(), 3);
    }

    #[test]
    fn test_sparsity_sampler_rosenbrock() {
        let rosenbrock = |x: &[f64]| (1.0 - x[0]).powi(2) + 100.0 * (x[1] - x[0].powi(2)).powi(2);
        let bounds = vec![(-5.0, 5.0), (-5.0, 5.0)];

        let config = SparsitySamplerConfig::new(2, 42)
            .with_initial_samples(30)
            .with_solvers(8)
            .with_eval_budget(50)
            .with_iterations(5);

        let result = sparsity_sampler(rosenbrock, &bounds, &config).unwrap();

        // Should find a reasonable solution (not necessarily global optimum)
        assert!(
            result.f_best < 50.0,
            "Should find reasonable Rosenbrock solution, got f={}",
            result.f_best
        );

        // Should have surrogate from last iteration
        assert!(result.surrogate.is_some());
    }

    #[test]
    fn test_sparsity_sampler_captures_all_evals() {
        let f = |x: &[f64]| x[0].powi(2) + x[1].powi(2);
        let bounds = vec![(-5.0, 5.0), (-5.0, 5.0)];

        let config = SparsitySamplerConfig::new(2, 42)
            .with_initial_samples(10)
            .with_solvers(3)
            .with_iterations(2);

        let result = sparsity_sampler(f, &bounds, &config).unwrap();

        // Cache should have at least initial + iteration evaluations
        assert!(
            result.cache.len() >= 10,
            "Should have at least initial samples, got {}",
            result.cache.len()
        );

        // Training data should match cache
        let (x_data, y_data) = result.cache.training_data();
        assert_eq!(x_data.len(), y_data.len());
    }

    #[test]
    fn test_sparsity_sampler_iteration_diagnostics() {
        let f = |x: &[f64]| x[0].powi(2);
        let bounds = vec![(-5.0, 5.0)];

        let config = SparsitySamplerConfig::new(1, 42)
            .with_initial_samples(10)
            .with_solvers(3)
            .with_eval_budget(20)
            .with_iterations(3);

        let result = sparsity_sampler(f, &bounds, &config).unwrap();

        assert_eq!(result.iteration_results.len(), 3);

        for (i, ir) in result.iteration_results.iter().enumerate() {
            assert_eq!(ir.iteration, i);
            assert!(ir.n_new_evals > 0);
            assert!(ir.total_evals > 0);
        }

        // Total evals should increase monotonically
        for i in 1..result.iteration_results.len() {
            assert!(
                result.iteration_results[i].total_evals
                    >= result.iteration_results[i - 1].total_evals
            );
        }
    }

    #[test]
    fn test_sparsity_config_builder() {
        let config = SparsitySamplerConfig::new(3, 42)
            .with_initial_samples(50)
            .with_solvers(16)
            .with_eval_budget(100)
            .with_iterations(10)
            .with_kernel(RBFKernel::Gaussian { epsilon: 1.0 });

        assert_eq!(config.n_initial, 50);
        assert_eq!(config.n_solvers, 16);
        assert_eq!(config.max_eval_per_solver, 100);
        assert_eq!(config.n_iterations, 10);
        assert_eq!(config.seed, 42);
    }

    #[test]
    fn test_sparsity_sampler_total_budget() {
        let config = SparsitySamplerConfig::new(2, 42)
            .with_initial_samples(20)
            .with_solvers(4)
            .with_eval_budget(50)
            .with_iterations(5);

        // Budget = 20 + 5 * 4 * 50 = 1020
        assert_eq!(config.total_budget(), 1020);
    }

    #[test]
    fn test_sparsity_sampler_errors() {
        let f = |x: &[f64]| x[0].powi(2);

        // Empty bounds
        let config = SparsitySamplerConfig::new(1, 42);
        assert!(sparsity_sampler(&f, &[], &config).is_err());

        // Too few initial samples
        let bounds = vec![(0.0, 1.0)];
        let config = SparsitySamplerConfig::new(1, 42).with_initial_samples(1);
        assert!(sparsity_sampler(&f, &bounds, &config).is_err());
    }

    #[test]
    fn test_sparsity_sampler_1d() {
        // Simple 1D function with clear minimum
        let f = |x: &[f64]| (x[0] - 3.0).powi(2) + 1.0;
        let bounds = vec![(-10.0, 10.0)];

        let config = SparsitySamplerConfig::new(1, 42)
            .with_initial_samples(10)
            .with_solvers(4)
            .with_eval_budget(30)
            .with_iterations(3);

        let result = sparsity_sampler(f, &bounds, &config).unwrap();

        assert!(
            (result.x_best[0] - 3.0).abs() < 2.0,
            "Should find x near 3.0, got {}",
            result.x_best[0]
        );
        assert!(result.f_best < 5.0);
    }

    #[test]
    fn test_sparsity_sampler_with_gaussian_kernel() {
        let f = |x: &[f64]| x[0].powi(2) + x[1].powi(2);
        let bounds = vec![(-5.0, 5.0), (-5.0, 5.0)];

        let config = SparsitySamplerConfig::new(2, 42)
            .with_initial_samples(15)
            .with_solvers(3)
            .with_iterations(2)
            .with_kernel(RBFKernel::Gaussian { epsilon: 0.5 });

        let result = sparsity_sampler(f, &bounds, &config).unwrap();

        assert!(result.f_best < 10.0);
        assert!(result.surrogate.is_some());
    }

    #[test]
    fn test_surrogate_rmse() {
        // Train on y = x^2 and check RMSE is very small (exact interpolation)
        let x_train = vec![vec![0.0], vec![1.0], vec![2.0], vec![3.0]];
        let y_train = vec![0.0, 1.0, 4.0, 9.0];

        let surrogate =
            RBFSurrogate::train(&x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-12).unwrap();

        let rmse = compute_surrogate_rmse(&surrogate, &x_train, &y_train);
        assert!(
            rmse < 1e-6,
            "Surrogate should interpolate training data exactly, RMSE={}",
            rmse
        );
    }
}
