//! BiLSTM - Bidirectional LSTM
//!
//! Processes sequence in both forward and backward directions.

use super::lstm_cell::{lstm_cell, LSTMWeights};

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
        let state = lstm_cell(
            device,
            queue,
            input,
            &fwd_h,
            &fwd_c,
            &weights.forward,
            batch_size,
            input_size,
            hidden_size,
        )
        .await?;
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
        let state = lstm_cell(
            device,
            queue,
            input,
            &bwd_h,
            &bwd_c,
            &weights.backward,
            batch_size,
            input_size,
            hidden_size,
        )
        .await?;
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
    use crate::device::test_pool::get_test_device;

    fn create_lstm_weights(input_size: usize, hidden: usize) -> LSTMWeights {
        LSTMWeights {
            w_ii: vec![0.01; hidden * input_size],
            w_hi: vec![0.01; hidden * hidden],
            w_if: vec![0.01; hidden * input_size],
            w_hf: vec![0.01; hidden * hidden],
            w_ig: vec![0.01; hidden * input_size],
            w_hg: vec![0.01; hidden * hidden],
            w_io: vec![0.01; hidden * input_size],
            w_ho: vec![0.01; hidden * hidden],
            b_ii: vec![0.0; hidden],
            b_hi: vec![0.0; hidden],
            b_if: vec![0.0; hidden],
            b_hf: vec![0.0; hidden],
            b_ig: vec![0.0; hidden],
            b_hg: vec![0.0; hidden],
            b_io: vec![0.0; hidden],
            b_ho: vec![0.0; hidden],
        }
    }

    #[tokio::test]
    async fn test_bi_lstm_basic() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        let seq_len = 3;
        let batch = 1;
        let input_size = 2;
        let hidden = 2;

        let sequence = vec![0.5; seq_len * batch * input_size];
        let fwd_weights = create_lstm_weights(input_size, hidden);

        let weights = BiLSTMWeights {
            forward: fwd_weights.clone(),
            backward: fwd_weights,
        };

        let output = bi_lstm(
            &device, &queue, &sequence, &weights, seq_len, batch, input_size, hidden,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), seq_len * batch * hidden * 2); // *2 for bidirectional
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_bi_lstm_edge_cases() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Single timestep
        let seq_len = 1;
        let batch = 1;
        let input_size = 2;
        let hidden = 2;

        let sequence = vec![1.0; seq_len * batch * input_size];
        let fwd_weights = create_lstm_weights(input_size, hidden);

        let weights = BiLSTMWeights {
            forward: fwd_weights.clone(),
            backward: fwd_weights,
        };

        let output = bi_lstm(
            &device, &queue, &sequence, &weights, seq_len, batch, input_size, hidden,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), seq_len * batch * hidden * 2);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_bi_lstm_boundary() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Longer sequence
        let seq_len = 8;
        let batch = 1;
        let input_size = 4;
        let hidden = 4;

        let sequence: Vec<f32> = (0..seq_len * batch * input_size)
            .map(|i| (i % 3) as f32 * 0.1)
            .collect();
        let fwd_weights = create_lstm_weights(input_size, hidden);

        let weights = BiLSTMWeights {
            forward: fwd_weights.clone(),
            backward: fwd_weights,
        };

        let output = bi_lstm(
            &device, &queue, &sequence, &weights, seq_len, batch, input_size, hidden,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), seq_len * batch * hidden * 2);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_bi_lstm_large_batch() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Multiple batches
        let seq_len = 4;
        let batch = 4;
        let input_size = 8;
        let hidden = 8;

        let sequence = vec![0.1; seq_len * batch * input_size];
        let fwd_weights = create_lstm_weights(input_size, hidden);

        let weights = BiLSTMWeights {
            forward: fwd_weights.clone(),
            backward: fwd_weights,
        };

        let output = bi_lstm(
            &device, &queue, &sequence, &weights, seq_len, batch, input_size, hidden,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), seq_len * batch * hidden * 2);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_bi_lstm_precision() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Test that forward and backward are concatenated
        let seq_len = 2;
        let batch = 1;
        let input_size = 2;
        let hidden = 2;

        let sequence = vec![1.0; seq_len * batch * input_size];
        let fwd_weights = create_lstm_weights(input_size, hidden);

        let weights = BiLSTMWeights {
            forward: fwd_weights.clone(),
            backward: fwd_weights,
        };

        let output = bi_lstm(
            &device, &queue, &sequence, &weights, seq_len, batch, input_size, hidden,
        )
        .await
        .unwrap();

        // Output format: [fwd_t0, bwd_t0, fwd_t1, bwd_t1]
        // Each timestep has hidden*2 values
        assert_eq!(output.len(), seq_len * batch * hidden * 2);

        // First timestep should have forward and backward components
        let t0_fwd = &output[0..hidden];
        let t0_bwd = &output[hidden..hidden * 2];

        assert!(t0_fwd.iter().all(|&x| x.is_finite()));
        assert!(t0_bwd.iter().all(|&x| x.is_finite()));
    }
}
