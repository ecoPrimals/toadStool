//! LSTM Cell and Layer

use anyhow::Result;
use std::sync::Arc;

/// LSTM Cell (Long Short-Term Memory)
///
/// LSTM cell with forget gate, input gate, output gate, and cell state.
///
/// ## Architecture
///
/// ```text
/// f_t = σ(W_if·x_t + b_if + W_hf·h_{t-1} + b_hf)  # Forget gate
/// i_t = σ(W_ii·x_t + b_ii + W_hi·h_{t-1} + b_hi)  # Input gate
/// g_t = tanh(W_ig·x_t + b_ig + W_hg·h_{t-1} + b_hg)  # Cell gate
/// o_t = σ(W_io·x_t + b_io + W_ho·h_{t-1} + b_ho)  # Output gate
/// c_t = f_t ⊙ c_{t-1} + i_t ⊙ g_t  # Cell state
/// h_t = o_t ⊙ tanh(c_t)  # Hidden state
/// ```
///
/// ## Parameters
///
/// - Input size: d_input
/// - Hidden size: d_hidden
/// - 4 weight matrices (forget, input, cell, output)
/// - 4 bias vectors
pub struct LSTMCell {
    #[allow(dead_code)]
    device: Arc<wgpu::Device>,
    #[allow(dead_code)]
    queue: Arc<wgpu::Queue>,
    input_size: u32,
    hidden_size: u32,
}

impl LSTMCell {
    /// Create new LSTM Cell
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

    /// Forward pass through LSTM cell
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor [batch, d_input]
    /// * `hidden` - Previous hidden state [batch, d_hidden]
    /// * `cell` - Previous cell state [batch, d_hidden]
    /// * `w_ih` - Input-hidden weights [d_input, 4*d_hidden] (concat: f,i,g,o)
    /// * `w_hh` - Hidden-hidden weights [d_hidden, 4*d_hidden] (concat: f,i,g,o)
    /// * `b_ih` - Input-hidden bias `4*d_hidden`
    /// * `b_hh` - Hidden-hidden bias `4*d_hidden`
    /// * `batch` - Batch size
    ///
    /// # Returns
    ///
    /// Tuple of (new_hidden, new_cell): ([batch, d_hidden], [batch, d_hidden])
    #[allow(clippy::too_many_arguments)]
    pub async fn forward(
        &self,
        input: &[f32],
        hidden: &[f32],
        cell: &[f32],
        w_ih: &[f32],
        w_hh: &[f32],
        b_ih: &[f32],
        b_hh: &[f32],
        batch: u32,
    ) -> Result<(Vec<f32>, Vec<f32>)> {
        let mut new_hidden = vec![0.0f32; (batch * self.hidden_size) as usize];
        let mut new_cell = vec![0.0f32; (batch * self.hidden_size) as usize];

        for b in 0..batch {
            // Compute all 4 gates
            let mut gates = vec![0.0f32; (4 * self.hidden_size) as usize];

            for gate_idx in 0..(4 * self.hidden_size) {
                let mut sum = 0.0f32;

                // W_ih·x_t
                for i in 0..self.input_size {
                    let input_idx = (b * self.input_size + i) as usize;
                    let weight_idx = (i * 4 * self.hidden_size + gate_idx) as usize;
                    sum += input[input_idx] * w_ih[weight_idx];
                }

                // + b_ih
                sum += b_ih[gate_idx as usize];

                // W_hh·h_{t-1}
                for hh in 0..self.hidden_size {
                    let hidden_idx = (b * self.hidden_size + hh) as usize;
                    let weight_idx = (hh * 4 * self.hidden_size + gate_idx) as usize;
                    sum += hidden[hidden_idx] * w_hh[weight_idx];
                }

                // + b_hh
                sum += b_hh[gate_idx as usize];

                gates[gate_idx as usize] = sum;
            }

            // Apply activations and compute new states
            for h in 0..self.hidden_size {
                let h_usize = h as usize;

                // Forget gate: σ(gates[0:hidden_size])
                let f_t = Self::sigmoid(gates[h_usize]);

                // Input gate: σ(gates[hidden_size:2*hidden_size])
                let i_t = Self::sigmoid(gates[(self.hidden_size + h) as usize]);

                // Cell gate: tanh(gates[2*hidden_size:3*hidden_size])
                let g_t = gates[(2 * self.hidden_size + h) as usize].tanh();

                // Output gate: σ(gates[3*hidden_size:4*hidden_size])
                let o_t = Self::sigmoid(gates[(3 * self.hidden_size + h) as usize]);

                // New cell state: c_t = f_t ⊙ c_{t-1} + i_t ⊙ g_t
                let cell_idx = (b * self.hidden_size + h) as usize;
                let new_c = f_t * cell[cell_idx] + i_t * g_t;
                new_cell[cell_idx] = new_c;

                // New hidden state: h_t = o_t ⊙ tanh(c_t)
                new_hidden[cell_idx] = o_t * new_c.tanh();
            }
        }

        Ok((new_hidden, new_cell))
    }

    fn sigmoid(x: f32) -> f32 {
        1.0 / (1.0 + (-x).exp())
    }
}

/// GRU Cell (Gated Recurrent Unit)
///
/// Simplified LSTM with 2 gates (reset, update) instead of 3.
///
/// ## Architecture
///
/// ```text
/// r_t = σ(W_ir·x_t + b_ir + W_hr·h_{t-1} + b_hr)  # Reset gate
/// z_t = σ(W_iz·x_t + b_iz + W_hz·h_{t-1} + b_hz)  # Update gate
/// n_t = tanh(W_in·x_t + b_in + r_t ⊙ (W_hn·h_{t-1} + b_hn))  # New gate
/// h_t = (1 - z_t) ⊙ n_t + z_t ⊙ h_{t-1}  # Hidden state
/// ```
///
/// ## Benefits
///
/// - Fewer parameters than LSTM (no cell state)
pub struct LSTMLayer {
    cell: LSTMCell,
}

impl LSTMLayer {
    /// Create new LSTM Layer
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        input_size: u32,
        hidden_size: u32,
    ) -> Result<Self> {
        let cell = LSTMCell::new(device, queue, input_size, hidden_size).await?;
        Ok(Self { cell })
    }

    /// Forward pass through LSTM layer
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
    ) -> Result<(Vec<f32>, Vec<f32>)> {
        let hidden_size = self.cell.hidden_size;
        let input_size = self.cell.input_size;

        let mut outputs = Vec::new();
        let mut hidden = vec![0.0f32; (batch * hidden_size) as usize];
        let mut cell = vec![0.0f32; (batch * hidden_size) as usize];

        for t in 0..seq_len {
            let input_start = (batch * t * input_size) as usize;
            let input_end = (batch * (t + 1) * input_size) as usize;
            let input_t = &sequence[input_start..input_end];

            let (new_hidden, new_cell) = self
                .cell
                .forward(input_t, &hidden, &cell, w_ih, w_hh, b_ih, b_hh, batch)
                .await?;

            hidden = new_hidden;
            cell = new_cell;
            outputs.extend_from_slice(&hidden);
        }

        Ok((outputs, cell))
    }
}
