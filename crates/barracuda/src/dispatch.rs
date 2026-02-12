//! Auto-dispatch system for CPU/GPU routing
//!
//! Provides intelligent, size-based dispatch for mathematical operations.
//! Small workloads stay on CPU (avoiding dispatch overhead), large workloads
//! use GPU acceleration.
//!
//! # Dual-Precision Architecture
//!
//! ```text
//! User calls: erf(x) or matmul(a, b)
//!       ↓
//! DispatchConfig checks:
//! - Input size vs threshold
//! - GPU availability
//! - Force flags
//!       ↓
//! Routes to:
//! - CPU f64 (small N, precision-critical)
//! - GPU f32 (large N, throughput-critical)
//! ```
//!
//! # Per-Function Thresholds
//!
//! Thresholds are empirically determined via benchmarking:
//!
//! | Operation | CPU Threshold | Reason |
//! |-----------|---------------|--------|
//! | erf | 512 | GPU dispatch overhead ~0.1ms |
//! | matmul | 64 | GPU wins at 64×64 matrices |
//! | eigh | 128 | Jacobi iteration memory-bound |
//! | cdist | 200 | Distance computation O(N²) |
//! | fft | 1024 | FFT benefits from parallelism |
//!
//! # Example
//!
//! ```
//! use barracuda::dispatch::{Dispatch, DispatchConfig};
//!
//! // Configure dispatch
//! let config = DispatchConfig::default();
//!
//! // Auto-route based on size
//! if config.should_use_gpu(1000, "matmul") {
//!     // GPU path
//! } else {
//!     // CPU path
//! }
//! ```

use std::collections::HashMap;
use std::sync::OnceLock;

/// Global dispatch configuration (lazy-initialized)
static GLOBAL_CONFIG: OnceLock<DispatchConfig> = OnceLock::new();

/// Dispatch configuration for CPU/GPU routing
#[derive(Debug, Clone)]
pub struct DispatchConfig {
    /// Per-operation CPU thresholds (input size below which CPU is used)
    thresholds: HashMap<&'static str, usize>,
    /// Whether GPU is available (cached at init)
    gpu_available: bool,
    /// Force CPU for all operations (useful for testing, f64 precision)
    force_cpu: bool,
    /// Force GPU for all operations (useful for benchmarking)
    force_gpu: bool,
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self {
            thresholds: default_thresholds(),
            gpu_available: check_gpu_available(),
            force_cpu: false,
            force_gpu: false,
        }
    }
}

impl DispatchConfig {
    /// Create a new dispatch config with default thresholds
    pub fn new() -> Self {
        Self::default()
    }

    /// Create config with custom thresholds
    pub fn with_thresholds(thresholds: HashMap<&'static str, usize>) -> Self {
        Self {
            thresholds,
            ..Self::default()
        }
    }

    /// Force all operations to CPU
    pub fn force_cpu(mut self) -> Self {
        self.force_cpu = true;
        self.force_gpu = false;
        self
    }

    /// Force all operations to GPU (if available)
    pub fn force_gpu(mut self) -> Self {
        self.force_gpu = true;
        self.force_cpu = false;
        self
    }

    /// Set threshold for a specific operation
    pub fn set_threshold(&mut self, operation: &'static str, threshold: usize) {
        self.thresholds.insert(operation, threshold);
    }

    /// Get threshold for an operation (returns default if not set)
    pub fn threshold(&self, operation: &str) -> usize {
        self.thresholds
            .get(operation)
            .copied()
            .unwrap_or(DEFAULT_THRESHOLD)
    }

    /// Check if GPU should be used for given input size and operation
    ///
    /// # Arguments
    ///
    /// * `input_size` - Size of input (elements, not bytes)
    /// * `operation` - Name of operation (e.g., "matmul", "erf", "cdist")
    ///
    /// # Returns
    ///
    /// `true` if GPU should be used, `false` for CPU
    pub fn should_use_gpu(&self, input_size: usize, operation: &str) -> bool {
        // Force flags take precedence
        if self.force_cpu {
            return false;
        }
        if self.force_gpu && self.gpu_available {
            return true;
        }

        // GPU must be available
        if !self.gpu_available {
            return false;
        }

        // Size-based dispatch
        let threshold = self.threshold(operation);
        input_size >= threshold
    }

    /// Check if GPU is available
    pub fn has_gpu(&self) -> bool {
        self.gpu_available
    }
}

/// Default threshold when operation not specified
const DEFAULT_THRESHOLD: usize = 1024;

