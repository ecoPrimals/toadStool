//! Tests for Transpose operation

use crate::tensor::Tensor;
use std::sync::Arc;

#[tokio::test]
async fn test_transpose_basic() {
    let device = crate::device::Auto::new().await.unwrap();
    let device = Arc::new(device);

    // Test data: 2x3 matrix [[1,2,3], [4,5,6]]
    let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3], device)
        .await
        .unwrap();

    let output = input.transpose().unwrap();
    let result = output.to_vec().unwrap();

    // Expected: 3x2 matrix [[1,4], [2,5], [3,6]]
    let expected = vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0];
    assert_eq!(output.shape(), &[3, 2]);
    for (i, (&r, &e)) in result.iter().zip(expected.iter()).enumerate() {
        assert!(
            (r - e).abs() < 1e-5,
            "Mismatch at index {}: {} vs {}",
            i,
            r,
            e
        );
    }
}

#[tokio::test]
async fn test_transpose_nd() {
    let device = crate::device::Auto::new().await.unwrap();
    let device = Arc::new(device);

    // Test 3D transpose: [B, C, H] -> [B, H, C]
    let input = Tensor::from_vec_on(
        (0..24).map(|i| i as f32).collect(),
        vec![2, 3, 4],
        device.clone(),
    )
    .await
    .unwrap();

    let output = input.transpose_with_permutation(vec![0, 2, 1]).unwrap();
    assert_eq!(output.shape(), &[2, 4, 3]);
}
