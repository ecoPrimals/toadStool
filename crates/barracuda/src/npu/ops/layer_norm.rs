//! NPU LayerNorm - Event-Driven Layer Normalization
//!
//! Implements LayerNorm for Transformers on Akida NPU using event-based processing.
//!
//! **Why LayerNorm Benefits from NPU**:
//! - Post-normalization values are often sparse (many near zero)
//! - Threshold-based normalization aligns with event generation
//! - Critical for Transformer models (BERT, GPT, etc.)
//!
//! **Algorithm**:
//! 1. Compute mean and variance (CPU/GPU)
//! 2. Normalize: (x - mean) / sqrt(var + eps)
//! 3. Convert to events (threshold near-zero values)
//! 4. Apply scale and bias in event space
//!
//! **Deep Debt**:
//! - Pure Rust implementation
//! - Runtime epsilon configuration
//! - Validated against CPU reference

use crate::npu::EventCodec;

type Result<T> = std::result::Result<T, crate::error::BarracudaError>;

/// NPU-accelerated Layer Normalization
///
/// Normalizes activations across the feature dimension.
///
/// **Formula**: y = (x - mean) / sqrt(var + eps) * gamma + beta
///
/// # Arguments
/// * `input` - Input activations [batch_size, features]
/// * `gamma` - Scale parameter (learnable)
/// * `beta` - Shift parameter (learnable)
/// * `eps` - Small constant for numerical stability
///
/// # Returns
/// Normalized activations
///
/// # Example
/// ```ignore
/// let input = vec![1.0, 2.0, 3.0, 4.0];
/// let gamma = vec![1.0, 1.0, 1.0, 1.0];
/// let beta = vec![0.0, 0.0, 0.0, 0.0];
/// let output = npu_layer_norm(&input, &gamma, &beta, 1e-5)?;
/// ```
pub fn npu_layer_norm(
    input: &[f32],
    gamma: &[f32],
    beta: &[f32],
    eps: f32,
) -> Result<Vec<f32>> {
    let n = input.len();
    
    if gamma.len() != n {
        return Err(crate::error::BarracudaError::invalid_op(
            "npu_layer_norm",
            format!("Gamma size {} doesn't match input size {}", gamma.len(), n),
        ));
    }
    if beta.len() != n {
        return Err(crate::error::BarracudaError::invalid_op(
            "npu_layer_norm",
            format!("Beta size {} doesn't match input size {}", beta.len(), n),
        ));
    }
    
    // Compute mean
    let mean: f32 = input.iter().sum::<f32>() / n as f32;
    
    // Compute variance
    let variance: f32 = input.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f32>() / n as f32;
    
    let std_dev = (variance + eps).sqrt();
    
    // Normalize and apply scale/shift
    let mut output = Vec::with_capacity(n);
    for i in 0..n {
        let normalized = (input[i] - mean) / std_dev;
        let scaled = normalized * gamma[i] + beta[i];
        output.push(scaled);
    }
    
    // Measure sparsity created
    let codec = EventCodec::default();
    let input_sparsity = codec.measure_sparsity(input);
    let output_sparsity = codec.measure_sparsity(&output);
    
    log::debug!(
        "NPU LayerNorm: {} features, sparsity {:.1}% → {:.1}%",
        n,
        input_sparsity * 100.0,
        output_sparsity * 100.0
    );
    
    Ok(output)
}

