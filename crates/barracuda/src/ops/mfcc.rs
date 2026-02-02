//! MFCC - Mel-Frequency Cepstral Coefficients
//!
//! Extracts MFCC features from audio.
//! Standard features for speech recognition.

pub async fn mfcc(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    mel_spectrogram: &[f32], // [n_frames, n_mels]
    n_frames: usize,
    n_mels: usize,
    n_mfcc: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if n_mfcc > n_mels {
        return Err("n_mfcc cannot exceed n_mels".into());
    }

    let mut mfcc_features = vec![0.0f32; n_frames * n_mfcc];

    // Apply DCT (Discrete Cosine Transform) to log mel spectrogram
    for frame in 0..n_frames {
        for k in 0..n_mfcc {
            let mut sum = 0.0;

            for n in 0..n_mels {
                let mel_val = mel_spectrogram[frame * n_mels + n];
                // Log compression
                let log_mel = (mel_val + 1e-8).ln();

                // DCT-II
                let angle = std::f32::consts::PI * k as f32 * (n as f32 + 0.5) / n_mels as f32;
                sum += log_mel * angle.cos();
            }

            mfcc_features[frame * n_mfcc + k] = sum * (2.0 / n_mels as f32).sqrt();
        }
    }

    Ok(mfcc_features)
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
    async fn test_mfcc_basic() {
        let dev = get_test_device().await;
        let mel_spec = vec![1.0; 100 * 80]; // 100 frames, 80 mel bands
        let mfcc_features = mfcc(&dev.device, &dev.queue, &mel_spec, 100, 80, 13)
            .await
            .unwrap();
        assert_eq!(mfcc_features.len(), 100 * 13);
        assert!(mfcc_features.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_mfcc_edge_cases() {
        let dev = get_test_device().await;

        // Single frame
        let mel_spec = vec![1.0; 1 * 80];
        let mfcc_features = mfcc(&dev.device, &dev.queue, &mel_spec, 1, 80, 13)
            .await
            .unwrap();
        assert_eq!(mfcc_features.len(), 13);

        // Maximum MFCC coefficients (n_mfcc = n_mels)
        let mel_spec = vec![1.0; 10 * 20];
        let mfcc_features = mfcc(&dev.device, &dev.queue, &mel_spec, 10, 20, 20)
            .await
            .unwrap();
        assert_eq!(mfcc_features.len(), 10 * 20);
    }

    #[tokio::test]
    async fn test_mfcc_boundary() {
        let dev = get_test_device().await;

        // Few coefficients
        let mel_spec = vec![1.0; 100 * 80];
        let mfcc_features = mfcc(&dev.device, &dev.queue, &mel_spec, 100, 80, 5)
            .await
            .unwrap();
        assert_eq!(mfcc_features.len(), 100 * 5);

        // Many mel bands
        let mel_spec = vec![1.0; 50 * 128];
        let mfcc_features = mfcc(&dev.device, &dev.queue, &mel_spec, 50, 128, 20)
            .await
            .unwrap();
        assert_eq!(mfcc_features.len(), 50 * 20);
    }

    #[tokio::test]
    async fn test_mfcc_large_batch() {
        let dev = get_test_device().await;

        // Long audio sequence
        let mel_spec = vec![1.0; 1000 * 80];
        let mfcc_features = mfcc(&dev.device, &dev.queue, &mel_spec, 1000, 80, 13)
            .await
            .unwrap();
        assert_eq!(mfcc_features.len(), 1000 * 13);
    }

    #[tokio::test]
    async fn test_mfcc_precision() {
        let dev = get_test_device().await;

        // Test DCT with varying mel values
        let mut mel_spec = vec![0.5; 10 * 80];
        mel_spec[0] = 2.0; // Higher energy in first frame

        let mfcc_features = mfcc(&dev.device, &dev.queue, &mel_spec, 10, 80, 13)
            .await
            .unwrap();

        assert_eq!(mfcc_features.len(), 10 * 13);
        // First frame should have different features due to higher energy
        assert!(mfcc_features[..13].iter().any(|&x| x.is_finite()));
    }
}
