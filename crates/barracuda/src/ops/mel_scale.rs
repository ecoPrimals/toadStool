//! MelScale - Mel filterbank for audio feature extraction
//!
//! Converts linear frequency scale to mel scale.
//! Used in speech recognition (MFCC, mel spectrograms).

fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0_f32.powf(mel / 2595.0) - 1.0)
}

pub async fn mel_scale(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    spectrogram: &[f32], // Power spectrogram [n_frames, n_freqs]
    n_frames: usize,
    n_freqs: usize,
    n_mels: usize,
    sample_rate: f32,
    f_min: f32,
    f_max: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Create mel filterbank
    let mel_min = hz_to_mel(f_min);
    let mel_max = hz_to_mel(f_max);
    let mel_points: Vec<f32> = (0..=n_mels + 1)
        .map(|i| mel_to_hz(mel_min + (mel_max - mel_min) * i as f32 / (n_mels + 1) as f32))
        .collect();
    
    let freq_bin = sample_rate / (2.0 * (n_freqs - 1) as f32);
    
    // Build filterbank
    let mut filterbank = vec![vec![0.0f32; n_freqs]; n_mels];
    for m in 0..n_mels {
        for f in 0..n_freqs {
            let freq = f as f32 * freq_bin;
            
            if freq >= mel_points[m] && freq <= mel_points[m + 1] {
                filterbank[m][f] = (freq - mel_points[m]) / (mel_points[m + 1] - mel_points[m]);
            } else if freq >= mel_points[m + 1] && freq <= mel_points[m + 2] {
                filterbank[m][f] = (mel_points[m + 2] - freq) / (mel_points[m + 2] - mel_points[m + 1]);
            }
        }
    }
    
    // Apply filterbank to spectrogram
    let mut mel_spec = vec![0.0f32; n_frames * n_mels];
    for frame in 0..n_frames {
        for m in 0..n_mels {
            for f in 0..n_freqs {
                mel_spec[frame * n_mels + m] += 
                    spectrogram[frame * n_freqs + f] * filterbank[m][f];
            }
        }
    }
    
    Ok(mel_spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_mel_scale() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let spectrogram = vec![1.0; 100 * 257]; // 100 frames, 257 freq bins
        let mel_spec = mel_scale(&dev.device, &dev.queue, &spectrogram, 100, 257, 80, 16000.0, 0.0, 8000.0).await.unwrap();
        assert_eq!(mel_spec.len(), 100 * 80);
    }
}
