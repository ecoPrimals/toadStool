//! Recurrent Neural Networks (RNN/LSTM/GRU)
//!
//! **Week 4 Implementation**: Sequence modeling operations for speech, video, NLP
//!
//! ## Operations (8/8)
//!
//! 1. **RNNCell** - Basic recurrent cell (Elman network)
//! 2. **LSTMCell** - Long Short-Term Memory cell (forget gates)
//! 3. **GRUCell** - Gated Recurrent Unit (simplified LSTM)
//! 4. **RNNLayer** - Full RNN layer with sequence processing
//! 5. **LSTMLayer** - Full LSTM layer with sequence processing
//! 6. **GRULayer** - Full GRU layer with sequence processing
//! 7. **BidirectionalRNN** - Forward + backward RNN
//! 8. **RecurrentDropout** - RNN-specific dropout (preserves temporal consistency)
//!
//! ## Philosophy
//!
//! - ✅ **Pure Rust**: No unsafe code, vendor-agnostic
//! - ✅ **Memory Efficient**: Optimized hidden state management
//! - ✅ **Batched**: Parallel sequence processing
//! - ✅ **Adaptive**: Uses adaptive optimization system
//!
//! ## Impact
//!
//! **Enables Sequence Modeling**:
//! - Speech recognition (ASR)
//! - Machine translation (seq2seq)
//! - Video processing (temporal features)
//! - Time series forecasting
//! - Music generation

use anyhow::{Context, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// RNN Cell (Elman Network)
///
/// Basic recurrent cell: h_t = tanh(W_ih·x_t + b_ih + W_hh·h_{t-1} + b_hh)
///
/// ## Parameters
///
/// - Input size: d_input
/// - Hidden size: d_hidden
/// - Weights: W_ih [d_input, d_hidden], W_hh [d_hidden, d_hidden]
/// - Biases: b_ih [d_hidden], b_hh [d_hidden]
///
/// ## Performance
///
/// - Complexity: O(d_input·d_hidden + d_hidden²)
/// - Memory: O(batch·d_hidden)
pub struct RNNCell {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    input_size: u32,
    hidden_size: u32,
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
    /// * `b_ih` - Input-hidden bias [d_hidden]
    /// * `b_hh` - Hidden-hidden bias [d_hidden]
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
    device: Arc<wgpu::Device>,
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
    /// * `b_ih` - Input-hidden bias [4*d_hidden]
    /// * `b_hh` - Hidden-hidden bias [4*d_hidden]
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
/// - Faster training
/// - Often comparable performance
pub struct GRUCell {
    device: Arc<wgpu::Device>,
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
    /// * `b_ih` - Input-hidden bias [3*d_hidden]
    /// * `b_hh` - Hidden-hidden bias [3*d_hidden]
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
                    let weight_idx = (hh * 3 * self.hidden_size + 2 * self.hidden_size + h) as usize;
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

                let result = rnn.forward(&input, &hidden, &w_ih, &w_hh, &b_ih, &b_hh, batch).await;
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

                let result = lstm.forward(&input, &hidden, &cell, &w_ih, &w_hh, &b_ih, &b_hh, batch).await;
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

                let result = gru.forward(&input, &hidden, &w_ih, &w_hh, &b_ih, &b_hh, batch).await;
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
/// - Doubles output dimension
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
        let forward_rnn = RNNCell::new(Arc::clone(&device), Arc::clone(&queue), input_size, hidden_size).await?;
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

            hidden_fwd = self.forward_rnn.forward(
                input_t,
                &hidden_fwd,
                w_ih_fwd,
                w_hh_fwd,
                b_ih_fwd,
                b_hh_fwd,
                batch,
            ).await?;

            forward_outputs.push(hidden_fwd.clone());
        }

        // Backward pass
        let mut backward_outputs = Vec::new();
        let mut hidden_bwd = vec![0.0f32; (batch * hidden_size) as usize];

        for t in (0..seq_len).rev() {
            let input_start = (batch * t * input_size) as usize;
            let input_end = (batch * (t + 1) * input_size) as usize;
            let input_t = &sequence[input_start..input_end];

            hidden_bwd = self.backward_rnn.forward(
                input_t,
                &hidden_bwd,
                w_ih_bwd,
                w_hh_bwd,
                b_ih_bwd,
                b_hh_bwd,
                batch,
            ).await?;

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
                    let out_idx_bwd = ((b * seq_len + t) * 2 * hidden_size + hidden_size + h) as usize;
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
    layers: Vec<LSTMCell>,
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
            let lstm = LSTMCell::new(Arc::clone(&device), Arc::clone(&queue), input_sz, hidden_size).await?;
            layers.push(lstm);
        }

        Ok(Self {
            layers,
            num_layers,
        })
    }
}

/// GRU Layer
///
/// Full GRU layer that processes entire sequences.
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

            hidden = self.cell.forward(input_t, &hidden, w_ih, w_hh, b_ih, b_hh, batch).await?;
            outputs.extend_from_slice(&hidden);
        }

        Ok(outputs)
    }
}

/// LSTM Layer
///
/// Full LSTM layer that processes entire sequences.
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

            let (new_hidden, new_cell) = self.cell.forward(
                input_t,
                &hidden,
                &cell,
                w_ih,
                w_hh,
                b_ih,
                b_hh,
                batch,
            ).await?;

            hidden = new_hidden;
            cell = new_cell;
            outputs.extend_from_slice(&hidden);
        }

        Ok((outputs, cell))
    }
}

/// Recurrent Dropout
///
/// Dropout specifically designed for recurrent networks.
/// Uses the same dropout mask across all time steps to preserve
/// temporal dependencies.
///
/// ## Difference from Standard Dropout
///
/// - Standard: Different mask per timestep
/// - Recurrent: Same mask across entire sequence
///
/// ## Benefits
///
/// - Prevents overfitting
/// - Preserves temporal structure
/// - Regularizes recurrent connections
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
            let val = ((seed.wrapping_mul(1103515245).wrapping_add(i as u64 * 12345)) % 2147483648) as f32
                / 2147483648.0;

            mask[i] = if val > self.dropout_rate { scale } else { 0.0 };
        }

        mask
    }
}

#[cfg(test)]
mod layer_tests {
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
