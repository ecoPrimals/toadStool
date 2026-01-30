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
    
    #[tokio::test]
    async fn test_spectrogram() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let stft_data = vec![(3.0, 4.0); 1000]; // Magnitude = 5.0
        let power_spec = spectrogram(&dev.device, &dev.queue, &stft_data, 2.0).await.unwrap();
        assert_eq!(power_spec.len(), 1000);
        assert!((power_spec[0] - 25.0).abs() < 1e-5); // 5^2 = 25
    }
}
