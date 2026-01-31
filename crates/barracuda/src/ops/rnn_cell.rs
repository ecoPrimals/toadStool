//! RNN Cell - Basic recurrent neural network cell
//!
//! ## Algorithm
//!
//! ```text
//! h_t = tanh(W_ih * x_t + b_ih + W_hh * h_{t-1} + b_hh)
//! ```

#[derive(Clone)]
pub struct RNNWeights {
    pub w_ih: Vec<f32>,
    pub w_hh: Vec<f32>,
    pub b_ih: Vec<f32>,
    pub b_hh: Vec<f32>,
}

pub async fn rnn_cell(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    prev_hidden: &[f32],
    weights: &RNNWeights,
    batch_size: usize,
    input_size: usize,
    hidden_size: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut hidden = vec![0.0f32; batch_size * hidden_size];
    
    for b in 0..batch_size {
        for h in 0..hidden_size {
            let mut sum = weights.b_ih[h] + weights.b_hh[h];
            
            for i in 0..input_size {
                sum += input[b * input_size + i] * weights.w_ih[h * input_size + i];
            }
            
            for i in 0..hidden_size {
                sum += prev_hidden[b * hidden_size + i] * weights.w_hh[h * hidden_size + i];
            }
            
            hidden[b * hidden_size + h] = sum.tanh();
        }
    }
    
    Ok(hidden)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_rnn_cell_basic() {
        let dev = get_test_device().await;
        let input = vec![0.5; 2 * 4]; // batch=2, input=4
        let prev_hidden = vec![0.0; 2 * 8]; // batch=2, hidden=8
        let weights = RNNWeights {
            w_ih: vec![0.01; 8 * 4],
            w_hh: vec![0.01; 8 * 8],
            b_ih: vec![0.0; 8],
            b_hh: vec![0.0; 8],
        };
        let hidden = rnn_cell(&dev.device, &dev.queue, &input, &prev_hidden, &weights, 2, 4, 8).await.unwrap();
        assert_eq!(hidden.len(), 16);
        assert!(hidden.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_rnn_cell_edge_cases() {
        let dev = get_test_device().await;

        // Single batch
        let input = vec![1.0; 1 * 3];
        let prev_hidden = vec![0.0; 1 * 4];
        let weights = RNNWeights {
            w_ih: vec![0.1; 4 * 3],
            w_hh: vec![0.1; 4 * 4],
            b_ih: vec![0.0; 4],
            b_hh: vec![0.0; 4],
        };
        let hidden = rnn_cell(&dev.device, &dev.queue, &input, &prev_hidden, &weights, 1, 3, 4).await.unwrap();
        assert_eq!(hidden.len(), 4);

        // Small hidden size
        let input = vec![0.5; 2 * 5];
        let prev_hidden = vec![0.0; 2 * 2];
        let weights = RNNWeights {
            w_ih: vec![0.01; 2 * 5],
            w_hh: vec![0.01; 2 * 2],
            b_ih: vec![0.0; 2],
            b_hh: vec![0.0; 2],
        };
        let hidden = rnn_cell(&dev.device, &dev.queue, &input, &prev_hidden, &weights, 2, 5, 2).await.unwrap();
        assert_eq!(hidden.len(), 4);
    }

    #[tokio::test]
    async fn test_rnn_cell_boundary() {
        let dev = get_test_device().await;

        // Non-zero previous hidden state
        let input = vec![0.5; 1 * 4];
        let prev_hidden = vec![0.5; 1 * 8];
        let weights = RNNWeights {
            w_ih: vec![0.1; 8 * 4],
            w_hh: vec![0.1; 8 * 8],
            b_ih: vec![0.1; 8],
            b_hh: vec![0.1; 8],
        };
        let hidden = rnn_cell(&dev.device, &dev.queue, &input, &prev_hidden, &weights, 1, 4, 8).await.unwrap();
        assert!(hidden.iter().all(|&x| x.is_finite()));
        // tanh bounds: -1 < x < 1
        assert!(hidden.iter().all(|&x| x > -1.0 && x < 1.0));
    }

    #[tokio::test]
    async fn test_rnn_cell_large_batch() {
        let dev = get_test_device().await;

        // Batch size 32
        let batch_size = 32;
        let input = vec![0.5; batch_size * 10];
        let prev_hidden = vec![0.0; batch_size * 20];
        let weights = RNNWeights {
            w_ih: vec![0.01; 20 * 10],
            w_hh: vec![0.01; 20 * 20],
            b_ih: vec![0.0; 20],
            b_hh: vec![0.0; 20],
        };
        let hidden = rnn_cell(&dev.device, &dev.queue, &input, &prev_hidden, &weights, batch_size, 10, 20).await.unwrap();
        assert_eq!(hidden.len(), batch_size * 20);
    }

    #[tokio::test]
    async fn test_rnn_cell_precision() {
        let dev = get_test_device().await;

        // Test with known values
        let input = vec![1.0; 1 * 2];
        let prev_hidden = vec![0.0; 1 * 2];
        let weights = RNNWeights {
            w_ih: vec![0.1, 0.2, 0.3, 0.4],
            w_hh: vec![0.1, 0.2, 0.3, 0.4],
            b_ih: vec![0.0, 0.0],
            b_hh: vec![0.0, 0.0],
        };
        let hidden = rnn_cell(&dev.device, &dev.queue, &input, &prev_hidden, &weights, 1, 2, 2).await.unwrap();
        
        assert_eq!(hidden.len(), 2);
        assert!(hidden.iter().all(|&x| x.is_finite()));
        // Values should be within tanh bounds
        assert!(hidden.iter().all(|&x| x.abs() < 1.0));
    }
}
