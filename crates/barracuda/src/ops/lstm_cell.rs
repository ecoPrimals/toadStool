//! LSTM Cell - Long Short-Term Memory unit
//!
//! ## Deep Debt Principles
//!
//! - **Complete implementation**: All gates (input, forget, output, cell)
//! - **Production-ready**: Handles hidden state and cell state correctly
//! - **Modern Rust**: Clean API with proper state management
//!
//! ## Algorithm
//!
//! ```text
//! i_t = sigmoid(W_ii * x_t + b_ii + W_hi * h_{t-1} + b_hi)  // Input gate
//! f_t = sigmoid(W_if * x_t + b_if + W_hf * h_{t-1} + b_hf)  // Forget gate
//! g_t = tanh(W_ig * x_t + b_ig + W_hg * h_{t-1} + b_hg)     // Cell gate
//! o_t = sigmoid(W_io * x_t + b_io + W_ho * h_{t-1} + b_ho)  // Output gate
//!
//! c_t = f_t ⊙ c_{t-1} + i_t ⊙ g_t                           // Cell state
//! h_t = o_t ⊙ tanh(c_t)                                      // Hidden state
//! ```
//!
//! Where ⊙ denotes element-wise multiplication.
//!
//! Reference: Hochreiter & Schmidhuber (1997)

/// LSTM cell state
#[derive(Debug, Clone)]
pub struct LSTMState {
    /// Hidden state [batch, hidden_size]
    pub hidden: Vec<f32>,
    /// Cell state [batch, hidden_size]
    pub cell: Vec<f32>,
}

/// LSTM cell weights
#[derive(Clone)]
pub struct LSTMWeights {
    /// Input gate weights: W_ii [hidden_size, input_size]
    pub w_ii: Vec<f32>,
    /// Input gate hidden weights: W_hi [hidden_size, hidden_size]
    pub w_hi: Vec<f32>,
    /// Forget gate weights: W_if [hidden_size, input_size]
    pub w_if: Vec<f32>,
    /// Forget gate hidden weights: W_hf [hidden_size, hidden_size]
    pub w_hf: Vec<f32>,
    /// Cell gate weights: W_ig [hidden_size, input_size]
    pub w_ig: Vec<f32>,
    /// Cell gate hidden weights: W_hg [hidden_size, hidden_size]
    pub w_hg: Vec<f32>,
    /// Output gate weights: W_io [hidden_size, input_size]
    pub w_io: Vec<f32>,
    /// Output gate hidden weights: W_ho [hidden_size, hidden_size]
    pub w_ho: Vec<f32>,
    
    /// Biases (can be zero)
    pub b_ii: Vec<f32>,
    pub b_hi: Vec<f32>,
    pub b_if: Vec<f32>,
    pub b_hf: Vec<f32>,
    pub b_ig: Vec<f32>,
    pub b_hg: Vec<f32>,
    pub b_io: Vec<f32>,
    pub b_ho: Vec<f32>,
}

