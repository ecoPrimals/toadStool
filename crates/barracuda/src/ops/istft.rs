//! ISTFT - Inverse Short-Time Fourier Transform
//!
//! Reconstructs time-domain signal from STFT.
//! Uses overlap-add method.

pub async fn istft(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    stft_data: &[(f32, f32)], // Complex STFT coefficients
    n_fft: usize,
    hop_length: usize,
    window: &[f32],
    num_frames: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let output_length = (num_frames - 1) * hop_length + n_fft;
    let mut output = vec![0.0f32; output_length];
    let mut window_sum = vec![0.0f32; output_length];
    
    let bins_per_frame = n_fft / 2 + 1;
    
    for frame_idx in 0..num_frames {
        let start = frame_idx * hop_length;
        let mut frame = vec![0.0f32; n_fft];
        
        // Inverse DFT for this frame
        for n in 0..n_fft {
            for k in 0..bins_per_frame {
                let idx = frame_idx * bins_per_frame + k;
                if idx < stft_data.len() {
                    let (real, imag) = stft_data[idx];
                    let angle = 2.0 * std::f32::consts::PI * (k * n) as f32 / n_fft as f32;
                    frame[n] += real * angle.cos() - imag * angle.sin();
                }
            }
            frame[n] /= n_fft as f32;
        }
        
        // Overlap-add with window
        for n in 0..n_fft {
            if start + n < output_length {
                output[start + n] += frame[n] * window[n];
                window_sum[start + n] += window[n] * window[n];
            }
        }
    }
    
    // Normalize by window overlap
    for i in 0..output_length {
        if window_sum[i] > 1e-8 {
            output[i] /= window_sum[i];
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
    async fn test_istft() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let stft_data = vec![(1.0, 0.0); 257 * 5]; // 5 frames, 257 bins
        let window = vec![1.0; 512];
        let output = istft(&dev.device, &dev.queue, &stft_data, 512, 256, &window, 5).await.unwrap();
        assert!(output.len() > 0);
    }
}
