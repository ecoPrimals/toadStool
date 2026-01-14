//! Type definitions for WGPU executor
//!
//! All configuration types, enums, and structs for GPU operations.
//! Deep Debt: No hardcoded values, all configurable at runtime.

/// Binary operations for elementwise operations
#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add = 0,
    Sub = 1,
    Mul = 2,
    Div = 3,
}

/// Reduction operations
#[derive(Debug, Clone, Copy)]
pub enum ReduceOp {
    Sum = 0,
    Max = 1,
    Min = 2,
    Mean = 3,
}

/// Map operations
#[derive(Debug, Clone, Copy)]
pub enum MapOp {
    Square = 0,
    Sqrt = 1,
    Abs = 2,
    Negate = 3,
    Reciprocal = 4,
}

/// Scan operations (for prefix sum/scan)
#[derive(Debug, Clone, Copy)]
pub enum ScanOp {
    Sum = 0,
    Max = 1,
    Min = 2,
}

/// Normalization configuration
#[derive(Debug, Clone)]
pub struct NormConfig {
    pub epsilon: f32,
    pub gamma: Option<Vec<f32>>, // Scale (default: all 1s)
    pub beta: Option<Vec<f32>>,  // Shift (default: all 0s)
}

/// BatchNorm configuration with pre-computed statistics
#[derive(Debug, Clone)]
pub struct BatchNormConfig {
    pub epsilon: f32,
    pub gamma: Vec<f32>,        // Scale (learned parameter)
    pub beta: Vec<f32>,         // Shift (learned parameter)
    pub running_mean: Vec<f32>, // Pre-computed mean
    pub running_var: Vec<f32>,  // Pre-computed variance
}

/// MaxPool2D configuration
#[derive(Debug, Clone, Copy)]
pub struct Pool2DConfig {
    pub kernel_size: (usize, usize), // (height, width)
    pub stride: (usize, usize),      // (height, width)
    pub padding: (usize, usize),     // (height, width)
}

impl Default for Pool2DConfig {
    fn default() -> Self {
        Self {
            kernel_size: (2, 2),
            stride: (2, 2),
            padding: (0, 0),
        }
    }
}

/// CrossEntropy loss reduction mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossReduction {
    None, // Return per-sample losses
    Mean, // Return mean loss
    Sum,  // Return sum of losses
}

/// CrossEntropy loss configuration
#[derive(Debug, Clone, Copy)]
pub struct CrossEntropyConfig {
    pub epsilon: f32, // Small constant to prevent log(0)
    pub reduction: LossReduction,
}

impl Default for CrossEntropyConfig {
    fn default() -> Self {
        Self {
            epsilon: 1e-7,
            reduction: LossReduction::Mean,
        }
    }
}

/// GroupNorm configuration
#[derive(Debug, Clone)]
pub struct GroupNormConfig {
    pub num_groups: usize,
    pub epsilon: f32,
    pub gamma: Vec<f32>, // Scale (per channel)
    pub beta: Vec<f32>,  // Shift (per channel)
}

/// Adam optimizer configuration
#[derive(Debug, Clone, Copy)]
pub struct AdamConfig {
    pub learning_rate: f32,
    pub beta1: f32,        // First moment decay (default: 0.9)
    pub beta2: f32,        // Second moment decay (default: 0.999)
    pub epsilon: f32,      // Numerical stability (default: 1e-8)
    pub weight_decay: f32, // L2 regularization (default: 0.0)
}

impl Default for AdamConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            weight_decay: 0.0,
        }
    }
}

/// SGD (Stochastic Gradient Descent) optimizer configuration
#[derive(Debug, Clone, Copy)]
pub struct SgdConfig {
    pub learning_rate: f32,
    pub momentum: f32,     // 0.0 for no momentum, typically 0.9
    pub weight_decay: f32, // L2 regularization (default: 0.0)
    pub dampening: f32,    // Dampening for momentum (default: 0.0)
}

impl Default for SgdConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            momentum: 0.0,
            weight_decay: 0.0,
            dampening: 0.0,
        }
    }
}

/// RMSprop optimizer configuration
#[derive(Debug, Clone, Copy)]
pub struct RmspropConfig {
    pub learning_rate: f32,
    pub alpha: f32,        // Decay rate (default: 0.99)
    pub epsilon: f32,      // Numerical stability (default: 1e-8)
    pub weight_decay: f32, // L2 regularization (default: 0.0)
}

impl Default for RmspropConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            alpha: 0.99,
            epsilon: 1e-8,
            weight_decay: 0.0,
        }
    }
}

/// Regression loss configuration (MSE, MAE, etc.)
#[derive(Debug, Clone, Copy)]
pub struct RegressionLossConfig {
    pub reduction: LossReduction,
}

impl Default for RegressionLossConfig {
    fn default() -> Self {
        Self {
            reduction: LossReduction::Mean,
        }
    }
}

/// Huber loss configuration (robust regression)
#[derive(Debug, Clone, Copy)]
pub struct HuberLossConfig {
    pub delta: f32,        // Threshold for switching from quadratic to linear
    pub reduction: LossReduction,
}

impl Default for HuberLossConfig {
    fn default() -> Self {
        Self {
            delta: 1.0,
            reduction: LossReduction::Mean,
        }
    }
}

/// BCE (Binary Cross Entropy) loss configuration
#[derive(Debug, Clone, Copy)]
pub struct BceLossConfig {
    pub epsilon: f32, // Small constant to prevent log(0)
    pub reduction: LossReduction,
}

impl Default for BceLossConfig {
    fn default() -> Self {
        Self {
            epsilon: 1e-7,
            reduction: LossReduction::Mean,
        }
    }
}
