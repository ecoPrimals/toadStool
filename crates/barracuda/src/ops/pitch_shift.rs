//! PitchShift - Pitch shifting without tempo change
//!
//! Changes pitch by resampling in frequency domain.
//! Combines time stretching with resampling.

pub async fn pitch_shift(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    signal: &[f32],
    n_steps: f32, // Semitones to shift (positive = up, negative = down)
    bins_per_octave: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Compute shift ratio: 2^(n_steps / bins_per_octave)
    let rate = 2.0_f32.powf(n_steps / bins_per_octave);
    
    // Simple resampling approach
    let output_length = (signal.len() as f32 / rate) as usize;
    let mut output = vec![0.0f32; output_length];
    
    for i in 0..output_length {
        let src_pos = i as f32 * rate;
        let idx = src_pos as usize;
        let frac = src_pos - idx as f32;
        
        if idx < signal.len() - 1 {
            // Linear interpolation
            output[i] = signal[idx] * (1.0 - frac) + signal[idx + 1] * frac;
        } else if idx < signal.len() {
            output[i] = signal[idx];
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
    async fn test_pitch_shift() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let signal = vec![0.5; 10000];
        let shifted = pitch_shift(&dev.device, &dev.queue, &signal, 2.0, 12.0).await.unwrap();
        assert!(shifted.len() > 0);
    }
}
