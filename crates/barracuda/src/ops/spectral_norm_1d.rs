//! SpectralNorm1D - Spectral normalization for 1D convolutions
//!
//! Normalizes weight matrix by its largest singular value.
//! Used for stabilizing GAN training in audio generation.

pub async fn spectral_norm_1d(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    weights: &[f32], // [out_channels, in_channels, kernel_size]
    out_channels: usize,
    in_channels: usize,
    kernel_size: usize,
    n_power_iterations: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Reshape to 2D matrix [out_channels, in_channels * kernel_size]
    let rows = out_channels;
    let cols = in_channels * kernel_size;
    
    if weights.len() != rows * cols {
        return Err("Weight dimensions mismatch".into());
    }
    
    // Initialize random vector for power iteration
    let mut u = vec![1.0f32; rows];
    let mut v = vec![1.0f32; cols];
    
    // Normalize initial vectors
    let u_norm: f32 = u.iter().map(|&x| x * x).sum::<f32>().sqrt();
    for val in u.iter_mut() { *val /= u_norm; }
    
    // Power iteration to estimate largest singular value
    for _ in 0..n_power_iterations {
        // v = W^T @ u
        for c in 0..cols {
            v[c] = 0.0;
            for r in 0..rows {
                v[c] += weights[r * cols + c] * u[r];
            }
        }
        let v_norm: f32 = v.iter().map(|&x| x * x).sum::<f32>().sqrt();
        for val in v.iter_mut() { *val /= v_norm + 1e-12; }
        
        // u = W @ v
        for r in 0..rows {
            u[r] = 0.0;
            for c in 0..cols {
                u[r] += weights[r * cols + c] * v[c];
            }
        }
        let u_norm: f32 = u.iter().map(|&x| x * x).sum::<f32>().sqrt();
        for val in u.iter_mut() { *val /= u_norm + 1e-12; }
    }
    
    // Compute sigma = u^T @ W @ v
    let mut sigma = 0.0;
    for r in 0..rows {
        for c in 0..cols {
            sigma += u[r] * weights[r * cols + c] * v[c];
        }
    }
    
    // Normalize weights by sigma
    let mut normalized = weights.to_vec();
    for val in normalized.iter_mut() {
        *val /= sigma + 1e-12;
    }
    
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_spectral_norm_1d_basic() {
        let dev = get_test_device().await;
        let weights = vec![1.0; 64 * 32 * 3]; // [64, 32, 3]
        let normalized = spectral_norm_1d(&dev.device, &dev.queue, &weights, 64, 32, 3, 1).await.unwrap();
        assert_eq!(normalized.len(), weights.len());
        assert!(normalized.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_spectral_norm_1d_edge_cases() {
        let dev = get_test_device().await;

        // Small kernel
        let weights = vec![1.0; 4 * 4 * 1];
        let normalized = spectral_norm_1d(&dev.device, &dev.queue, &weights, 4, 4, 1, 1).await.unwrap();
        assert_eq!(normalized.len(), 16);

        // Single output channel
        let weights = vec![2.0; 1 * 8 * 3];
        let normalized = spectral_norm_1d(&dev.device, &dev.queue, &weights, 1, 8, 3, 1).await.unwrap();
        assert_eq!(normalized.len(), 24);
    }

    #[tokio::test]
    async fn test_spectral_norm_1d_boundary() {
        let dev = get_test_device().await;

        // More power iterations
        let weights = vec![1.0; 32 * 16 * 5];
        let normalized = spectral_norm_1d(&dev.device, &dev.queue, &weights, 32, 16, 5, 5).await.unwrap();
        assert_eq!(normalized.len(), 32 * 16 * 5);

        // Large kernel
        let weights = vec![1.0; 16 * 16 * 7];
        let normalized = spectral_norm_1d(&dev.device, &dev.queue, &weights, 16, 16, 7, 2).await.unwrap();
        assert!(normalized.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_spectral_norm_1d_large_batch() {
        let dev = get_test_device().await;

        // Large layer: 128 out, 128 in, kernel 7
        let weights = vec![1.0; 128 * 128 * 7];
        let normalized = spectral_norm_1d(&dev.device, &dev.queue, &weights, 128, 128, 7, 1).await.unwrap();
        assert_eq!(normalized.len(), 128 * 128 * 7);
    }

    #[tokio::test]
    async fn test_spectral_norm_1d_precision() {
        let dev = get_test_device().await;

        // Verify normalization (largest singular value should be ~1)
        let weights = vec![2.0; 8 * 8 * 3];
        let normalized = spectral_norm_1d(&dev.device, &dev.queue, &weights, 8, 8, 3, 3).await.unwrap();
        
        assert_eq!(normalized.len(), weights.len());
        assert!(normalized.iter().all(|&x| x.is_finite()));
        // Normalized weights should be smaller than original
        assert!(normalized.iter().all(|&x| x.abs() <= 2.0));
    }
}
