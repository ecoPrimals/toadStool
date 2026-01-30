//! GRU Cell - Gated Recurrent Unit
//!
//! ## Deep Debt Principles
//!
//! - **Simpler than LSTM**: Fewer gates, faster computation
//! - **Production-ready**: Complete implementation with all gates
//! - **Modern Rust**: Clean API, proper error handling
//!
//! ## Algorithm
//!
//! ```text
//! r_t = sigmoid(W_ir * x_t + b_ir + W_hr * h_{t-1} + b_hr)  // Reset gate
//! z_t = sigmoid(W_iz * x_t + b_iz + W_hz * h_{t-1} + b_hz)  // Update gate
//! n_t = tanh(W_in * x_t + b_in + r_t ⊙ (W_hn * h_{t-1} + b_hn))  // New gate
//! h_t = (1 - z_t) ⊙ n_t + z_t ⊙ h_{t-1}                    // Hidden state
//! ```
//!
//! Reference: Cho et al. (2014)

/// GRU cell weights
pub struct GRUWeights {
    pub w_ir: Vec<f32>,  // Reset gate input weights
    pub w_hr: Vec<f32>,  // Reset gate hidden weights
    pub w_iz: Vec<f32>,  // Update gate input weights
    pub w_hz: Vec<f32>,  // Update gate hidden weights
    pub w_in: Vec<f32>,  // New gate input weights
    pub w_hn: Vec<f32>,  // New gate hidden weights
    
    pub b_ir: Vec<f32>,
    pub b_hr: Vec<f32>,
    pub b_iz: Vec<f32>,
    pub b_hz: Vec<f32>,
    pub b_in: Vec<f32>,
    pub b_hn: Vec<f32>,
}

/// GRU cell forward pass
///
/// ## Usage
///
/// ```no_run
/// use barracuda::ops::gru_cell::*;
///
/// # async fn example(device: &wgpu::Device, queue: &wgpu::Queue) {
/// let batch_size = 2;
/// let input_size = 128;
/// let hidden_size = 256;
///
/// let input = vec![0.5; batch_size * input_size];
/// let prev_hidden = vec![0.0; batch_size * hidden_size];
///
/// let weights = GRUWeights {
///     w_ir: vec![0.01; hidden_size * input_size],
///     // ... other weights ...
/// #     w_hr: vec![0.01; hidden_size * hidden_size],
/// #     w_iz: vec![0.01; hidden_size * input_size],
/// #     w_hz: vec![0.01; hidden_size * hidden_size],
/// #     w_in: vec![0.01; hidden_size * input_size],
/// #     w_hn: vec![0.01; hidden_size * hidden_size],
/// #     b_ir: vec![0.0; hidden_size],
/// #     b_hr: vec![0.0; hidden_size],
/// #     b_iz: vec![0.0; hidden_size],
/// #     b_hz: vec![0.0; hidden_size],
/// #     b_in: vec![0.0; hidden_size],
/// #     b_hn: vec![0.0; hidden_size],
/// };
///
/// let hidden = gru_cell(
///     device, queue,
///     &input, &prev_hidden,
///     &weights,
///     batch_size, input_size, hidden_size
/// ).await.unwrap();
/// # }
/// ```
///
/// ## Deep Debt Note
///
/// Current: CPU implementation
/// Evolution: GPU kernel for performance
pub async fn gru_cell(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    prev_hidden: &[f32],
    weights: &GRUWeights,
    batch_size: usize,
    input_size: usize,
    hidden_size: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    fn sigmoid(x: f32) -> f32 {
        1.0 / (1.0 + (-x).exp())
    }
    
    fn tanh(x: f32) -> f32 {
        x.tanh()
    }
    
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
    
    // Compute reset gate: r_t = sigmoid(W_ir * x + W_hr * h + b)
    let mut r_input = vec![0.0f32; batch_size * hidden_size];
    let mut r_hidden = vec![0.0f32; batch_size * hidden_size];
    matmul_add_bias(input, &weights.w_ir, &weights.b_ir, &mut r_input, batch_size, input_size, hidden_size);
    matmul_add_bias(prev_hidden, &weights.w_hr, &weights.b_hr, &mut r_hidden, batch_size, hidden_size, hidden_size);
    let mut r_gate = vec![0.0f32; batch_size * hidden_size];
    for i in 0..r_gate.len() {
        r_gate[i] = sigmoid(r_input[i] + r_hidden[i]);
    }
    
    // Compute update gate: z_t = sigmoid(W_iz * x + W_hz * h + b)
    let mut z_input = vec![0.0f32; batch_size * hidden_size];
    let mut z_hidden = vec![0.0f32; batch_size * hidden_size];
    matmul_add_bias(input, &weights.w_iz, &weights.b_iz, &mut z_input, batch_size, input_size, hidden_size);
    matmul_add_bias(prev_hidden, &weights.w_hz, &weights.b_hz, &mut z_hidden, batch_size, hidden_size, hidden_size);
    let mut z_gate = vec![0.0f32; batch_size * hidden_size];
    for i in 0..z_gate.len() {
        z_gate[i] = sigmoid(z_input[i] + z_hidden[i]);
    }
    
    // Compute new gate: n_t = tanh(W_in * x + r_t ⊙ (W_hn * h + b))
    let mut n_input = vec![0.0f32; batch_size * hidden_size];
    let mut n_hidden = vec![0.0f32; batch_size * hidden_size];
    matmul_add_bias(input, &weights.w_in, &weights.b_in, &mut n_input, batch_size, input_size, hidden_size);
    matmul_add_bias(prev_hidden, &weights.w_hn, &weights.b_hn, &mut n_hidden, batch_size, hidden_size, hidden_size);
    let mut n_gate = vec![0.0f32; batch_size * hidden_size];
    for i in 0..n_gate.len() {
        n_gate[i] = tanh(n_input[i] + r_gate[i] * n_hidden[i]);
    }
    
    // Compute hidden state: h_t = (1 - z_t) ⊙ n_t + z_t ⊙ h_{t-1}
    let mut hidden = vec![0.0f32; batch_size * hidden_size];
    for i in 0..hidden.len() {
        hidden[i] = (1.0 - z_gate[i]) * n_gate[i] + z_gate[i] * prev_hidden[i];
    }
    
    Ok(hidden)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_gru_cell_dimensions() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        
        let batch_size = 2;
        let input_size = 4;
        let hidden_size = 8;
        
        let input = vec![0.5; batch_size * input_size];
        let prev_hidden = vec![0.0; batch_size * hidden_size];
        
        let weights = GRUWeights {
            w_ir: vec![0.01; hidden_size * input_size],
            w_hr: vec![0.01; hidden_size * hidden_size],
            w_iz: vec![0.01; hidden_size * input_size],
            w_hz: vec![0.01; hidden_size * hidden_size],
            w_in: vec![0.01; hidden_size * input_size],
            w_hn: vec![0.01; hidden_size * hidden_size],
            b_ir: vec![0.0; hidden_size],
            b_hr: vec![0.0; hidden_size],
            b_iz: vec![0.0; hidden_size],
            b_hz: vec![0.0; hidden_size],
            b_in: vec![0.0; hidden_size],
            b_hn: vec![0.0; hidden_size],
        };
        
        let hidden = gru_cell(
            &device, &queue,
            &input, &prev_hidden,
            &weights,
            batch_size, input_size, hidden_size
        ).await.unwrap();
        
        assert_eq!(hidden.len(), batch_size * hidden_size);
    }
}
