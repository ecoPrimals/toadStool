//! Tests for Masked Select operation

use super::*;
use crate::device::WgpuDevice;
use std::sync::Arc;

async fn get_test_device() -> Arc<WgpuDevice> {
    Arc::new(WgpuDevice::new().await.unwrap())
}

#[tokio::test]
async fn test_masked_select_basic() {
    let device = get_test_device().await;
    let input = Tensor::from_data(
        &[1.0, 2.0, 3.0, 4.0, 5.0],
        vec![5],
        device.clone(),
    ).unwrap();
    let mask = Tensor::from_data(
        &[1.0, 0.0, 1.0, 0.0, 1.0],
        vec![5],
        device.clone(),
    ).unwrap();
    
    let result = MaskedSelect::new(input, mask).unwrap().execute().unwrap();
    assert_eq!(result.shape(), &vec![3]);
}

#[tokio::test]
async fn test_masked_select_all_true() {
    let device = get_test_device().await;
    let input = Tensor::from_data(
        &[1.0, 2.0, 3.0],
        vec![3],
        device.clone(),
    ).unwrap();
    let mask = Tensor::from_data(
        &[1.0, 1.0, 1.0],
        vec![3],
        device.clone(),
    ).unwrap();
    
    let result = MaskedSelect::new(input, mask).unwrap().execute().unwrap();
    assert_eq!(result.shape(), &vec![3]);
}

#[tokio::test]
async fn test_masked_select_all_false() {
    let device = get_test_device().await;
    let input = Tensor::from_data(
        &[1.0, 2.0, 3.0],
        vec![3],
        device.clone(),
    ).unwrap();
    let mask = Tensor::from_data(
        &[0.0, 0.0, 0.0],
        vec![3],
        device.clone(),
    ).unwrap();
    
    let result = MaskedSelect::new(input, mask).unwrap().execute().unwrap();
    assert_eq!(result.shape(), &vec![0]);
}

#[tokio::test]
async fn test_masked_select_shape_mismatch() {
    let device = get_test_device().await;
    let input = Tensor::from_data(
        &[1.0, 2.0],
        vec![2],
        device.clone(),
    ).unwrap();
    let mask = Tensor::from_data(
        &[1.0, 1.0, 1.0],
        vec![3],
        device.clone(),
    ).unwrap();
    
    assert!(MaskedSelect::new(input, mask).is_err());
}

#[tokio::test]
async fn test_masked_select_large() {
    let device = get_test_device().await;
    let data: Vec<f32> = (0..1000).map(|i| i as f32).collect();
    let input = Tensor::from_data(
        &data,
        vec![1000],
        device.clone(),
    ).unwrap();
    let mask_data: Vec<f32> = (0..1000).map(|i| if i % 2 == 0 { 1.0 } else { 0.0 }).collect();
    let mask = Tensor::from_data(
        &mask_data,
        vec![1000],
        device.clone(),
    ).unwrap();
    
    let result = MaskedSelect::new(input, mask).unwrap().execute().unwrap();
    assert_eq!(result.shape(), &vec![500]);
}
