//! STFT - Short-Time Fourier Transform
//!
//! Converts time-domain signal to time-frequency representation.
//! Foundation for spectrograms and audio analysis.

pub async fn stft(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    signal: &[f32],
    n_fft: usize,
    hop_length: usize,
    window: &[f32],
) -> Result<Vec<(f32, f32)>, Box<dyn std::error::Error>> {
    if window.len() != n_fft {
        return Err("Window length must match n_fft".into());
    }
    
    let num_frames = (signal.len() - n_fft) / hop_length + 1;
    let mut output = Vec::with_capacity(num_frames * (n_fft / 2 + 1));
    
    // Process each frame
    for frame_idx in 0..num_frames {
        let start = frame_idx * hop_length;
        
        // Apply window and compute DFT for positive frequencies
        for k in 0..=(n_fft / 2) {
            let mut real = 0.0;
            let mut imag = 0.0;
            
            for n in 0..n_fft {
                if start + n < signal.len() {
                    let windowed = signal[start + n] * window[n];
                    let angle = -2.0 * std::f32::consts::PI * (k * n) as f32 / n_fft as f32;
                    real += windowed * angle.cos();
                    imag += windowed * angle.sin();
                }
            }
            
            output.push((real, imag));
        }
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_stft() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let signal = vec![0.0; 1024];
        let window = vec![1.0; 512]; // Rectangular window
        let output = stft(&dev.device, &dev.queue, &signal, 512, 256, &window).await.unwrap();
        assert!(output.len() > 0);
    }
}
