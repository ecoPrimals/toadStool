//! BiLSTM - Bidirectional LSTM
//!
//! Processes sequence in both forward and backward directions.

use super::lstm_cell::{LSTMWeights, lstm_cell};

pub struct BiLSTMWeights {
    pub forward: LSTMWeights,
    pub backward: LSTMWeights,
}

/// Bidirectional LSTM over a sequence
///
/// ## Usage
///
/// Process sequence with both forward and backward LSTM cells,
/// concatenating the outputs.
pub async fn bi_lstm(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sequence: &[f32], // [seq_len, batch, input_size]
    weights: &BiLSTMWeights,
    seq_len: usize,
    batch_size: usize,
    input_size: usize,
    hidden_size: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Forward pass
    let mut forward_outputs = Vec::new();
    let mut fwd_h = vec![0.0f32; batch_size * hidden_size];
    let mut fwd_c = vec![0.0f32; batch_size * hidden_size];
    
    for t in 0..seq_len {
        let input = &sequence[t * batch_size * input_size..(t + 1) * batch_size * input_size];
        let state = lstm_cell(device, queue, input, &fwd_h, &fwd_c, &weights.forward, 
            batch_size, input_size, hidden_size).await?;
        forward_outputs.push(state.hidden.clone());
        fwd_h = state.hidden;
        fwd_c = state.cell;
    }
    
    // Backward pass
    let mut backward_outputs = Vec::new();
    let mut bwd_h = vec![0.0f32; batch_size * hidden_size];
    let mut bwd_c = vec![0.0f32; batch_size * hidden_size];
    
    for t in (0..seq_len).rev() {
        let input = &sequence[t * batch_size * input_size..(t + 1) * batch_size * input_size];
        let state = lstm_cell(device, queue, input, &bwd_h, &bwd_c, &weights.backward,
            batch_size, input_size, hidden_size).await?;
        backward_outputs.push(state.hidden.clone());
        bwd_h = state.hidden;
        bwd_c = state.cell;
    }
    backward_outputs.reverse();
    
    // Concatenate forward and backward outputs
    let mut output = Vec::with_capacity(seq_len * batch_size * hidden_size * 2);
    for t in 0..seq_len {
        output.extend_from_slice(&forward_outputs[t]);
        output.extend_from_slice(&backward_outputs[t]);
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_bi_lstm_dimensions() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        // Small example
        let seq_len = 3;
        let batch = 1;
        let input_size = 2;
        let hidden = 2;
        
        let sequence = vec![0.5; seq_len * batch * input_size];
        
        let fwd_weights = LSTMWeights {
            w_ii: vec![0.01; hidden * input_size], w_hi: vec![0.01; hidden * hidden],
            w_if: vec![0.01; hidden * input_size], w_hf: vec![0.01; hidden * hidden],
            w_ig: vec![0.01; hidden * input_size], w_hg: vec![0.01; hidden * hidden],
            w_io: vec![0.01; hidden * input_size], w_ho: vec![0.01; hidden * hidden],
            b_ii: vec![0.0; hidden], b_hi: vec![0.0; hidden],
            b_if: vec![0.0; hidden], b_hf: vec![0.0; hidden],
            b_ig: vec![0.0; hidden], b_hg: vec![0.0; hidden],
            b_io: vec![0.0; hidden], b_ho: vec![0.0; hidden],
        };
        
        let weights = BiLSTMWeights {
            forward: fwd_weights.clone(),
            backward: fwd_weights,
        };
        
        let output = bi_lstm(&device, &queue, &sequence, &weights, seq_len, batch, input_size, hidden).await.unwrap();
        assert_eq!(output.len(), seq_len * batch * hidden * 2); // *2 for bidirectional
    }
}
