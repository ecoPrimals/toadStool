// SPDX-License-Identifier: AGPL-3.0-or-later
//! Basic RNN Cell (Elman Network)

use anyhow::Result;
use std::sync::Arc;

/// RNN Cell (Elman Network)
///
/// Basic recurrent cell: h_t = tanh(W_ih·x_t + b_ih + W_hh·h_{t-1} + b_hh)
///
/// ## Parameters
///
/// - Input size: d_input
/// - Hidden size: d_hidden
/// - Weights: W_ih [d_input, d_hidden], W_hh [d_hidden, d_hidden]
/// - Biases: b_ih `d_hidden`, b_hh `d_hidden`
///
/// ## Performance
///
/// - Complexity: O(d_input·d_hidden + d_hidden²)
/// - Memory: O(batch·d_hidden)
pub struct RNNCell {
    #[allow(dead_code)]
    device: Arc<wgpu::Device>,
    #[allow(dead_code)]
    queue: Arc<wgpu::Queue>,
    pub(crate) input_size: u32,
    pub(crate) hidden_size: u32,
}

impl RNNCell {
    /// Create new RNN Cell
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

    /// Forward pass through RNN cell
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor [batch, d_input]
    /// * `hidden` - Previous hidden state [batch, d_hidden]
    /// * `w_ih` - Input-hidden weights [d_input, d_hidden]
    /// * `w_hh` - Hidden-hidden weights [d_hidden, d_hidden]
    /// * `b_ih` - Input-hidden bias `d_hidden`
    /// * `b_hh` - Hidden-hidden bias `d_hidden`
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
        let mut output = vec![0.0f32; (batch * self.hidden_size) as usize];

        // h_t = tanh(W_ih·x_t + b_ih + W_hh·h_{t-1} + b_hh)
        for b in 0..batch {
            for h in 0..self.hidden_size {
                let mut sum = 0.0f32;

                // W_ih·x_t
                for i in 0..self.input_size {
                    let input_idx = (b * self.input_size + i) as usize;
                    let weight_idx = (i * self.hidden_size + h) as usize;
                    sum += input[input_idx] * w_ih[weight_idx];
                }

                // + b_ih
                sum += b_ih[h as usize];

                // W_hh·h_{t-1}
                for hh in 0..self.hidden_size {
                    let hidden_idx = (b * self.hidden_size + hh) as usize;
                    let weight_idx = (hh * self.hidden_size + h) as usize;
                    sum += hidden[hidden_idx] * w_hh[weight_idx];
                }

                // + b_hh
                sum += b_hh[h as usize];

                // tanh activation
                let output_idx = (b * self.hidden_size + h) as usize;
                output[output_idx] = sum.tanh();
            }
        }

        Ok(output)
    }
}
