//! INTT (Inverse Number Theoretic Transform) unit tests.

use super::helpers::*;
use barracuda::device::WgpuDevice;
use barracuda::ops::fhe_intt::{compute_inverse_root, FheIntt};
use barracuda::ops::fhe_ntt::FheNtt;
use barracuda::ops::fhe_poly_add::create_fhe_poly_tensor;
use std::sync::Arc;

#[tokio::test]
async fn test_intt_basic() {
    if !crate::common::run_gpu_resilient_async(|| async {
        // INTT should be inverse of NTT

        let degree = 16u32;
        let modulus = 12289u64;
        let root = find_root_of_unity(degree, modulus).expect("Should find root");
        let inv_root = compute_inverse_root(degree, modulus, root);
        let input = random_polynomial(degree as usize, modulus);

        let device = Arc::new(
            WgpuDevice::new()
                .await
                .expect("Failed to create GPU device"),
        );
        let input_tensor = create_fhe_poly_tensor(&input, device.clone())
            .await
            .unwrap();

        // Forward NTT
        let ntt = FheNtt::new(input_tensor.clone(), degree, modulus, root).unwrap();
        let ntt_result_tensor = ntt.execute().unwrap();

        // Inverse NTT
        let intt = FheIntt::new(ntt_result_tensor, degree, modulus, inv_root).unwrap();
        let intt_result_tensor = intt.execute().unwrap();
        let intt_result = read_poly_from_tensor(&intt_result_tensor).await;

        // Scale by N to compare with original
        let scaled_result: Vec<u64> = intt_result
            .iter()
            .map(|&x| (x as u128 * degree as u128 % modulus as u128) as u64)
            .collect();

        assert_eq!(input.len(), scaled_result.len());
        for (i, (&orig, &recovered)) in input.iter().zip(scaled_result.iter()).enumerate() {
            assert_eq!(
                orig, recovered,
                "INTT should recover original coefficient {}",
                i
            );
        }

        println!("✅ INTT basic test passed");
    }) {
        return;
    }
}

#[tokio::test]
async fn test_intt_scaling() {
    if !crate::common::run_gpu_resilient_async(|| async {
        // INTT must scale by N^(-1) mod q

        let degree = 8u32;
        let modulus = 12289u64;
        let root = find_root_of_unity(degree, modulus).expect("Should find root");
        let inv_root = compute_inverse_root(degree, modulus, root);

        // Create a polynomial with all coefficients = 1
        let input = vec![1u64; degree as usize];

        let device = Arc::new(
            WgpuDevice::new()
                .await
                .expect("Failed to create GPU device"),
        );
        let input_tensor = create_fhe_poly_tensor(&input, device.clone())
            .await
            .unwrap();

        // Forward NTT
        let ntt = FheNtt::new(input_tensor, degree, modulus, root).unwrap();
        let ntt_result_tensor = ntt.execute().unwrap();

        // Inverse NTT (should scale by N^(-1))
        let intt = FheIntt::new(ntt_result_tensor, degree, modulus, inv_root).unwrap();
        let intt_result_tensor = intt.execute().unwrap();
        let intt_result = read_poly_from_tensor(&intt_result_tensor).await;

        // Verify scaling: result should be scaled by N^(-1)
        // If input is all 1s, NTT result should be sum, and INTT should scale it back
        let n_inv = mod_inverse(degree as u64, modulus);
        let expected_scaled = (n_inv as u128 % modulus as u128) as u64;

        // All coefficients should be scaled by N^(-1)
        for &coeff in &intt_result {
            assert_eq!(coeff, expected_scaled, "INTT should scale by N^(-1)");
        }

        println!("✅ INTT scaling verified");
    }) {
        return;
    }
}