/// LSTM cell forward pass
///
/// ## Usage
///
/// ```no_run
/// use barracuda::ops::lstm_cell::*;
///
/// # async fn example(device: &wgpu::Device, queue: &wgpu::Queue) {
/// let batch_size = 2;
/// let input_size = 128;
/// let hidden_size = 256;
///
/// let input = vec![0.5; batch_size * input_size];
/// let prev_hidden = vec![0.0; batch_size * hidden_size];
/// let prev_cell = vec![0.0; batch_size * hidden_size];
///
/// // Initialize weights (in practice, from trained model)
/// let weights = LSTMWeights {
///     w_ii: vec![0.01; hidden_size * input_size],
///     w_hi: vec![0.01; hidden_size * hidden_size],
///     // ... other weights ...
/// #     w_if: vec![0.01; hidden_size * input_size],
/// #     w_hf: vec![0.01; hidden_size * hidden_size],
/// #     w_ig: vec![0.01; hidden_size * input_size],
/// #     w_hg: vec![0.01; hidden_size * hidden_size],
/// #     w_io: vec![0.01; hidden_size * input_size],
/// #     w_ho: vec![0.01; hidden_size * hidden_size],
/// #     b_ii: vec![0.0; hidden_size],
/// #     b_hi: vec![0.0; hidden_size],
/// #     b_if: vec![0.0; hidden_size],
/// #     b_hf: vec![0.0; hidden_size],
/// #     b_ig: vec![0.0; hidden_size],
/// #     b_hg: vec![0.0; hidden_size],
/// #     b_io: vec![0.0; hidden_size],
/// #     b_ho: vec![0.0; hidden_size],
/// };
///
/// let state = lstm_cell(
///     device, queue,
///     &input, &prev_hidden, &prev_cell,
///     &weights,
///     batch_size, input_size, hidden_size
/// ).await.unwrap();
/// # }
/// ```
///
/// ## Deep Debt Note
///
/// Current: CPU implementation for correctness
/// Evolution: GPU kernel with fused operations for performance
pub async fn lstm_cell(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],              // [batch, input_size]
    prev_hidden: &[f32],        // [batch, hidden_size]
    prev_cell: &[f32],          // [batch, hidden_size]
    weights: &LSTMWeights,
    batch_size: usize,
    input_size: usize,
    hidden_size: usize,
) -> Result<LSTMState, Box<dyn std::error::Error>> {
    // Validate dimensions
    if input.len() != batch_size * input_size {
        return Err(format!("Input size mismatch: expected {}, got {}", 
            batch_size * input_size, input.len()).into());
    }
    
    if prev_hidden.len() != batch_size * hidden_size {
        return Err("Hidden state size mismatch".into());
    }
    
    // Helper: sigmoid activation
    fn sigmoid(x: f32) -> f32 {
        1.0 / (1.0 + (-x).exp())
    }
    
    // Helper: tanh activation
    fn tanh(x: f32) -> f32 {
        x.tanh()
    }
    
    // Helper: matrix-vector product + bias
    fn matmul_add_bias(
        input: &[f32],
        weights: &[f32],
        bias: &[f32],
        output: &mut [f32],
        batch_size: usize,
        in_size: usize,
        out_size: usize,
    ) {
        for b in 0..batch_size {
            for i in 0..out_size {
                let mut sum = bias[i];
                for j in 0..in_size {
                    sum += input[b * in_size + j] * weights[i * in_size + j];
                }
                output[b * out_size + i] = sum;
            }
        }
    }
    
    // Allocate gate activations
    let mut i_gate = vec![0.0f32; batch_size * hidden_size];
    let mut f_gate = vec![0.0f32; batch_size * hidden_size];
    let mut g_gate = vec![0.0f32; batch_size * hidden_size];
    let mut o_gate = vec![0.0f32; batch_size * hidden_size];
    
    // Compute gate pre-activations
    let mut i_input = vec![0.0f32; batch_size * hidden_size];
    let mut i_hidden = vec![0.0f32; batch_size * hidden_size];
    matmul_add_bias(input, &weights.w_ii, &weights.b_ii, &mut i_input, batch_size, input_size, hidden_size);
    matmul_add_bias(prev_hidden, &weights.w_hi, &weights.b_hi, &mut i_hidden, batch_size, hidden_size, hidden_size);
    for i in 0..i_gate.len() {
        i_gate[i] = sigmoid(i_input[i] + i_hidden[i]);
    }
    
    let mut f_input = vec![0.0f32; batch_size * hidden_size];
    let mut f_hidden = vec![0.0f32; batch_size * hidden_size];
    matmul_add_bias(input, &weights.w_if, &weights.b_if, &mut f_input, batch_size, input_size, hidden_size);
    matmul_add_bias(prev_hidden, &weights.w_hf, &weights.b_hf, &mut f_hidden, batch_size, hidden_size, hidden_size);
    for i in 0..f_gate.len() {
        f_gate[i] = sigmoid(f_input[i] + f_hidden[i]);
    }
    
    let mut g_input = vec![0.0f32; batch_size * hidden_size];
    let mut g_hidden = vec![0.0f32; batch_size * hidden_size];
    matmul_add_bias(input, &weights.w_ig, &weights.b_ig, &mut g_input, batch_size, input_size, hidden_size);
    matmul_add_bias(prev_hidden, &weights.w_hg, &weights.b_hg, &mut g_hidden, batch_size, hidden_size, hidden_size);
    for i in 0..g_gate.len() {
        g_gate[i] = tanh(g_input[i] + g_hidden[i]);
    }
    
    let mut o_input = vec![0.0f32; batch_size * hidden_size];
    let mut o_hidden = vec![0.0f32; batch_size * hidden_size];
    matmul_add_bias(input, &weights.w_io, &weights.b_io, &mut o_input, batch_size, input_size, hidden_size);
    matmul_add_bias(prev_hidden, &weights.w_ho, &weights.b_ho, &mut o_hidden, batch_size, hidden_size, hidden_size);
    for i in 0..o_gate.len() {
        o_gate[i] = sigmoid(o_input[i] + o_hidden[i]);
    }
    
    // Update cell state: c_t = f_t ⊙ c_{t-1} + i_t ⊙ g_t
    let mut cell = vec![0.0f32; batch_size * hidden_size];
    for i in 0..cell.len() {
        cell[i] = f_gate[i] * prev_cell[i] + i_gate[i] * g_gate[i];
    }
    
    // Update hidden state: h_t = o_t ⊙ tanh(c_t)
    let mut hidden = vec![0.0f32; batch_size * hidden_size];
    for i in 0..hidden.len() {
        hidden[i] = o_gate[i] * tanh(cell[i]);
    }
    
    Ok(LSTMState { hidden, cell })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_lstm_cell_dimensions() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        
        let batch_size = 2;
        let input_size = 4;
        let hidden_size = 8;
        
        let input = vec![0.5; batch_size * input_size];
        let prev_hidden = vec![0.0; batch_size * hidden_size];
        let prev_cell = vec![0.0; batch_size * hidden_size];
        
        let weights = LSTMWeights {
            w_ii: vec![0.01; hidden_size * input_size],
            w_hi: vec![0.01; hidden_size * hidden_size],
            w_if: vec![0.01; hidden_size * input_size],
            w_hf: vec![0.01; hidden_size * hidden_size],
            w_ig: vec![0.01; hidden_size * input_size],
            w_hg: vec![0.01; hidden_size * hidden_size],
            w_io: vec![0.01; hidden_size * input_size],
            w_ho: vec![0.01; hidden_size * hidden_size],
            b_ii: vec![0.0; hidden_size],
            b_hi: vec![0.0; hidden_size],
            b_if: vec![0.0; hidden_size],
            b_hf: vec![0.0; hidden_size],
            b_ig: vec![0.0; hidden_size],
            b_hg: vec![0.0; hidden_size],
            b_io: vec![0.0; hidden_size],
            b_ho: vec![0.0; hidden_size],
        };
        
        let state = lstm_cell(
            &device, &queue,
            &input, &prev_hidden, &prev_cell,
            &weights,
            batch_size, input_size, hidden_size
        ).await.unwrap();
        
        assert_eq!(state.hidden.len(), batch_size * hidden_size);
        assert_eq!(state.cell.len(), batch_size * hidden_size);
    }
}
