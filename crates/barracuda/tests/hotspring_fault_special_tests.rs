//! hotSpring Evolution: Fault (error handling) and Special functions (Hermite, Laguerre).

use barracuda::error::BarracudaError;
use barracuda::ops::grid::Gradient1D;
use barracuda::ops::mixing::{LinearMixer, MixingParams};

mod fault {
    use super::*;

    #[tokio::test]
    async fn test_mixer_dimension_mismatch_old() {
        let device = barracuda::device::test_pool::get_test_device().await;
        let params = MixingParams::default();
        let mixer = LinearMixer::new(device, 100, params).unwrap();
        let x_old = vec![1.0; 50];
        let x_computed = vec![2.0; 100];
        let result = mixer.mix(&x_old, &x_computed).await;
        assert!(result.is_err(), "Should fail on dimension mismatch");
        match result.unwrap_err() {
            BarracudaError::InvalidInput { message } => {
                assert!(message.contains("mismatch") || message.contains("dimension"));
            }
            other => panic!("Expected InvalidInput error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_mixer_dimension_mismatch_computed() {
        let device = barracuda::device::test_pool::get_test_device().await;
        let params = MixingParams::default();
        let mixer = LinearMixer::new(device, 100, params).unwrap();
        let x_old = vec![1.0; 100];
        let x_computed = vec![2.0; 200];
        let result = mixer.mix(&x_old, &x_computed).await;
        assert!(result.is_err(), "Should fail on dimension mismatch");
    }

    #[tokio::test]
    async fn test_gradient_size_mismatch() {
        let device = barracuda::device::test_pool::get_test_device().await;
        let grad = Gradient1D::new(device, 100, 0.1).unwrap();
        let input = vec![1.0; 50];
        let result = grad.compute(&input).await;
        assert!(result.is_err(), "Should fail on size mismatch");
        match result.unwrap_err() {
            BarracudaError::InvalidInput { message } => {
                assert!(message.contains("mismatch") || message.contains("size"));
            }
            other => panic!("Expected InvalidInput error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_mixer_zero_dimension() {
        let device = barracuda::device::test_pool::get_test_device().await;
        let params = MixingParams::default();
        let mixer = LinearMixer::new(device, 0, params);
        if let Ok(m) = mixer {
            let result = m.mix(&[], &[]).await;
            if let Ok(output) = result {
                assert!(output.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_gradient_empty_input() {
        let device = barracuda::device::test_pool::get_test_device().await;
        let grad = Gradient1D::new(device, 0, 0.1);
        if let Ok(g) = grad {
            let result = g.compute(&[]).await;
            if let Ok(output) = result {
                assert!(output.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_mixer_nan_propagation() {
        let device = barracuda::device::test_pool::get_test_device().await;
        let params = MixingParams {
            alpha: 0.5,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 10, params).unwrap();
        let x_old = vec![1.0; 10];
        let mut x_computed = vec![2.0; 10];
        x_computed[5] = f64::NAN;
        let result = mixer.mix(&x_old, &x_computed).await.unwrap();
        assert!(result[5].is_nan(), "NaN should propagate through mixing");
        for i in [0, 1, 2, 3, 4, 6, 7, 8, 9] {
            assert!(
                !result[i].is_nan(),
                "Non-NaN input should produce non-NaN output"
            );
        }
    }

    #[tokio::test]
    async fn test_mixer_infinity() {
        let device = barracuda::device::test_pool::get_test_device().await;
        let params = MixingParams {
            alpha: 0.5,
            ..Default::default()
        };
        let mixer = LinearMixer::new(device, 5, params).unwrap();
        let x_old = vec![1.0, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0];
        let x_computed = vec![2.0, 3.0, 4.0, f64::INFINITY, f64::NEG_INFINITY];
        let result = mixer.mix(&x_old, &x_computed).await.unwrap();
        assert!(result[1].is_infinite(), "Infinity should propagate");
        assert!(result[2].is_infinite(), "Neg infinity should propagate");
        assert!(result[3].is_infinite(), "Infinity should propagate");
        assert!(result[4].is_infinite(), "Neg infinity should propagate");
    }

    #[tokio::test]
    async fn test_gradient_nan_handling() {
        let device = barracuda::device::test_pool::get_test_device().await;
        let grad = Gradient1D::new(device, 20, 0.1).unwrap();
        let mut input: Vec<f64> = (0..20).map(|i| i as f64).collect();
        input[10] = f64::NAN;
        let result = grad.compute(&input).await.unwrap();
        assert!(result[9].is_nan() || result[10].is_nan() || result[11].is_nan());
    }
}

mod special_functions {
    fn hermite_cpu(n: u32, x: f64) -> f64 {
        if n == 0 {
            return 1.0;
        }
        if n == 1 {
            return 2.0 * x;
        }
        let mut h_prev = 1.0;
        let mut h_curr = 2.0 * x;
        for k in 1..n {
            let h_next = 2.0 * x * h_curr - 2.0 * k as f64 * h_prev;
            h_prev = h_curr;
            h_curr = h_next;
        }
        h_curr
    }

    fn laguerre_cpu(n: u32, alpha: f64, x: f64) -> f64 {
        if n == 0 {
            return 1.0;
        }
        if n == 1 {
            return 1.0 + alpha - x;
        }
        let mut l_prev = 1.0;
        let mut l_curr = 1.0 + alpha - x;
        for k in 1..n {
            let kf = k as f64;
            let l_next =
                ((2.0 * kf + 1.0 + alpha - x) * l_curr - (kf + alpha) * l_prev) / (kf + 1.0);
            l_prev = l_curr;
            l_curr = l_next;
        }
        l_curr
    }

    #[test]
    fn test_hermite_n0() {
        for x in [-2.0_f64, -1.0, 0.0, 0.5, 1.0, 2.0, 2.72] {
            assert!((hermite_cpu(0, x) - 1.0).abs() < 1e-15);
        }
    }

    #[test]
    fn test_hermite_n1() {
        for x in [-2.0_f64, -1.0, 0.0, 0.5, 1.0, 2.0] {
            let expected = 2.0 * x;
            assert!((hermite_cpu(1, x) - expected).abs() < 1e-14);
        }
    }

    #[test]
    fn test_hermite_n2() {
        for x in [-2.0_f64, -1.0, 0.0, 0.5, 1.0, 2.0] {
            let expected = 4.0 * x * x - 2.0;
            assert!(
                (hermite_cpu(2, x) - expected).abs() < 1e-13,
                "H_2({}) = {} vs expected {}",
                x,
                hermite_cpu(2, x),
                expected
            );
        }
    }

    #[test]
    fn test_hermite_n3() {
        for x in [-2.0_f64, -1.0, 0.0, 0.5, 1.0, 2.0] {
            let expected = 8.0 * x.powi(3) - 12.0 * x;
            assert!(
                (hermite_cpu(3, x) - expected).abs() < 1e-12,
                "H_3({}) = {} vs expected {}",
                x,
                hermite_cpu(3, x),
                expected
            );
        }
    }

    #[test]
    fn test_hermite_at_zero() {
        assert!((hermite_cpu(0, 0.0) - 1.0).abs() < 1e-15);
        assert!((hermite_cpu(1, 0.0) - 0.0).abs() < 1e-15);
        assert!((hermite_cpu(2, 0.0) - (-2.0)).abs() < 1e-14);
        assert!((hermite_cpu(3, 0.0) - 0.0).abs() < 1e-14);
        assert!((hermite_cpu(4, 0.0) - 12.0).abs() < 1e-13);
    }

    #[test]
    fn test_laguerre_n0() {
        for x in [0.0_f64, 0.5, 1.0, 2.0, 5.0] {
            assert!((laguerre_cpu(0, 0.0, x) - 1.0).abs() < 1e-15);
        }
    }

    #[test]
    fn test_laguerre_n1() {
        for x in [0.0_f64, 0.5, 1.0, 2.0, 5.0] {
            let expected = 1.0 - x;
            assert!(
                (laguerre_cpu(1, 0.0, x) - expected).abs() < 1e-14,
                "L_1({}) = {} vs expected {}",
                x,
                laguerre_cpu(1, 0.0, x),
                expected
            );
        }
    }

    #[test]
    fn test_laguerre_n2() {
        for x in [0.0_f64, 0.5, 1.0, 2.0, 5.0] {
            let expected = (x * x - 4.0 * x + 2.0) / 2.0;
            assert!(
                (laguerre_cpu(2, 0.0, x) - expected).abs() < 1e-13,
                "L_2({}) = {} vs expected {}",
                x,
                laguerre_cpu(2, 0.0, x),
                expected
            );
        }
    }

    #[test]
    fn test_laguerre_alpha1() {
        assert!((laguerre_cpu(0, 1.0, 2.0) - 1.0).abs() < 1e-15);
        for x in [0.0_f64, 1.0, 2.0] {
            let expected = 2.0 - x;
            assert!(
                (laguerre_cpu(1, 1.0, x) - expected).abs() < 1e-14,
                "L_1^(1)({}) = {} vs expected {}",
                x,
                laguerre_cpu(1, 1.0, x),
                expected
            );
        }
    }

    #[test]
    fn test_laguerre_at_zero() {
        for n in 0..10 {
            assert!(
                (laguerre_cpu(n, 0.0, 0.0) - 1.0).abs() < 1e-10,
                "L_{}(0) should be 1, got {}",
                n,
                laguerre_cpu(n, 0.0, 0.0)
            );
        }
    }

    #[test]
    fn test_hermite_high_order() {
        let h10 = hermite_cpu(10, 1.0);
        assert!(h10.is_finite(), "H_10(1) should be finite");
        assert!(h10.abs() > 1.0, "H_10(1) should be non-trivial");
        let h20 = hermite_cpu(20, 0.5);
        assert!(h20.is_finite(), "H_20(0.5) should be finite");
    }

    #[test]
    fn test_laguerre_high_order() {
        let l10 = laguerre_cpu(10, 0.0, 1.0);
        assert!(l10.is_finite(), "L_10(1) should be finite");
        let l20 = laguerre_cpu(20, 2.0, 2.0);
        assert!(l20.is_finite(), "L_20^(2)(2) should be finite");
    }
}
