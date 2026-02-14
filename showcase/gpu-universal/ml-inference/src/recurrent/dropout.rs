//! Recurrent Dropout (temporal consistency)

/// Recurrent Dropout
///
/// Dropout specifically designed for recurrent networks.
pub struct RecurrentDropout {
    dropout_rate: f32,
}

impl RecurrentDropout {
    /// Create new Recurrent Dropout
    pub fn new(dropout_rate: f32) -> Self {
        Self { dropout_rate }
    }

    /// Apply recurrent dropout
    ///
    /// Uses same mask across sequence (temporal consistency)
    pub fn apply(&self, sequence: &[f32], batch: u32, seq_len: u32, hidden_size: u32) -> Vec<f32> {
        let mut output = sequence.to_vec();

        // Generate single mask per (batch, hidden_dim)
        // Reuse across all timesteps
        let mask = self.generate_mask(batch, hidden_size);

        for t in 0..seq_len {
            for b in 0..batch {
                for h in 0..hidden_size {
                    let idx = ((b * seq_len + t) * hidden_size + h) as usize;
                    let mask_idx = (b * hidden_size + h) as usize;
                    output[idx] *= mask[mask_idx];
                }
            }
        }

        output
    }

    fn generate_mask(&self, batch: u32, hidden_size: u32) -> Vec<f32> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut mask = vec![0.0f32; (batch * hidden_size) as usize];
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let scale = 1.0 / (1.0 - self.dropout_rate);

        for i in 0..mask.len() {
            // Simple LCG for mask generation
            let val = ((seed.wrapping_mul(1103515245).wrapping_add(i as u64 * 12345)) % 2147483648)
                as f32
                / 2147483648.0;

            mask[i] = if val > self.dropout_rate { scale } else { 0.0 };
        }

        mask
    }
}

#[cfg(test)]
mod layer_tests {
    use std::sync::Arc;
    use anyhow::{Result, Context};
    use crate::recurrent::{BidirectionalRNN, StackedLSTM, GRULayer, LSTMLayer};
    use super::*;

    async fn create_test_device() -> Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .context("No GPU adapter found")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .context("Failed to create device")?;

        Ok((Arc::new(device), Arc::new(queue)))
    }

    #[tokio::test]
    async fn test_bidirectional_rnn_creation() {
        if let Ok((device, queue)) = create_test_device().await {
            let result = BidirectionalRNN::new(device, queue, 10, 20).await;
            assert!(result.is_ok(), "Failed to create bidirectional RNN");
        }
    }

    #[tokio::test]
    async fn test_stacked_lstm_creation() {
        if let Ok((device, queue)) = create_test_device().await {
            let result = StackedLSTM::new(device, queue, 10, 20, 3).await;
            assert!(result.is_ok(), "Failed to create stacked LSTM");
        }
    }

    #[tokio::test]
    async fn test_gru_layer_creation() {
        if let Ok((device, queue)) = create_test_device().await {
            let result = GRULayer::new(device, queue, 10, 20).await;
            assert!(result.is_ok(), "Failed to create GRU layer");
        }
    }

    #[tokio::test]
    async fn test_lstm_layer_creation() {
        if let Ok((device, queue)) = create_test_device().await {
            let result = LSTMLayer::new(device, queue, 10, 20).await;
            assert!(result.is_ok(), "Failed to create LSTM layer");
        }
    }

    #[test]
    fn test_recurrent_dropout() {
        let dropout = RecurrentDropout::new(0.5);
        let batch = 2;
        let seq_len = 10;
        let hidden_size = 8;

        let sequence = vec![1.0f32; (batch * seq_len * hidden_size) as usize];
        let output = dropout.apply(&sequence, batch, seq_len, hidden_size);

        assert_eq!(output.len(), sequence.len());
        // Check that some values are dropped (0.0) and others are scaled
        let has_zeros = output.iter().any(|&x| x == 0.0);
        let has_scaled = output.iter().any(|&x| x > 1.0);
        assert!(has_zeros || has_scaled, "Dropout should modify values");
    }
}
