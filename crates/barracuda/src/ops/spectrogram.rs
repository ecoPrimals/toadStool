//! Spectrogram - Power spectrogram computation
//!
//! Computes magnitude squared of STFT.
//! Visualizes frequency content over time.

pub async fn spectrogram(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    stft_data: &[(f32, f32)], // Complex STFT
    power: f32, // 1.0 for magnitude, 2.0 for power
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut spec = Vec::with_capacity(stft_data.len());
    
    for &(real, imag) in stft_data {
        let magnitude = (real * real + imag * imag).sqrt();
        spec.push(magnitude.powf(power));
    }
    
    Ok(spec)
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
    async fn test_spectrogram_basic() {
        let dev = get_test_device().await;
        let stft_data = vec![(3.0, 4.0); 1000]; // Magnitude = 5.0
        let power_spec = spectrogram(&dev.device, &dev.queue, &stft_data, 2.0).await.unwrap();
        assert_eq!(power_spec.len(), 1000);
        assert!((power_spec[0] - 25.0).abs() < 1e-5); // 5^2 = 25
    }

    #[tokio::test]
    async fn test_spectrogram_edge_cases() {
        let dev = get_test_device().await;

        // Single sample
        let stft_data = vec![(1.0, 0.0)];
        let mag_spec = spectrogram(&dev.device, &dev.queue, &stft_data, 1.0).await.unwrap();
        assert_eq!(mag_spec.len(), 1);
        assert!((mag_spec[0] - 1.0).abs() < 1e-5);

        // Zero magnitude
        let stft_data = vec![(0.0, 0.0); 10];
        let power_spec = spectrogram(&dev.device, &dev.queue, &stft_data, 2.0).await.unwrap();
        assert!(power_spec.iter().all(|&x| x.abs() < 1e-6));
    }

    #[tokio::test]
    async fn test_spectrogram_boundary() {
        let dev = get_test_device().await;

        // Power = 1.0 (magnitude spectrogram)
        let stft_data = vec![(3.0, 4.0)];
        let mag_spec = spectrogram(&dev.device, &dev.queue, &stft_data, 1.0).await.unwrap();
        assert!((mag_spec[0] - 5.0).abs() < 1e-5);

        // Different complex values
        let stft_data = vec![(1.0, 1.0), (0.0, 1.0), (1.0, 0.0)];
        let power_spec = spectrogram(&dev.device, &dev.queue, &stft_data, 2.0).await.unwrap();
        assert!(power_spec.iter().all(|&x| x >= 0.0));
    }

    #[tokio::test]
    async fn test_spectrogram_large_batch() {
        let dev = get_test_device().await;

        // 10000 frequency bins
        let stft_data = vec![(1.0, 1.0); 10000];
        let power_spec = spectrogram(&dev.device, &dev.queue, &stft_data, 2.0).await.unwrap();
        assert_eq!(power_spec.len(), 10000);
    }

    #[tokio::test]
    async fn test_spectrogram_precision() {
        let dev = get_test_device().await;

        // Known values: (3,4) -> mag=5, power=25
        let stft_data = vec![(3.0, 4.0)];
        let mag_spec = spectrogram(&dev.device, &dev.queue, &stft_data, 1.0).await.unwrap();
        let power_spec = spectrogram(&dev.device, &dev.queue, &stft_data, 2.0).await.unwrap();
        
        assert!((mag_spec[0] - 5.0).abs() < 1e-5);
        assert!((power_spec[0] - 25.0).abs() < 1e-5);
    }
}
