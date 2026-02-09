// Performance optimization utilities

pub mod mixed_precision;
pub mod batch_tuning;
pub mod learning_rate;

/// Optimization configuration
#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    /// Use mixed precision (FP16/BF16)
    pub use_mixed_precision: bool,
    
    /// Automatic mixed precision type
    pub precision_type: PrecisionType,
    
    /// Gradient scaling factor (for FP16)
    pub gradient_scale: f64,
    
    /// Adaptive batch size
    pub adaptive_batch_size: bool,
    
    /// Learning rate scaling strategy
    pub lr_scaling: LearningRateScaling,
}

#[derive(Debug, Clone, Copy)]
pub enum PrecisionType {
    /// Full precision (FP32)
    FP32,
    
    /// Half precision (FP16) - faster but less precise
    FP16,
    
    /// Brain float 16 (BF16) - better range than FP16
    BF16,
}

#[derive(Debug, Clone, Copy)]
pub enum LearningRateScaling {
    /// No scaling
    None,
    
    /// Linear scaling: lr_distributed = lr_single * num_towers
    Linear,
    
    /// Square root scaling: lr_distributed = lr_single * sqrt(num_towers)
    SquareRoot,
    
    /// Custom factor
    Custom(f64),
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            use_mixed_precision: false,
            precision_type: PrecisionType::FP32,
            gradient_scale: 1.0,
            adaptive_batch_size: false,
            lr_scaling: LearningRateScaling::Linear,
        }
    }
}

impl OptimizationConfig {
    /// Create configuration for maximum performance
    pub fn max_performance() -> Self {
        Self {
            use_mixed_precision: true,
            precision_type: PrecisionType::BF16,
            gradient_scale: 1.0,
            adaptive_batch_size: true,
            lr_scaling: LearningRateScaling::Linear,
        }
    }
    
    /// Create configuration for maximum accuracy
    pub fn max_accuracy() -> Self {
        Self {
            use_mixed_precision: false,
            precision_type: PrecisionType::FP32,
            gradient_scale: 1.0,
            adaptive_batch_size: false,
            lr_scaling: LearningRateScaling::SquareRoot,
        }
    }
    
    /// Create balanced configuration
    pub fn balanced() -> Self {
        Self {
            use_mixed_precision: true,
            precision_type: PrecisionType::BF16,
            gradient_scale: 1.0,
            adaptive_batch_size: true,
            lr_scaling: LearningRateScaling::Linear,
        }
    }
}

/// Calculate optimal batch size for given GPU
pub fn optimal_batch_size(vram_gb: usize, model_params_millions: f64) -> usize {
    // Rule of thumb: batch_size ≈ GPU_memory_GB * 16 / sqrt(params_M)
    let base_batch_size = (vram_gb * 16) as f64 / model_params_millions.sqrt();
    
    // Round to nearest power of 2
    let log2 = base_batch_size.log2();
    let rounded = 2_usize.pow(log2.round() as u32);
    
    // Clamp to reasonable range
    rounded.max(32).min(512)
}

/// Scale learning rate for distributed training
pub fn scale_learning_rate(
    base_lr: f64,
    num_towers: usize,
    strategy: LearningRateScaling,
) -> f64 {
    match strategy {
        LearningRateScaling::None => base_lr,
        LearningRateScaling::Linear => base_lr * num_towers as f64,
        LearningRateScaling::SquareRoot => base_lr * (num_towers as f64).sqrt(),
        LearningRateScaling::Custom(factor) => base_lr * factor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_optimal_batch_size() {
        // RTX 2070: 8GB, ResNet-18: 11.7M params
        let batch_size = optimal_batch_size(8, 11.7);
        assert!(batch_size >= 32 && batch_size <= 512);
        assert_eq!(batch_size & (batch_size - 1), 0); // Power of 2
    }
    
    #[test]
    fn test_lr_scaling() {
        let base_lr = 0.1;
        
        // Linear: 2x towers = 2x LR
        let scaled = scale_learning_rate(base_lr, 2, LearningRateScaling::Linear);
        assert!((scaled - 0.2).abs() < 1e-9);
        
        // Square root: 4x towers = 2x LR
        let scaled = scale_learning_rate(base_lr, 4, LearningRateScaling::SquareRoot);
        assert!((scaled - 0.2).abs() < 1e-9);
        
        // Custom
        let scaled = scale_learning_rate(base_lr, 2, LearningRateScaling::Custom(1.5));
        assert!((scaled - 0.15).abs() < 1e-9);
    }
}

