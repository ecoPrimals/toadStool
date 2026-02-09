// Mixed precision training utilities

use anyhow::Result;
use tch::{Kind, Tensor};

/// Mixed precision trainer wrapper
pub struct MixedPrecisionTrainer {
    /// Whether mixed precision is enabled
    enabled: bool,
    
    /// Gradient scaling factor
    scale: f64,
    
    /// Target precision kind
    precision_kind: Kind,
}

impl MixedPrecisionTrainer {
    /// Create new mixed precision trainer
    pub fn new(enabled: bool, use_bf16: bool) -> Self {
        let precision_kind = if use_bf16 {
            Kind::BFloat16
        } else {
            Kind::Half
        };
        
        Self {
            enabled,
            scale: if enabled { 128.0 } else { 1.0 },
            precision_kind,
        }
    }
    
    /// Convert tensor to mixed precision
    pub fn to_precision(&self, tensor: &Tensor) -> Tensor {
        if self.enabled {
            tensor.to_kind(self.precision_kind)
        } else {
            tensor.shallow_clone()
        }
    }
    
    /// Convert tensor back to FP32
    pub fn to_fp32(&self, tensor: &Tensor) -> Tensor {
        if self.enabled {
            tensor.to_kind(Kind::Float)
        } else {
            tensor.shallow_clone()
        }
    }
    
    /// Scale loss for mixed precision training
    pub fn scale_loss(&self, loss: &Tensor) -> Tensor {
        if self.enabled {
            loss * self.scale
        } else {
            loss.shallow_clone()
        }
    }
    
    /// Unscale gradients before optimizer step
    pub fn unscale_gradients(&self, gradients: &[Tensor]) -> Vec<Tensor> {
        if self.enabled {
            gradients.iter()
                .map(|g| g / self.scale)
                .collect()
        } else {
            gradients.iter()
                .map(|g| g.shallow_clone())
                .collect()
        }
    }
    
    /// Check if gradients contain NaN/Inf (common in FP16)
    pub fn check_gradients(&self, gradients: &[Tensor]) -> bool {
        for grad in gradients {
            if grad.isnan().any().int64_value(&[]) != 0 ||
               grad.isinf().any().int64_value(&[]) != 0 {
                return false;
            }
        }
        true
    }
}

/// Automatic Mixed Precision (AMP) context
pub struct AMPContext {
    trainer: MixedPrecisionTrainer,
}

impl AMPContext {
    /// Create new AMP context
    pub fn new(enabled: bool) -> Self {
        Self {
            trainer: MixedPrecisionTrainer::new(enabled, true), // Use BF16
        }
    }
    
    /// Run forward pass with automatic precision
    pub fn forward<F>(&self, f: F) -> Tensor
    where
        F: FnOnce() -> Tensor,
    {
        let output = f();
        self.trainer.to_precision(&output)
    }
    
    /// Run backward pass with gradient scaling
    pub fn backward(&self, loss: &Tensor) -> Result<()> {
        let scaled_loss = self.trainer.scale_loss(loss);
        scaled_loss.backward();
        Ok(())
    }
}

/// Estimate speedup from mixed precision
pub fn estimate_speedup(use_bf16: bool, model_size_mb: f64) -> f64 {
    if use_bf16 {
        // BF16 typically gives 1.5-2x speedup on modern GPUs
        // Larger models benefit more
        if model_size_mb > 100.0 {
            1.8
        } else if model_size_mb > 50.0 {
            1.6
        } else {
            1.4
        }
    } else {
        1.0
    }
}

/// Calculate memory savings from mixed precision
pub fn memory_savings(use_mixed_precision: bool) -> f64 {
    if use_mixed_precision {
        // FP16/BF16 uses half the memory
        0.5
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mixed_precision_trainer() {
        let trainer = MixedPrecisionTrainer::new(true, true);
        assert!(trainer.enabled);
        assert_eq!(trainer.scale, 128.0);
    }
    
    #[test]
    fn test_speedup_estimation() {
        // Large model (ResNet-50: ~100MB)
        let speedup = estimate_speedup(true, 100.0);
        assert!(speedup >= 1.5 && speedup <= 2.0);
        
        // Small model
        let speedup = estimate_speedup(true, 20.0);
        assert!(speedup >= 1.3 && speedup <= 1.5);
        
        // No mixed precision
        let speedup = estimate_speedup(false, 100.0);
        assert_eq!(speedup, 1.0);
    }
    
    #[test]
    fn test_memory_savings() {
        assert_eq!(memory_savings(true), 0.5); // 50% savings
        assert_eq!(memory_savings(false), 1.0); // No savings
    }
}

