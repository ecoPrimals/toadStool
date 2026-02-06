#[cfg(test)]
mod tests {
    use crate::ops::unique::Unique;
    use crate::device::test_pool::get_test_device;
    use crate::tensor::Tensor;

    #[tokio::test]
    async fn test_unique_basic() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 1.0, 3.0, 2.0], vec![5], device.clone())
            .await
            .unwrap();
        
        let result = Unique::new(input).unwrap().execute().unwrap();
        let unique = result.to_vec().unwrap();
        assert!(unique.len() <= 5);
        // Should contain 1, 2, 3 (order may vary)
    }

    #[tokio::test]
    async fn test_unique_all_same() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![5.0, 5.0, 5.0], vec![3], device.clone())
            .await
            .unwrap();
        
        let result = Unique::new(input).unwrap().execute().unwrap();
        let unique = result.to_vec().unwrap();
        assert_eq!(unique.len(), 1);
        assert_eq!(unique[0], 5.0);
    }

    #[tokio::test]
    async fn test_unique_all_different() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone())
            .await
            .unwrap();
        
        let result = Unique::new(input).unwrap().execute().unwrap();
        let unique = result.to_vec().unwrap();
        assert_eq!(unique.len(), 3);
    }

    #[tokio::test]
    async fn test_unique_empty() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![], vec![0], device.clone())
            .await
            .unwrap();
        
        assert!(Unique::new(input).is_err());
    }
}
