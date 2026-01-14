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

/// Conv1D configuration
#[derive(Debug, Clone, Copy)]
pub struct Conv1DConfig {
    pub kernel_size: usize,
    pub stride: usize,
    pub padding: usize,
    pub dilation: usize,
}

impl Default for Conv1DConfig {
    fn default() -> Self {
        Self {
            kernel_size: 3,
            stride: 1,
            padding: 0,
            dilation: 1,
        }
    }
}

/// Conv2D configuration (standard 2D convolution)
#[derive(Debug, Clone, Copy)]
pub struct Conv2DConfig {
    pub kernel_size: (usize, usize), // (height, width)
    pub stride: (usize, usize),      // (height, width)
    pub padding: (usize, usize),     // (height, width)
    pub dilation: (usize, usize),    // (height, width) - for dilated/atrous convolutions
}

impl Default for Conv2DConfig {
    fn default() -> Self {
        Self {
            kernel_size: (3, 3),
            stride: (1, 1),
            padding: (0, 0),
            dilation: (1, 1),
        }
    }
}

/// TransposedConv2D configuration (deconvolution/upsampling)
#[derive(Debug, Clone, Copy)]
pub struct TransposedConv2DConfig {
    pub kernel_size: (usize, usize),       // (height, width)
    pub stride: (usize, usize),            // (height, width)
    pub padding: (usize, usize),           // (height, width)
    pub output_padding: (usize, usize),    // (height, width) - controls output size
}

impl Default for TransposedConv2DConfig {
    fn default() -> Self {
        Self {
            kernel_size: (2, 2),
            stride: (2, 2),
            padding: (0, 0),
            output_padding: (0, 0),
        }
    }
}

/// Conv3D configuration (3D convolution for video/medical imaging)
#[derive(Debug, Clone, Copy)]
pub struct Conv3DConfig {
    pub kernel_size: (usize, usize, usize),  // (depth, height, width)
    pub stride: (usize, usize, usize),       // (depth, height, width)
    pub padding: (usize, usize, usize),      // (depth, height, width)
    pub dilation: (usize, usize, usize),     // (depth, height, width)
}

impl Default for Conv3DConfig {
    fn default() -> Self {
        Self {
            kernel_size: (3, 3, 3),
            stride: (1, 1, 1),
            padding: (0, 0, 0),
            dilation: (1, 1, 1),
        }
    }
}

/// DepthwiseConv2D configuration
#[derive(Debug, Clone, Copy)]
pub struct DepthwiseConv2DConfig {
    pub kernel_size: (usize, usize), // (height, width)
    pub stride: (usize, usize),      // (height, width)
    pub padding: (usize, usize),     // (height, width)
}

impl Default for DepthwiseConv2DConfig {
    fn default() -> Self {
        Self {
            kernel_size: (3, 3),
            stride: (1, 1),
            padding: (1, 1),
        }
    }
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

/// InstanceNorm configuration
#[derive(Debug, Clone)]
pub struct InstanceNormConfig {
    pub epsilon: f32,
    pub gamma: Vec<f32>, // Scale (per channel)
    pub beta: Vec<f32>,  // Shift (per channel)
}

/// RMSNorm configuration
#[derive(Debug, Clone)]
pub struct RmsNormConfig {
    pub epsilon: f32,
    pub gamma: Vec<f32>, // Scale parameters
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

/// AdaGrad optimizer configuration
#[derive(Debug, Clone, Copy)]
pub struct AdagradConfig {
    pub learning_rate: f32,
    pub epsilon: f32,      // Numerical stability (default: 1e-8)
    pub weight_decay: f32, // L2 regularization (default: 0.0)
}

impl Default for AdagradConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            epsilon: 1e-8,
            weight_decay: 0.0,
        }
    }
}

/// NAdam optimizer configuration
#[derive(Debug, Clone, Copy)]
pub struct NadamConfig {
    pub learning_rate: f32,
    pub beta1: f32,        // First moment decay (default: 0.9)
    pub beta2: f32,        // Second moment decay (default: 0.999)
    pub epsilon: f32,      // Numerical stability (default: 1e-8)
    pub weight_decay: f32, // L2 regularization (default: 0.0)
}

impl Default for NadamConfig {
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

/// AdaDelta optimizer configuration
#[derive(Debug, Clone, Copy)]
pub struct AdadeltaConfig {
    pub rho: f32,          // Decay rate (default: 0.95)
    pub epsilon: f32,      // Numerical stability (default: 1e-6)
    pub weight_decay: f32, // L2 regularization (default: 0.0)
}

impl Default for AdadeltaConfig {
    fn default() -> Self {
        Self {
            rho: 0.95,
            epsilon: 1e-6,
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

/// Focal loss configuration (for class imbalance in object detection)
#[derive(Debug, Clone, Copy)]
pub struct FocalLossConfig {
    pub alpha: f32,    // Balancing factor, typically 0.25
    pub gamma: f32,    // Focusing parameter, typically 2.0
    pub epsilon: f32,  // Numerical stability
    pub reduction: LossReduction,
}

impl Default for FocalLossConfig {
    fn default() -> Self {
        Self {
            alpha: 0.25,
            gamma: 2.0,
            epsilon: 1e-7,
            reduction: LossReduction::Mean,
        }
    }
}

/// Dice loss configuration (for segmentation tasks)
#[derive(Debug, Clone, Copy)]
pub struct DiceLossConfig {
    pub smooth: f32,  // Smoothing factor, typically 1.0
    pub reduction: LossReduction,
}

impl Default for DiceLossConfig {
    fn default() -> Self {
        Self {
            smooth: 1.0,
            reduction: LossReduction::Mean,
        }
    }
}
