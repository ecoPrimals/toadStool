//! OpenCL backend tests

use super::*;

#[test]
fn test_opencl_device_discovery() {
    // Should discover devices without assumptions
    match OpenClBackend::new() {
        Ok(backend) => {
            println!("✅ Discovered: {}", backend.device_info.name);
            assert!(!backend.device_info.name.is_empty());
        }
        Err(e) => {
            println!("⚠️  No OpenCL devices: {}", e);
            // Not a failure - just no GPU available
        }
    }
}

#[tokio::test]
async fn test_capability_discovery() {
    if let Ok(backend) = OpenClBackend::new() {
        let caps = backend.capabilities();
        println!("Device capabilities:");
        println!(
            "  Parallel threads: {}",
            caps.parallelism.max_parallel_threads
        );
        println!(
            "  Memory: {} GB",
            caps.memory.total_bytes / (1024 * 1024 * 1024)
        );
        println!("  FP64 support: {}", caps.precision.fp64);

        assert!(caps.parallelism.max_parallel_threads > 0);
        assert!(caps.memory.total_bytes > 0);
    }
}
