//! WindowFunction - Various windowing functions for signal processing
//!
//! Implements Hann, Hamming, Blackman, and other windows.
//! Reduces spectral leakage in FFT.

pub enum WindowType {
    Hann,
    Hamming,
    Blackman,
    Bartlett,
    Rectangular,
}

pub async fn window_function(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    length: usize,
    window_type: WindowType,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut window = vec![0.0f32; length];
    
    for n in 0..length {
        let val = match window_type {
            WindowType::Hann => {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * n as f32 / (length - 1) as f32).cos())
            }
            WindowType::Hamming => {
                0.54 - 0.46 * (2.0 * std::f32::consts::PI * n as f32 / (length - 1) as f32).cos()
            }
            WindowType::Blackman => {
                0.42 
                - 0.5 * (2.0 * std::f32::consts::PI * n as f32 / (length - 1) as f32).cos()
                + 0.08 * (4.0 * std::f32::consts::PI * n as f32 / (length - 1) as f32).cos()
            }
            WindowType::Bartlett => {
                1.0 - ((2 * n) as f32 / (length - 1) as f32 - 1.0).abs()
            }
            WindowType::Rectangular => 1.0,
        };
        
        window[n] = val;
    }
    
    Ok(window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_window_hann() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let window = window_function(&dev.device, &dev.queue, 512, WindowType::Hann).await.unwrap();
        assert_eq!(window.len(), 512);
        assert!((window[0] - 0.0).abs() < 1e-5); // Should be ~0 at edges
        assert!(window[256] > 0.9); // Should be ~1 at center
    }
}