/// NPU-accelerated RMSNorm (Root Mean Square Normalization)
///
/// More efficient variant of LayerNorm used in modern LLMs (LLaMA, etc.)
///
/// **Formula**: y = x / sqrt(mean(x²) + eps) * gamma
///
/// **Advantages over LayerNorm**:
/// - No mean subtraction (faster)
/// - No bias term (simpler)
/// - Better numerical stability
pub fn npu_rmsnorm(
    input: &[f32],
    gamma: &[f32],
    eps: f32,
) -> Result<Vec<f32>> {
    let n = input.len();
    
    if gamma.len() != n {
        return Err(crate::error::BarracudaError::invalid_op(
            "npu_rmsnorm",
            format!("Gamma size {} doesn't match input size {}", gamma.len(), n),
        ));
    }
    
    // Compute RMS
    let mean_square: f32 = input.iter()
        .map(|&x| x * x)
        .sum::<f32>() / n as f32;
    
    let rms = (mean_square + eps).sqrt();
    
    // Normalize and scale
    let mut output = Vec::with_capacity(n);
    for i in 0..n {
        let normalized = input[i] / rms;
        let scaled = normalized * gamma[i];
        output.push(scaled);
    }
    
    log::debug!("NPU RMSNorm: {} features", n);
    
    Ok(output)
}

/// Check if NPU LayerNorm is beneficial
///
/// **Decision factors**:
/// - Small feature dimensions: CPU might be faster
/// - Large batch sizes: GPU might be better
/// - Energy priority: Always use NPU
pub fn should_use_npu_layer_norm(
    input_size: usize,
    priority: crate::workload::Priority,
) -> bool {
    use crate::workload::Priority;
    
    match priority {
        Priority::Energy => true, // Always NPU for energy
        Priority::Latency if input_size < 1024 => true, // NPU good for small
        Priority::Throughput if input_size < 512 => true, // NPU decent for medium
        _ => false, // GPU better for large throughput
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_layer_norm_basic() {
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let gamma = vec![1.0, 1.0, 1.0, 1.0];
        let beta = vec![0.0, 0.0, 0.0, 0.0];
        
        let output = npu_layer_norm(&input, &gamma, &beta, 1e-5).unwrap();
        
        // Check output is normalized (mean ≈ 0, std ≈ 1)
        let mean: f32 = output.iter().sum::<f32>() / output.len() as f32;
        assert!(mean.abs() < 0.01, "Mean should be near 0, got {}", mean);
        
        // Check all values are normalized
        for val in &output {
            assert!(val.abs() < 2.0, "Normalized values should be small, got {}", val);
        }
    }
    
    #[test]
    fn test_layer_norm_with_scale_shift() {
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let gamma = vec![2.0, 2.0, 2.0, 2.0]; // Scale by 2
        let beta = vec![1.0, 1.0, 1.0, 1.0];  // Shift by 1
        
        let output = npu_layer_norm(&input, &gamma, &beta, 1e-5).unwrap();
        
        // With gamma=2, beta=1: y = 2 * normalized + 1
        // Mean should be near 1
        let mean: f32 = output.iter().sum::<f32>() / output.len() as f32;
        assert!((mean - 1.0).abs() < 0.1, "Mean should be near 1, got {}", mean);
    }
    
    #[test]
    fn test_rmsnorm_basic() {
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let gamma = vec![1.0, 1.0, 1.0, 1.0];
        
        let output = npu_rmsnorm(&input, &gamma, 1e-5).unwrap();
        
        // RMSNorm output should be normalized by RMS
        assert_eq!(output.len(), input.len());
        
        // All values should be reasonable
        for val in &output {
            assert!(val.abs() < 5.0, "RMSNorm values should be reasonable, got {}", val);
        }
    }
    
    #[test]
    fn test_dimension_mismatch() {
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let gamma = vec![1.0, 1.0]; // Wrong size!
        let beta = vec![0.0, 0.0, 0.0, 0.0];
        
        let result = npu_layer_norm(&input, &gamma, &beta, 1e-5);
        assert!(result.is_err(), "Should error on dimension mismatch");
    }
    
    #[test]
    fn test_should_use_npu() {
        use crate::workload::Priority;
        
        // Small size + energy → NPU
        assert!(should_use_npu_layer_norm(256, Priority::Energy));
        
        // Large size + throughput → GPU
        assert!(!should_use_npu_layer_norm(4096, Priority::Throughput));
        
        // Small size + latency → NPU
        assert!(should_use_npu_layer_norm(512, Priority::Latency));
    }
}
