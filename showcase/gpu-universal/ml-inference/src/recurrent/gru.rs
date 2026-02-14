//! GRU Cell and Layer

use anyhow::Result;
use std::sync::Arc;

/// GRU Cell (Gated Recurrent Unit)
///
/// Gated Recurrent Unit - simplified LSTM with fewer parameters.
///
/// ## Benefits
///
/// - Fewer parameters than LSTM
/// - Faster training
/// - Often comparable performance
pub struct GRUCell {
    #[allow(dead_code)]
    device: Arc<wgpu::Device>,
    #[allow(dead_code)]
    queue: Arc<wgpu::Queue>,
    input_size: u32,
    hidden_size: u32,
}

impl GRUCell {
    /// Create new GRU Cell
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        input_size: u32,
        hidden_size: u32,
    ) -> Result<Self> {
        Ok(Self {
            device,
            queue,
            input_size,
            hidden_size,
        })
    }

    /// Forward pass through GRU cell
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor [batch, d_input]
    /// * `hidden` - Previous hidden state [batch, d_hidden]
    /// * `w_ih` - Input-hidden weights [d_input, 3*d_hidden] (concat: r,z,n)
    /// * `w_hh` - Hidden-hidden weights [d_hidden, 3*d_hidden] (concat: r,z,n)
    /// * `b_ih` - Input-hidden bias `3*d_hidden`
    /// * `b_hh` - Hidden-hidden bias `3*d_hidden`
    /// * `batch` - Batch size
    ///
    /// # Returns
    ///
    /// New hidden state [batch, d_hidden]
    #[allow(clippy::too_many_arguments)]
    pub async fn forward(
        &self,
        input: &[f32],
        hidden: &[f32],
        w_ih: &[f32],
        w_hh: &[f32],
        b_ih: &[f32],
        b_hh: &[f32],
        batch: u32,
    ) -> Result<Vec<f32>> {
        let mut new_hidden = vec![0.0f32; (batch * self.hidden_size) as usize];

        for b in 0..batch {
            // Compute all 3 gates
            let mut gates = vec![0.0f32; (3 * self.hidden_size) as usize];

            for gate_idx in 0..(3 * self.hidden_size) {
                let mut sum = 0.0f32;

                // W_ih·x_t
                for i in 0..self.input_size {
                    let input_idx = (b * self.input_size + i) as usize;
                    let weight_idx = (i * 3 * self.hidden_size + gate_idx) as usize;
                    sum += input[input_idx] * w_ih[weight_idx];
                }

                // + b_ih
                sum += b_ih[gate_idx as usize];

                // For reset and update gates: W_hh·h_{t-1}
                // For new gate: applied differently (after reset gate)
                if gate_idx < 2 * self.hidden_size {
                    for hh in 0..self.hidden_size {
                        let hidden_idx = (b * self.hidden_size + hh) as usize;
                        let weight_idx = (hh * 3 * self.hidden_size + gate_idx) as usize;
                        sum += hidden[hidden_idx] * w_hh[weight_idx];
                    }
                    sum += b_hh[gate_idx as usize];
                }

                gates[gate_idx as usize] = sum;
            }

            // Compute new hidden state
            for h in 0..self.hidden_size {
                let h_usize = h as usize;

                // Reset gate: r_t = σ(gates[0:hidden_size])
                let r_t = Self::sigmoid(gates[h_usize]);

                // Update gate: z_t = σ(gates[hidden_size:2*hidden_size])
                let z_t = Self::sigmoid(gates[(self.hidden_size + h) as usize]);

                // New gate: n_t = tanh(W_in·x_t + r_t ⊙ (W_hn·h_{t-1} + b_hn))
                let mut n_t = gates[(2 * self.hidden_size + h) as usize];
                for hh in 0..self.hidden_size {
                    let hidden_idx = (b * self.hidden_size + hh) as usize;
                    let weight_idx =
                        (hh * 3 * self.hidden_size + 2 * self.hidden_size + h) as usize;
                    n_t += r_t * hidden[hidden_idx] * w_hh[weight_idx];
                }
                n_t += r_t * b_hh[(2 * self.hidden_size + h) as usize];
                n_t = n_t.tanh();

                // Hidden state: h_t = (1 - z_t) ⊙ n_t + z_t ⊙ h_{t-1}
                let hidden_idx = (b * self.hidden_size + h) as usize;
                new_hidden[hidden_idx] = (1.0 - z_t) * n_t + z_t * hidden[hidden_idx];
            }
        }

        Ok(new_hidden)
    }

    fn sigmoid(x: f32) -> f32 {
        1.0 / (1.0 + (-x).exp())
    }
}

#[cfg(test)]
mod tests {
    use crate::recurrent::{RNNCell, LSTMCell};
    use anyhow::Context;
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
    async fn test_rnn_cell_creation() {
        if let Ok((device, queue)) = create_test_device().await {
            let result = RNNCell::new(device, queue, 10, 20).await;
            assert!(result.is_ok(), "Failed to create RNN cell");
        }
    }

    #[tokio::test]
    async fn test_lstm_cell_creation() {
        if let Ok((device, queue)) = create_test_device().await {
            let result = LSTMCell::new(device, queue, 10, 20).await;
            assert!(result.is_ok(), "Failed to create LSTM cell");
        }
    }