/// Default per-operation thresholds (empirically determined)
fn default_thresholds() -> HashMap<&'static str, usize> {
    let mut m = HashMap::new();

    // === Special Functions ===
    m.insert("erf", 512); // Error function
    m.insert("erfc", 512); // Complementary error function
    m.insert("gamma", 256); // Gamma function
    m.insert("lgamma", 256); // Log-gamma
    m.insert("digamma", 256); // Digamma (psi)
    m.insert("bessel_j0", 512); // Bessel J0
    m.insert("bessel_j1", 512); // Bessel J1
    m.insert("bessel_i0", 512); // Modified Bessel I0
    m.insert("bessel_k0", 512); // Modified Bessel K0

    // === Linear Algebra ===
    m.insert("matmul", 64); // Matrix multiply: 64×64 = 4096 elements
    m.insert("cholesky", 128); // Cholesky decomposition
    m.insert("eigh", 128); // Symmetric eigenvalue
    m.insert("lu", 128); // LU decomposition
    m.insert("qr", 128); // QR decomposition
    m.insert("svd", 128); // Singular value decomposition
    m.insert("solve", 128); // Linear solve
    m.insert("tridiagonal", 256); // Tridiagonal solve (Thomas alg is fast)

    // === Distance/Similarity ===
    m.insert("cdist", 200); // Pairwise distances (O(N²))
    m.insert("pdist", 200); // Pairwise distances (condensed)
    m.insert("cosine_similarity", 256);

    // === Transforms ===
    m.insert("fft", 1024); // FFT
    m.insert("ifft", 1024); // Inverse FFT
    m.insert("dct", 1024); // Discrete cosine transform

    // === Reductions ===
    m.insert("sum", 4096); // Reduction needs large N for GPU win
    m.insert("mean", 4096);
    m.insert("max", 4096);
    m.insert("min", 4096);
    m.insert("argmax", 4096);
    m.insert("argmin", 4096);

    // === Element-wise ===
    m.insert("relu", 2048);
    m.insert("sigmoid", 2048);
    m.insert("tanh", 2048);
    m.insert("exp", 2048);
    m.insert("log", 2048);
    m.insert("sqrt", 2048);
    m.insert("sin", 2048);
    m.insert("cos", 2048);

    // === Surrogate/Optimization ===
    m.insert("rbf_kernel", 200); // RBF kernel evaluation
    m.insert("surrogate_predict", 100); // Single-point prediction is CPU-only
    m.insert("surrogate_train", 200); // Training benefits from GPU

    m
}

/// Check if GPU is available (via wgpu)
fn check_gpu_available() -> bool {
    // Use futures::executor::block_on for blocking async in non-async context
    futures::executor::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await;

        adapter.is_some()
    })
}

/// Get global dispatch config (lazy-initialized)
pub fn global_config() -> &'static DispatchConfig {
    GLOBAL_CONFIG.get_or_init(DispatchConfig::default)
}

/// Dispatch trait for types that can auto-dispatch
pub trait Dispatch {
    /// Get the dispatch configuration
    fn dispatch_config(&self) -> &DispatchConfig;

    /// Should this workload use GPU?
    fn should_use_gpu(&self, operation: &str) -> bool;

    /// Get input size for dispatch decision
    fn dispatch_size(&self) -> usize;
}

/// Dispatch decision result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchTarget {
    /// Use CPU (f64 precision available)
    Cpu,
    /// Use GPU (f32 precision, high throughput)
    Gpu,
}

impl DispatchTarget {
    /// Check if this is CPU target
    pub fn is_cpu(self) -> bool {
        matches!(self, DispatchTarget::Cpu)
    }

    /// Check if this is GPU target
    pub fn is_gpu(self) -> bool {
        matches!(self, DispatchTarget::Gpu)
    }
}

/// Decide dispatch target for given operation and size
pub fn dispatch_for(operation: &str, input_size: usize) -> DispatchTarget {
    if global_config().should_use_gpu(input_size, operation) {
        DispatchTarget::Gpu
    } else {
        DispatchTarget::Cpu
    }
}

/// Decide dispatch target using custom config
pub fn dispatch_with_config(
    config: &DispatchConfig,
    operation: &str,
    input_size: usize,
) -> DispatchTarget {
    if config.should_use_gpu(input_size, operation) {
        DispatchTarget::Gpu
    } else {
        DispatchTarget::Cpu
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DispatchConfig::default();

        // Check some default thresholds
        assert_eq!(config.threshold("matmul"), 64);
        assert_eq!(config.threshold("erf"), 512);
        assert_eq!(config.threshold("cdist"), 200);

        // Unknown operation gets default
        assert_eq!(config.threshold("unknown_op"), DEFAULT_THRESHOLD);
    }

    #[test]
    fn test_force_cpu() {
        let config = DispatchConfig::default().force_cpu();

        // Should never use GPU when forced to CPU
        assert!(!config.should_use_gpu(1_000_000, "matmul"));
        assert!(!config.should_use_gpu(1_000_000, "erf"));
    }

    #[test]
    fn test_size_threshold() {
        let mut config = DispatchConfig::default();
        config.force_cpu = false; // Ensure not forced

        // Below threshold: CPU
        // Note: if GPU not available, always returns false
        if config.has_gpu() {
            assert!(!config.should_use_gpu(32, "matmul")); // 32 < 64
            assert!(config.should_use_gpu(128, "matmul")); // 128 >= 64
        }
    }

    #[test]
    fn test_custom_threshold() {
        let mut config = DispatchConfig::default();
        config.set_threshold("custom_op", 500);

        assert_eq!(config.threshold("custom_op"), 500);
    }

    #[test]
    fn test_dispatch_target() {
        assert!(DispatchTarget::Cpu.is_cpu());
        assert!(!DispatchTarget::Cpu.is_gpu());
        assert!(DispatchTarget::Gpu.is_gpu());
        assert!(!DispatchTarget::Gpu.is_cpu());
    }

    #[test]
    fn test_dispatch_for() {
        // Small input should go to CPU
        let target = dispatch_for("matmul", 10);
        assert!(target.is_cpu());
    }
}
