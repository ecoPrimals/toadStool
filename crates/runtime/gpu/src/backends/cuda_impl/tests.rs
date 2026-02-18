use super::*;

#[test]
fn test_cuda_device_discovery() {
    match CudaBackend::new() {
        Ok(backend) => {
            println!(
                "✅ Discovered: {} (SM {}.{})",
                backend.device_info.name,
                backend.device_info.compute_capability.0,
                backend.device_info.compute_capability.1
            );
            assert!(!backend.device_info.name.is_empty());
            assert!(backend.device_info.multiprocessor_count > 0);
        }
        Err(e) => {
            println!("⚠️  No CUDA devices: {}", e);
        }
    }
}

#[tokio::test]
async fn test_capability_discovery() {
    if let Ok(backend) = CudaBackend::new() {
        let caps = backend.capabilities();
        println!("Device capabilities:");
        println!(
            "  Compute: SM {}.{}",
            backend.device_info.compute_capability.0, backend.device_info.compute_capability.1
        );
        println!("  SMs: {}", backend.device_info.multiprocessor_count);
        println!(
            "  Memory: {} GB",
            caps.memory.total_bytes / (1024 * 1024 * 1024)
        );
        println!("  Peak TFLOPS: {:.2}", caps.performance.peak_flops / 1e12);

        assert!(caps.parallelism.max_parallel_threads > 0);
        assert!(caps.memory.total_bytes > 0);
        assert!(caps.performance.peak_flops > 0.0);
    }
}
