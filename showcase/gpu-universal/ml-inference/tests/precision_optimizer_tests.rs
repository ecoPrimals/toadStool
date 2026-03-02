// Precision tests - Optimizer
use ml_inference_showcase::gpu_resilience::gpu_test_resilient_async;
use ml_inference_showcase::wgpu::WgpuExecutor;
use ml_inference_showcase::wgpu::*;

const FP32_TOLERANCE_RELAXED: f32 = 1e-4;

#[tokio::test]
async fn test_adam_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        // Adam: Adaptive Moment Estimation (most popular optimizer)
        let mut params = vec![1.0, 2.0, 3.0, 4.0];
        let grads = vec![0.1, 0.2, 0.3, 0.4];
        let mut m = vec![0.0; 4]; // First moment estimate
        let mut v = vec![0.0; 4]; // Second moment estimate
        let step = 1;

        let config = AdamConfig {
            learning_rate: 0.001,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            weight_decay: 0.0,
        };

        let initial_params = params.clone();

        executor
            .execute_adam_step(&grads, &mut params, &mut m, &mut v, step, config)
            .await
            .unwrap();

        // Parameters should decrease with positive gradients
        for i in 0..4 {
            assert!(
                params[i] < initial_params[i],
                "Adam params[{}] should decrease: {} -> {}",
                i,
                initial_params[i],
                params[i]
            );
        }

        // First moment (m) should be updated (moving average of gradients)
        assert!(
            m.iter().all(|&val| val > 0.0),
            "First moment (m) should be positive"
        );

        // Second moment (v) should be updated (moving average of squared gradients)
        assert!(
            v.iter().all(|&val| val > 0.0),
            "Second moment (v) should be positive"
        );

        // Verify bias correction is working (step 1)
        // m after step 1: m = beta1 * 0 + (1 - beta1) * grad = 0.1 * grad
        // Expected m[0] ≈ 0.1 * 0.1 = 0.01
        assert!(
            (m[0] - 0.01).abs() < FP32_TOLERANCE_RELAXED,
            "First moment should be ≈ 0.01, got {}",
            m[0]
        );

        println!("✅ Adam optimizer precision test passed");
    })
    .await;
}

// Other Optimizers (Already Tested)

#[tokio::test]
async fn test_sgd_momentum_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let mut params = vec![1.0, 2.0, 3.0, 4.0];
        let grads = vec![0.1, 0.2, 0.3, 0.4];
        let mut velocity = vec![0.0; 4];

        let config = SgdConfig {
            learning_rate: 0.01,
            momentum: 0.9,
            dampening: 0.0,
            weight_decay: 0.0,
        };

        let initial_params = params.clone();

        executor
            .execute_sgd(&mut params, &grads, &mut velocity, config)
            .await
            .unwrap();

        // Parameters should decrease
        for i in 0..4 {
            assert!(params[i] <= initial_params[i], "Params should decrease");
        }

        // Velocity should be non-zero after first step
        assert!(
            velocity.iter().all(|&v| v > 0.0),
            "Velocity should be positive"
        );

        println!("✅ SGD with momentum precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_rmsprop_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let mut params = vec![1.0, 2.0, 3.0, 4.0];
        let grads = vec![0.1, 0.2, 0.3, 0.4];
        let mut square_avg = vec![0.0; 4];

        let config = RmspropConfig {
            learning_rate: 0.01,
            alpha: 0.99,
            epsilon: 1e-8,
            weight_decay: 0.0,
        };

        let initial_params = params.clone();

        executor
            .execute_rmsprop(&mut params, &grads, &mut square_avg, config)
            .await
            .unwrap();

        // RMSprop should decrease parameters
        for i in 0..4 {
            assert!(
                params[i] < initial_params[i],
                "Params should decrease with positive gradients"
            );
        }

        // Square average should be updated
        assert!(
            square_avg.iter().all(|&sa| sa > 0.0),
            "Square avg should be positive"
        );

        println!("✅ RMSprop precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_adagrad_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let mut params = vec![1.0, 2.0, 3.0, 4.0];
        let grads = vec![0.1, 0.2, 0.3, 0.4];
        let mut sum_squares = vec![0.0; 4];

        let config = AdagradConfig {
            learning_rate: 0.01,
            epsilon: 1e-10,
            weight_decay: 0.0,
        };

        let initial_params = params.clone();

        executor
            .execute_adagrad(&mut params, &grads, &mut sum_squares, config)
            .await
            .unwrap();

        // AdaGrad should decrease parameters
        for i in 0..4 {
            assert!(params[i] < initial_params[i], "Params should decrease");
        }

        // Sum of squares should accumulate
        for i in 0..4 {
            let expected_sum_sq = grads[i] * grads[i];
            assert!(
                (sum_squares[i] - expected_sum_sq).abs() < FP32_TOLERANCE_RELAXED,
                "Sum squares should match grad^2"
            );
        }

        println!("✅ AdaGrad precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_nadam_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let mut params = vec![1.0, 2.0, 3.0, 4.0];
        let grads = vec![0.1, 0.2, 0.3, 0.4];
        let mut m = vec![0.0; 4];
        let mut v = vec![0.0; 4];

        let config = NadamConfig {
            learning_rate: 0.001,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
            weight_decay: 0.0,
        };

        let initial_params = params.clone();

        executor
            .execute_nadam(&mut params, &grads, &mut m, &mut v, 1, config)
            .await
            .unwrap();

        // NAdam should decrease parameters
        for i in 0..4 {
            assert!(params[i] < initial_params[i], "Params should decrease");
        }

        // Moments should be updated
        assert!(
            m.iter().all(|&val| val > 0.0),
            "First moment should be positive"
        );
        assert!(
            v.iter().all(|&val| val > 0.0),
            "Second moment should be positive"
        );

        println!("✅ NAdam precision test passed");
    })
    .await;
}

#[tokio::test]
async fn test_adadelta_precision() {
    gpu_test_resilient_async(async {
        let executor = WgpuExecutor::new().await.unwrap();

        let mut params = vec![1.0, 2.0, 3.0, 4.0];
        let grads = vec![0.1, 0.2, 0.3, 0.4];
        let mut square_avg = vec![0.0; 4];
        let mut delta_square_avg = vec![0.0; 4];

        let config = AdadeltaConfig {
            rho: 0.9,
            epsilon: 1e-6,
            weight_decay: 0.0,
        };

        let initial_params = params.clone();

        executor
            .execute_adadelta(
                &mut params,
                &grads,
                &mut square_avg,
                &mut delta_square_avg,
                config,
            )
            .await
            .unwrap();

        // AdaDelta should update parameters
        for i in 0..4 {
            assert!(params[i] != initial_params[i], "Params should change");
            assert!(params[i].is_finite(), "Result should be finite");
        }

        // Square averages should be updated
        assert!(
            square_avg.iter().all(|&sa| sa > 0.0),
            "Square avg should be positive"
        );

        println!("✅ AdaDelta precision test passed");
    })
    .await;
}

// ============================================================================
// LOSS FUNCTIONS (7 total)
// ============================================================================

// Core Loss Function (Untested - HIGH PRIORITY)
