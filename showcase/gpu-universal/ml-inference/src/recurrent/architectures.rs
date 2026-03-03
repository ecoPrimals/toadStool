// SPDX-License-Identifier: AGPL-3.0-or-later
//! Advanced RNN Architectures (Bidirectional, Stacked)

use anyhow::Result;
use std::sync::Arc;

use super::{LSTMCell, RNNCell};

/// Bidirectional RNN
///
/// Processes sequences in both forward and backward directions.
pub struct BidirectionalRNN {
    forward_rnn: RNNCell,
    backward_rnn: RNNCell,
}

impl BidirectionalRNN {
    /// Create new Bidirectional RNN
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        input_size: u32,
        hidden_size: u32,
    ) -> Result<Self> {
        let forward_rnn = RNNCell::new(
            Arc::clone(&device),
            Arc::clone(&queue),
            input_size,
            hidden_size,
        )
        .await?;
        let backward_rnn = RNNCell::new(device, queue, input_size, hidden_size).await?;

        Ok(Self {
            forward_rnn,
            backward_rnn,
        })
    }

    /// Forward pass through bidirectional RNN
    ///
    /// # Arguments
    ///
    /// * `sequence` - Input sequence [batch, seq_len, d_input]
    /// * `batch` - Batch size
    /// * `seq_len` - Sequence length
    /// * Other weight/bias parameters for both directions
    ///
    /// # Returns
    ///
    /// Output sequence [batch, seq_len, 2*d_hidden]
    #[allow(clippy::too_many_arguments)]
    pub async fn forward(
        &self,
        sequence: &[f32],
        batch: u32,
        seq_len: u32,
        w_ih_fwd: &[f32],
        w_hh_fwd: &[f32],
        b_ih_fwd: &[f32],
        b_hh_fwd: &[f32],
        w_ih_bwd: &[f32],
        w_hh_bwd: &[f32],
        b_ih_bwd: &[f32],
        b_hh_bwd: &[f32],
    ) -> Result<Vec<f32>> {
        let input_size = self.forward_rnn.input_size;
        let hidden_size = self.forward_rnn.hidden_size;

        // Forward pass
        let mut forward_outputs = Vec::new();
        let mut hidden_fwd = vec![0.0f32; (batch * hidden_size) as usize];

        for t in 0..seq_len {
            let input_start = (batch * t * input_size) as usize;
            let input_end = (batch * (t + 1) * input_size) as usize;
            let input_t = &sequence[input_start..input_end];

            hidden_fwd = self
                .forward_rnn
                .forward(
                    input_t,
                    &hidden_fwd,
                    w_ih_fwd,
                    w_hh_fwd,
                    b_ih_fwd,
                    b_hh_fwd,
                    batch,
                )
                .await?;

            forward_outputs.push(hidden_fwd.clone());
        }

        // Backward pass
        let mut backward_outputs = Vec::new();
        let mut hidden_bwd = vec![0.0f32; (batch * hidden_size) as usize];

        for t in (0..seq_len).rev() {
            let input_start = (batch * t * input_size) as usize;
            let input_end = (batch * (t + 1) * input_size) as usize;
            let input_t = &sequence[input_start..input_end];

            hidden_bwd = self
                .backward_rnn
                .forward(
                    input_t,
                    &hidden_bwd,
                    w_ih_bwd,
                    w_hh_bwd,
                    b_ih_bwd,
                    b_hh_bwd,
                    batch,
                )
                .await?;

            backward_outputs.push(hidden_bwd.clone());
        }

        backward_outputs.reverse();

        // Concatenate forward and backward outputs
        let mut output = vec![0.0f32; (batch * seq_len * 2 * hidden_size) as usize];

        for t in 0..seq_len {
            for b in 0..batch {
                for h in 0..hidden_size {
                    let fwd_idx = (b * hidden_size + h) as usize;
                    let out_idx = ((b * seq_len + t) * 2 * hidden_size + h) as usize;
                    output[out_idx] = forward_outputs[t as usize][fwd_idx];

                    let bwd_idx = (b * hidden_size + h) as usize;
                    let out_idx_bwd =
                        ((b * seq_len + t) * 2 * hidden_size + hidden_size + h) as usize;
                    output[out_idx_bwd] = backward_outputs[t as usize][bwd_idx];
                }
            }
        }

        Ok(output)
    }
}

/// Stacked LSTM
///
/// Multi-layer LSTM where output of layer n becomes input to layer n+1.
///
/// ## Architecture
///
/// ```text
/// h^1_t = LSTM^1(x_t, h^1_{t-1}, c^1_{t-1})
/// h^2_t = LSTM^2(h^1_t, h^2_{t-1}, c^2_{t-1})
/// ...
/// h^L_t = LSTM^L(h^{L-1}_t, h^L_{t-1}, c^L_{t-1})
/// ```
///
/// ## Benefits
///
/// - Increased model capacity
/// - Hierarchical feature learning
/// - Better for complex sequence tasks
pub struct StackedLSTM {
    #[allow(dead_code)]
    layers: Vec<LSTMCell>,
    #[allow(dead_code)]
    num_layers: u32,
}

impl StackedLSTM {
    /// Create new Stacked LSTM
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        input_size: u32,
        hidden_size: u32,
        num_layers: u32,
    ) -> Result<Self> {
        let mut layers = Vec::new();

        for layer in 0..num_layers {
            let input_sz = if layer == 0 { input_size } else { hidden_size };
            let lstm = LSTMCell::new(
                Arc::clone(&device),
                Arc::clone(&queue),
                input_sz,
                hidden_size,
            )
            .await?;
            layers.push(lstm);
        }

        Ok(Self { layers, num_layers })
    }
}
