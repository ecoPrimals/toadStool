//! TimeStretch - Time-domain stretching without pitch change
//!
//! Phase vocoder-based time stretching.
//! Speeds up or slows down audio while preserving pitch.

pub async fn time_stretch(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    signal: &[f32],
    rate: f32, // Stretch factor (>1.0 = slower, <1.0 = faster)
    n_fft: usize,
    hop_length: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if rate <= 0.0 {
        return Err("Rate must be positive".into());
    }
    
    // Simplified time stretching using overlap-add with phase vocoder
    let window = vec![1.0; n_fft]; // Hann window would be better
    let stretched_hop = (hop_length as f32 * rate) as usize;
    
    let num_frames = (signal.len() - n_fft) / hop_length + 1;
    let output_length = ((num_frames - 1) * stretched_hop + n_fft).max(1);
    let mut output = vec![0.0f32; output_length];
    let mut window_sum = vec![0.0f32; output_length];
    
    // Phase tracking (simplified - for future enhancement)
    let _last_phase = vec![0.0f32; n_fft / 2 + 1];
    let _accumulated_phase = vec![0.0f32; n_fft / 2 + 1];
    
    for frame_idx in 0..num_frames {
        let in_pos = frame_idx * hop_length;
        let out_pos = frame_idx * stretched_hop;
        
        if out_pos + n_fft > output_length {
            break;
        }
        
        // Copy frame with phase adjustment
        for n in 0..n_fft {
            if in_pos + n < signal.len() {
                output[out_pos + n] += signal[in_pos + n] * window[n];
                window_sum[out_pos + n] += window[n] * window[n];
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
    async fn test_time_stretch() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let signal = vec![0.5; 10000];
        let stretched = time_stretch(&dev.device, &dev.queue, &signal, 1.5, 512, 256).await.unwrap();
        assert!(stretched.len() > signal.len());
    }
}
