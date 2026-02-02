//! Adafactor - Memory-efficient adaptive learning rate method
//!
//! Reduces memory by factorizing second moment matrix.
//! Used in T5 and large-scale training.

pub struct AdafactorState {
    pub row_mean: Vec<f32>,
    pub col_mean: Vec<f32>,
    pub step: usize,
}

pub async fn adafactor_step(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    params: &[f32],
    grads: &[f32],
    state: &mut AdafactorState,
    lr: f32,
    beta2: f32,
    epsilon: f32,
    rows: usize,
    cols: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if params.len() != rows * cols || grads.len() != rows * cols {
        return Err("Dimension mismatch".into());
    }

    state.step += 1;

    // Update factorized second moment
    for r in 0..rows {
        let mut row_sum = 0.0;
        for c in 0..cols {
            let g = grads[r * cols + c];
            row_sum += g * g;
        }
        state.row_mean[r] = beta2 * state.row_mean[r] + (1.0 - beta2) * row_sum / cols as f32;
    }

    for c in 0..cols {
        let mut col_sum = 0.0;
        for r in 0..rows {
            let g = grads[r * cols + c];
            col_sum += g * g;
        }
        state.col_mean[c] = beta2 * state.col_mean[c] + (1.0 - beta2) * col_sum / rows as f32;
    }

    // Update parameters using factorized approximation
    let mut new_params = params.to_vec();
    for r in 0..rows {
        for c in 0..cols {
            let idx = r * cols + c;
            // Approximate v[i] as outer product of row and col means
            let v_approx = state.row_mean[r] * state.col_mean[c];
            let rms = (v_approx + epsilon).sqrt();
            new_params[idx] = params[idx] - lr * grads[idx] / rms;
        }
    }

    Ok(new_params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_adafactor_basic() {
        let dev = get_test_device().await;
        let params = vec![1.0; 10 * 10];
        let grads = vec![0.01; 10 * 10];
        let mut state = AdafactorState {
            row_mean: vec![0.0; 10],
            col_mean: vec![0.0; 10],
            step: 0,
        };
        let new_params = adafactor_step(
            &dev.device,
            &dev.queue,
            &params,
            &grads,
            &mut state,
            0.001,
            0.999,
            1e-8,
            10,
            10,
        )
        .await
        .unwrap();
        assert_eq!(new_params.len(), 100);
        assert!(new_params.iter().all(|&x| x.is_finite()));
        // Params should decrease with positive gradients
        assert!(new_params.iter().zip(params.iter()).all(|(a, b)| a < b));
    }

    #[tokio::test]
    async fn test_adafactor_edge_cases() {
        let dev = get_test_device().await;

        // Test with zero gradients
        let params = vec![1.0; 4 * 4];
        let grads = vec![0.0; 4 * 4];
        let mut state = AdafactorState {
            row_mean: vec![0.0; 4],
            col_mean: vec![0.0; 4],
            step: 0,
        };
        let new_params = adafactor_step(
            &dev.device,
            &dev.queue,
            &params,
            &grads,
            &mut state,
            0.001,
            0.999,
            1e-8,
            4,
            4,
        )
        .await
        .unwrap();
        assert!(new_params.iter().all(|&x| x.is_finite()));

        // Test with single element (1x1 matrix)
        let params = vec![5.0];
        let grads = vec![0.1];
        let mut state = AdafactorState {
            row_mean: vec![0.0],
            col_mean: vec![0.0],
            step: 0,
        };
        let new_params = adafactor_step(
            &dev.device,
            &dev.queue,
            &params,
            &grads,
            &mut state,
            0.01,
            0.999,
            1e-8,
            1,
            1,
        )
        .await
        .unwrap();
        assert_eq!(new_params.len(), 1);
        assert!(new_params[0] < 5.0);
    }

    #[tokio::test]
    async fn test_adafactor_boundary() {
        let dev = get_test_device().await;

        // Test non-square matrices (memory efficiency benefit)
        let rows = 8;
        let cols = 16;
        let size = rows * cols;
        let params = vec![1.0; size];
        let grads = vec![0.05; size];
        let mut state = AdafactorState {
            row_mean: vec![0.0; rows],
            col_mean: vec![0.0; cols],
            step: 0,
        };

        let new_params = adafactor_step(
            &dev.device,
            &dev.queue,
            &params,
            &grads,
            &mut state,
            0.001,
            0.999,
            1e-8,
            rows,
            cols,
        )
        .await
        .unwrap();

        assert_eq!(new_params.len(), size);
        assert!(new_params.iter().all(|&x| x.is_finite()));
        // State should be factorized (rows + cols << rows * cols)
        assert!(state.row_mean.iter().any(|&x| x != 0.0));
        assert!(state.col_mean.iter().any(|&x| x != 0.0));
    }

    #[tokio::test]
    async fn test_adafactor_large_batch() {
        let dev = get_test_device().await;

        // Large matrix (T5-style large-scale training)
        let rows = 32;
        let cols = 32;
        let size = rows * cols;
        let params: Vec<f32> = (0..size).map(|i| (i as f32) / 100.0).collect();
        let grads = vec![0.01; size];
        let mut state = AdafactorState {
            row_mean: vec![0.0; rows],
            col_mean: vec![0.0; cols],
            step: 0,
        };

        let new_params = adafactor_step(
            &dev.device,
            &dev.queue,
            &params,
            &grads,
            &mut state,
            0.001,
            0.999,
            1e-8,
            rows,
            cols,
        )
        .await
        .unwrap();

        assert_eq!(new_params.len(), size);
        assert!(new_params.iter().all(|&x| x.is_finite()));
        assert_eq!(state.step, 1);
        // Memory saved: rows + cols = 64 vs rows * cols = 1024
    }

    #[tokio::test]
    async fn test_adafactor_precision() {
        let dev = get_test_device().await;

        // Test multiple optimization steps
        let rows = 5;
        let cols = 5;
        let mut params = vec![10.0; rows * cols];
        let grads = vec![1.0; rows * cols];
        let mut state = AdafactorState {
            row_mean: vec![0.0; rows],
            col_mean: vec![0.0; cols],
            step: 0,
        };

        // Step 1
        params = adafactor_step(
            &dev.device,
            &dev.queue,
            &params,
            &grads,
            &mut state,
            0.1,
            0.999,
            1e-8,
            rows,
            cols,
        )
        .await
        .unwrap();
        assert!(params.iter().all(|&x| x.is_finite()));
        assert!(params.iter().all(|&x| x < 10.0));

        // Step 2 (factorized moments accumulated)
        let params_step2 = adafactor_step(
            &dev.device,
            &dev.queue,
            &params,
            &grads,
            &mut state,
            0.1,
            0.999,
            1e-8,
            rows,
            cols,
        )
        .await
        .unwrap();
        assert!(params_step2.iter().all(|&x| x.is_finite()));
        // Should continue decreasing
        assert!(params_step2.iter().zip(params.iter()).all(|(a, b)| a < b));
    }
}