    #[tokio::test]
    async fn test_gru_cell_creation() {
        if let Ok((device, queue)) = create_test_device().await {
            let result = GRUCell::new(device, queue, 10, 20).await;
            assert!(result.is_ok(), "Failed to create GRU cell");
        }
    }

    #[tokio::test]
    async fn test_rnn_cell_forward() {
        if let Ok((device, queue)) = create_test_device().await {
            if let Ok(rnn) = RNNCell::new(device, queue, 4, 8).await {
                let batch = 2;
                let input = vec![0.5f32; (batch * 4) as usize];
                let hidden = vec![0.0f32; (batch * 8) as usize];
                let w_ih = vec![0.1f32; (4 * 8) as usize];
                let w_hh = vec![0.1f32; (8 * 8) as usize];
                let b_ih = vec![0.0f32; 8];
                let b_hh = vec![0.0f32; 8];

                let result = rnn
                    .forward(&input, &hidden, &w_ih, &w_hh, &b_ih, &b_hh, batch)
                    .await;
                assert!(result.is_ok(), "RNN forward pass failed");

                if let Ok(output) = result {
                    assert_eq!(output.len(), (batch * 8) as usize);
                    assert!(output.iter().all(|x| x.is_finite()));
                }
            }
        }
    }

    #[tokio::test]
    async fn test_lstm_cell_forward() {
        if let Ok((device, queue)) = create_test_device().await {
            if let Ok(lstm) = LSTMCell::new(device, queue, 4, 8).await {
                let batch = 2;
                let input = vec![0.5f32; (batch * 4) as usize];
                let hidden = vec![0.0f32; (batch * 8) as usize];
                let cell = vec![0.0f32; (batch * 8) as usize];
                let w_ih = vec![0.1f32; (4 * 4 * 8) as usize];
                let w_hh = vec![0.1f32; (8 * 4 * 8) as usize];
                let b_ih = vec![0.0f32; (4 * 8) as usize];
                let b_hh = vec![0.0f32; (4 * 8) as usize];

                let result: Result<(Vec<f32>, Vec<f32>)> = lstm
                    .forward(&input, &hidden, &cell, &w_ih, &w_hh, &b_ih, &b_hh, batch)
                    .await;
                assert!(result.is_ok(), "LSTM forward pass failed");

                if let Ok((new_hidden, new_cell)) = result {
                    assert_eq!(new_hidden.len(), (batch * 8) as usize);
                    assert_eq!(new_cell.len(), (batch * 8) as usize);
                    assert!(new_hidden.iter().all(|x| x.is_finite()));
                    assert!(new_cell.iter().all(|x| x.is_finite()));
                }
            }
        }
    }

    #[tokio::test]
    async fn test_gru_cell_forward() {
        if let Ok((device, queue)) = create_test_device().await {
            if let Ok(gru) = GRUCell::new(device, queue, 4, 8).await {
                let batch = 2;
                let input = vec![0.5f32; (batch * 4) as usize];
                let hidden = vec![0.0f32; (batch * 8) as usize];
                let w_ih = vec![0.1f32; (4 * 3 * 8) as usize];
                let w_hh = vec![0.1f32; (8 * 3 * 8) as usize];
                let b_ih = vec![0.0f32; (3 * 8) as usize];
                let b_hh = vec![0.0f32; (3 * 8) as usize];

                let result = gru
                    .forward(&input, &hidden, &w_ih, &w_hh, &b_ih, &b_hh, batch)
                    .await;
                assert!(result.is_ok(), "GRU forward pass failed");

                if let Ok(output) = result {
                    assert_eq!(output.len(), (batch * 8) as usize);
                    assert!(output.iter().all(|x| x.is_finite()));
                }
            }
        }
    }
}

/// Bidirectional RNN
///
/// Processes sequences in both forward and backward directions,
/// then concatenates the outputs.
///
/// ## Architecture
///
/// ```text
/// h_forward = RNN_forward(x_0, x_1, ..., x_T)
/// h_backward = RNN_backward(x_T, x_{T-1}, ..., x_0)
/// output = concat(h_forward, h_backward)
/// ```
///
/// ## Benefits
///
/// - Captures both past and future context
/// - Better for non-causal tasks (e.g., named entity recognition)
pub struct GRULayer {
    cell: GRUCell,
}

impl GRULayer {
    /// Create new GRU Layer
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        input_size: u32,
        hidden_size: u32,
    ) -> Result<Self> {
        let cell = GRUCell::new(device, queue, input_size, hidden_size).await?;
        Ok(Self { cell })
    }

    /// Forward pass through GRU layer
    #[allow(clippy::too_many_arguments)]
    pub async fn forward(
        &self,
        sequence: &[f32],
        batch: u32,
        seq_len: u32,
        w_ih: &[f32],
        w_hh: &[f32],
        b_ih: &[f32],
        b_hh: &[f32],
    ) -> Result<Vec<f32>> {
        let hidden_size = self.cell.hidden_size;
        let input_size = self.cell.input_size;

        let mut outputs = Vec::new();
        let mut hidden = vec![0.0f32; (batch * hidden_size) as usize];

        for t in 0..seq_len {
            let input_start = (batch * t * input_size) as usize;
            let input_end = (batch * (t + 1) * input_size) as usize;
            let input_t = &sequence[input_start..input_end];

            hidden = self
                .cell
                .forward(input_t, &hidden, w_ih, w_hh, b_ih, b_hh, batch)
                .await?;
            outputs.extend_from_slice(&hidden);
        }

        Ok(outputs)
    }
}
