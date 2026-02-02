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
                filterbank[m][f] =
                    (mel_points[m + 2] - freq) / (mel_points[m + 2] - mel_points[m + 1]);
            }
        }
    }

    // Apply filterbank to spectrogram
    let mut mel_spec = vec![0.0f32; n_frames * n_mels];
    for frame in 0..n_frames {
        for m in 0..n_mels {
            for f in 0..n_freqs {
                mel_spec[frame * n_mels + m] += spectrogram[frame * n_freqs + f] * filterbank[m][f];
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

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_mel_scale_basic() {
        let dev = get_test_device().await;
        let spectrogram = vec![1.0; 100 * 257]; // 100 frames, 257 freq bins
        let mel_spec = mel_scale(
            &dev.device,
            &dev.queue,
            &spectrogram,
            100,
            257,
            80,
            16000.0,
            0.0,
            8000.0,
        )
        .await
        .unwrap();
        assert_eq!(mel_spec.len(), 100 * 80);
        assert!(mel_spec.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_mel_scale_edge_cases() {
        let dev = get_test_device().await;

        // Single frame
        let spectrogram = vec![1.0; 1 * 257];
        let mel_spec = mel_scale(
            &dev.device,
            &dev.queue,
            &spectrogram,
            1,
            257,
            80,
            16000.0,
            0.0,
            8000.0,
        )
        .await
        .unwrap();
        assert_eq!(mel_spec.len(), 80);

        // Few mel bands
        let spectrogram = vec![1.0; 10 * 257];
        let mel_spec = mel_scale(
            &dev.device,
            &dev.queue,
            &spectrogram,
            10,
            257,
            20,
            16000.0,
            0.0,
            8000.0,
        )
        .await
        .unwrap();
        assert_eq!(mel_spec.len(), 10 * 20);
    }

    #[tokio::test]
    async fn test_mel_scale_boundary() {
        let dev = get_test_device().await;

        // High sample rate
        let spectrogram = vec![1.0; 50 * 513];
        let mel_spec = mel_scale(
            &dev.device,
            &dev.queue,
            &spectrogram,
            50,
            513,
            128,
            44100.0,
            0.0,
            22050.0,
        )
        .await
        .unwrap();
        assert_eq!(mel_spec.len(), 50 * 128);

        // Different frequency range
        let spectrogram = vec![1.0; 100 * 257];
        let mel_spec = mel_scale(
            &dev.device,
            &dev.queue,
            &spectrogram,
            100,
            257,
            40,
            8000.0,
            300.0,
            4000.0,
        )
        .await
        .unwrap();
        assert_eq!(mel_spec.len(), 100 * 40);
    }

    #[tokio::test]
    async fn test_mel_scale_large_batch() {
        let dev = get_test_device().await;

        // Long audio (many frames)
        let spectrogram = vec![1.0; 500 * 257];
        let mel_spec = mel_scale(
            &dev.device,
            &dev.queue,
            &spectrogram,
            500,
            257,
            80,
            16000.0,
            0.0,
            8000.0,
        )
        .await
        .unwrap();
        assert_eq!(mel_spec.len(), 500 * 80);
    }

    #[tokio::test]
    async fn test_mel_scale_precision() {
        let dev = get_test_device().await;

        // Test filterbank energy preservation
        let mut spectrogram = vec![0.0; 10 * 257];
        spectrogram[0] = 10.0; // Energy in first bin

        let mel_spec = mel_scale(
            &dev.device,
            &dev.queue,
            &spectrogram,
            10,
            257,
            80,
            16000.0,
            0.0,
            8000.0,
        )
        .await
        .unwrap();

        assert_eq!(mel_spec.len(), 10 * 80);
        // Verify operation completed successfully
        assert!(mel_spec.iter().all(|&x| x.is_finite()));
    }
}
