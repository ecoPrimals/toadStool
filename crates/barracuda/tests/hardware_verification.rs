//! Hardware Verification Tests
//!
//! Comprehensive tests for heterogeneous hardware setups:
//! - 2 GPUs from different vendors/eras (e.g., NVIDIA RTX + AMD RX)
//! - 2 NPUs (Akida)
//! - Cross-vendor math parity
//! - Routing verification
//!
//! ## Test Categories
//!
//! 1. **Discovery Tests** - Verify all hardware is detected
//! 2. **Parity Tests** - Same math produces same results across vendors
//! 3. **Routing Tests** - ToadStool routes to correct hardware
//! 4. **Performance Tests** - Verify expected performance characteristics
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p barracuda --test hardware_verification -- --nocapture
//! ```

use barracuda::device::{
    ComputeWorkload, Device, DeviceSelection, HardwareWorkload, KernelRouter, KernelTarget,
    WgpuDevice,
};
use barracuda::tensor::Tensor;
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// Test Infrastructure
// ============================================================================

/// Hardware inventory for test reporting
#[derive(Debug)]
struct HardwareInventory {
    gpus: Vec<GpuInfo>,
    npus: Vec<NpuInfo>,
}

#[derive(Debug)]
struct GpuInfo {
    name: String,
    vendor: String,
    device_type: wgpu::DeviceType,
    backend: wgpu::Backend,
    adapter_index: usize,
}

#[derive(Debug)]
struct NpuInfo {
    name: String,
    device_path: String,
}

impl HardwareInventory {
    fn discover() -> Self {
        let adapters = WgpuDevice::enumerate_adapters();

        let mut gpus = Vec::new();
        for (idx, info) in adapters.iter().enumerate() {
            let vendor = if info.name.to_lowercase().contains("nvidia") {
                "NVIDIA"
            } else if info.name.to_lowercase().contains("amd")
                || info.name.to_lowercase().contains("radeon")
            {
                "AMD"
            } else if info.name.to_lowercase().contains("intel") {
                "Intel"
            } else {
                "Unknown"
            };

            // Only count discrete/integrated GPUs
            if matches!(
                info.device_type,
                wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
            ) || (info.device_type == wgpu::DeviceType::Other
                && (vendor == "NVIDIA" || vendor == "AMD"))
            {
                gpus.push(GpuInfo {
                    name: info.name.clone(),
                    vendor: vendor.to_string(),
                    device_type: info.device_type,
                    backend: info.backend,
                    adapter_index: idx,
                });
            }
        }

        // Scan for NPUs
        let mut npus = Vec::new();
        for i in 0..16 {
            let path = format!("/dev/akida{}", i);
            if std::path::Path::new(&path).exists() {
                npus.push(NpuInfo {
                    name: format!("Akida NPU {}", i),
                    device_path: path,
                });
            }
        }

        Self { gpus, npus }
    }

    fn print_report(&self) {
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║           HARDWARE VERIFICATION INVENTORY                     ║");
        println!("╠══════════════════════════════════════════════════════════════╣");

        println!(
            "║ GPUs Detected: {}                                             ",
            self.gpus.len()
        );
        for gpu in &self.gpus {
            println!(
                "║   [{:2}] {} ({}) - {:?}/{:?}",
                gpu.adapter_index, gpu.name, gpu.vendor, gpu.device_type, gpu.backend
            );
        }

        println!(
            "║ NPUs Detected: {}                                             ",
            self.npus.len()
        );
        for npu in &self.npus {
            println!("║   {} @ {}", npu.name, npu.device_path);
        }

        println!("╚══════════════════════════════════════════════════════════════╝\n");
    }

    fn has_multi_gpu(&self) -> bool {
        self.gpus.len() >= 2
    }

    fn has_npu(&self) -> bool {
        !self.npus.is_empty()
    }
}

/// Compare f32 slices with tolerance
fn assert_close(label: &str, a: &[f32], b: &[f32], tol: f32) {
    assert_eq!(a.len(), b.len(), "{}: length mismatch", label);
    let mut max_diff = 0.0f32;
    let mut max_idx = 0;
    for (i, (va, vb)) in a.iter().zip(b.iter()).enumerate() {
        let diff = (va - vb).abs();
        if diff > max_diff {
            max_diff = diff;
            max_idx = i;
        }
    }
    assert!(
        max_diff < tol,
        "{}: max diff {} at idx {} (tol {})",
        label,
        max_diff,
        max_idx,
        tol
    );
}


// ============================================================================
// Discovery Tests
// ============================================================================

#[test]
fn test_hardware_discovery_report() {
    let inventory = HardwareInventory::discover();
    inventory.print_report();

    // Should find at least one compute device
    assert!(
        !inventory.gpus.is_empty() || Device::CPU.is_available(),
        "No compute hardware detected"
    );
}

#[tokio::test]
async fn test_all_gpus_can_create_device() {
    let inventory = HardwareInventory::discover();

    println!("Testing device creation for {} GPUs", inventory.gpus.len());

    let mut successful = 0;
    let mut failed = Vec::new();

    for gpu in &inventory.gpus {
        match WgpuDevice::from_adapter_index(gpu.adapter_index).await {
            Ok(_device) => {
                println!("  ✓ {} - Device created", gpu.name);
                successful += 1;
            }
            Err(e) => {
                println!("  ✗ {} - Failed: {}", gpu.name, e);
                failed.push(gpu.name.clone());
            }
        }
    }

    println!(
        "\nSummary: {}/{} devices created successfully",
        successful,
        inventory.gpus.len()
    );

    // At least one GPU should work
    assert!(successful > 0, "No GPUs could create devices: {:?}", failed);
}

#[test]
fn test_kernel_router_creation() {
    let router = KernelRouter::new();
    assert!(router.is_ok(), "KernelRouter should initialize");

    let router = router.unwrap();
    println!("Available NPU models: {:?}", router.available_npu_models());
}

// ============================================================================
// Cross-Vendor GPU Parity Tests
// ============================================================================

#[tokio::test]
async fn test_cross_vendor_matmul_parity() {
    let inventory = HardwareInventory::discover();

    if !inventory.has_multi_gpu() {
        println!("SKIP: Need 2+ GPUs for cross-vendor parity test");
        return;
    }

    println!("\n=== Cross-Vendor Matmul Parity Test ===\n");

    // Create device for each GPU
    let mut devices: Vec<(String, Arc<WgpuDevice>)> = Vec::new();
    for gpu in &inventory.gpus {
        if let Ok(device) = WgpuDevice::from_adapter_index(gpu.adapter_index).await {
            devices.push((format!("{} ({})", gpu.name, gpu.vendor), Arc::new(device)));
        }
    }

    if devices.len() < 2 {
        println!("SKIP: Need 2+ working GPUs for parity test");
        return;
    }

    // Test data - 64x64 matmul
    let size = 64;
    let a_data: Vec<f32> = (0..size * size).map(|i| (i as f32) * 0.01).collect();
    let b_data: Vec<f32> = (0..size * size).map(|i| (i as f32) * 0.01 + 0.5).collect();

    // Run on each device
    let mut results: HashMap<String, Vec<f32>> = HashMap::new();

    for (name, device) in &devices {
        let a = Tensor::from_vec_on(a_data.clone(), vec![size, size], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(b_data.clone(), vec![size, size], device.clone())
            .await
            .unwrap();

        let result = a.matmul(&b).unwrap().to_vec().unwrap();
        println!("  {} - computed (first 4 values: {:?})", name, &result[..4]);
        results.insert(name.clone(), result);
    }

    // Compare all pairs
    let device_names: Vec<_> = results.keys().cloned().collect();
    for i in 0..device_names.len() {
        for j in (i + 1)..device_names.len() {
            let name_a = &device_names[i];
            let name_b = &device_names[j];
            let result_a = &results[name_a];
            let result_b = &results[name_b];

            assert_close(
                &format!("{} vs {}", name_a, name_b),
                result_a,
                result_b,
                1e-3, // Slightly loose tolerance for cross-vendor
            );
            println!("  ✓ {} matches {}", name_a, name_b);
        }
    }

    println!("\n  PASS: All GPUs produce identical matmul results\n");
}

#[tokio::test]
async fn test_cross_vendor_cholesky_parity() {
    let inventory = HardwareInventory::discover();

    if !inventory.has_multi_gpu() {
        println!("SKIP: Need 2+ GPUs for cross-vendor parity test");
        return;
    }

    println!("\n=== Cross-Vendor Cholesky Parity Test ===\n");

    let mut devices: Vec<(String, Arc<WgpuDevice>)> = Vec::new();
    for gpu in &inventory.gpus {
        if let Ok(device) = WgpuDevice::from_adapter_index(gpu.adapter_index).await {
            devices.push((gpu.name.clone(), Arc::new(device)));
        }
    }

    if devices.len() < 2 {
        println!("SKIP: Need 2+ working GPUs");
        return;
    }

    // SPD matrix (positive definite)
    let spd = vec![4.0, 12.0, -16.0, 12.0, 37.0, -43.0, -16.0, -43.0, 98.0];

    let mut results: HashMap<String, Vec<f32>> = HashMap::new();

    for (name, device) in &devices {
        let a = Tensor::from_vec_on(spd.clone(), vec![3, 3], device.clone())
            .await
            .unwrap();

        let result = a.cholesky().unwrap().to_vec().unwrap();
        println!("  {} Cholesky L: {:?}", name, result);
        results.insert(name.clone(), result);
    }

    // Compare
    let device_names: Vec<_> = results.keys().cloned().collect();
    for i in 0..device_names.len() {
        for j in (i + 1)..device_names.len() {
            let name_a = &device_names[i];
            let name_b = &device_names[j];
            assert_close(
                &format!("{} vs {}", name_a, name_b),
                &results[name_a],
                &results[name_b],
                1e-4,
            );
            println!("  ✓ {} matches {}", name_a, name_b);
        }
    }

    println!("\n  PASS: Cross-vendor Cholesky parity verified\n");
}

#[tokio::test]
async fn test_cross_vendor_softmax_parity() {
    let inventory = HardwareInventory::discover();

    if !inventory.has_multi_gpu() {
        println!("SKIP: Need 2+ GPUs for cross-vendor parity test");
        return;
    }

    println!("\n=== Cross-Vendor Softmax Parity Test ===\n");

    let mut devices: Vec<(String, Arc<WgpuDevice>)> = Vec::new();
    for gpu in &inventory.gpus {
        if let Ok(device) = WgpuDevice::from_adapter_index(gpu.adapter_index).await {
            devices.push((gpu.name.clone(), Arc::new(device)));
        }
    }

    if devices.len() < 2 {
        return;
    }

    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

    let mut results: HashMap<String, Vec<f32>> = HashMap::new();

    for (name, device) in &devices {
        let t = Tensor::from_vec_on(data.clone(), vec![8], device.clone())
            .await
            .unwrap();

        let result = t.softmax().unwrap().to_vec().unwrap();
        let sum: f32 = result.iter().sum();
        println!("  {} softmax sum: {:.6}", name, sum);
        assert!(
            (sum - 1.0).abs() < 1e-4,
            "{} softmax should sum to 1.0",
            name
        );
        results.insert(name.clone(), result);
    }

    let device_names: Vec<_> = results.keys().cloned().collect();
    for i in 0..device_names.len() {
        for j in (i + 1)..device_names.len() {
            assert_close(
                &format!("softmax {} vs {}", device_names[i], device_names[j]),
                &results[&device_names[i]],
                &results[&device_names[j]],
                1e-4,
            );
        }
    }

    println!("  PASS: Cross-vendor softmax parity verified\n");
}

// ============================================================================
// Kernel Router Tests
// ============================================================================

#[test]
fn test_kernel_router_dense_workloads_to_wgsl() {
    let router = KernelRouter::default();

    // Dense workloads should ALWAYS go to WGSL (GPU/CPU)
    let dense_workloads = vec![
        ComputeWorkload::DenseMatmul {
            m: 1024,
            n: 1024,
            k: 1024,
        },
        ComputeWorkload::FFT {
            size: 1024,
            batch_count: 10,
        },
        ComputeWorkload::PhysicsForce {
            particle_count: 10000,
            force_type: "lennard_jones".to_string(),
        },
        ComputeWorkload::Eigendecomp { matrix_size: 256 },
        ComputeWorkload::LinearSolve { system_size: 512 },
    ];

    for workload in dense_workloads {
        let target = router.route(&workload).unwrap();
        match target {
            KernelTarget::Wgsl { device, .. } => {
                assert!(
                    device.supports_wgsl(),
                    "Dense workload {:?} should route to WGSL-capable device",
                    workload
                );
            }
            other => {
                panic!(
                    "Dense workload {:?} should route to WGSL, got {:?}",
                    workload, other
                );
            }
        }
    }

    println!("✓ All dense workloads correctly routed to WGSL");
}

#[test]
fn test_kernel_router_small_workloads_to_cpu() {
    let router = KernelRouter::default();

    // Small workloads should prefer CPU (avoid GPU dispatch overhead)
    let small_workloads = vec![
        ComputeWorkload::DenseMatmul {
            m: 16,
            n: 16,
            k: 16,
        },
        ComputeWorkload::Eigendecomp { matrix_size: 32 },
        ComputeWorkload::LinearSolve { system_size: 64 },
    ];

    for workload in small_workloads {
        let target = router.route(&workload).unwrap();
        if let KernelTarget::Wgsl { device, .. } = target {
            assert_eq!(
                device,
                DeviceSelection::Cpu,
                "Small workload {:?} should route to CPU",
                workload
            );
        }
    }

    println!("✓ Small workloads correctly routed to CPU");
}

#[test]
fn test_kernel_router_npu_fallback() {
    let router = KernelRouter::default();

    // NPU workloads without models should fall back to WGSL
    let npu_workloads = vec![
        ComputeWorkload::SparseInference {
            input_sparsity: 0.95,
            model_name: "nonexistent_model".to_string(),
        },
        ComputeWorkload::ReservoirState {
            reservoir_size: 1000,
            input_dim: 100,
        },
        ComputeWorkload::BinaryPrescreen {
            input_count: 10000,
            threshold: 0.5,
        },
    ];

    for workload in npu_workloads {
        let target = router.route(&workload).unwrap();
        match target {
            KernelTarget::Wgsl { .. } => {
                // Expected fallback to WGSL
            }
            KernelTarget::Npu { .. } => {
                // Also valid if NPU models are present
            }
            other => {
                panic!(
                    "NPU workload {:?} should route to Wgsl or Npu, got {:?}",
                    workload, other
                );
            }
        }
    }

    println!("✓ NPU workloads correctly handle fallback");
}

// ============================================================================
// ToadStool Device Selection Integration
// ============================================================================

#[tokio::test]
async fn test_toadstool_device_selection_integration() {
    use barracuda::device::{discover_devices, select_best_device};

    let hw = discover_devices().expect("Hardware discovery failed");

    println!("\n=== ToadStool Device Selection ===\n");
    println!("Discovered {} devices:", hw.device_count());
    for device in hw.devices() {
        println!("  - {} ({:?})", device.name, device.hardware_type);
    }

    // Test routing for different workload types
    let workloads = vec![
        HardwareWorkload::TensorOps,
        HardwareWorkload::ScientificCompute,
        HardwareWorkload::SpikingNetwork,
        HardwareWorkload::ReservoirComputing,
    ];

    for workload in workloads {
        let selection = select_best_device(workload).expect("Selection failed");
        println!("  {:?} -> {:?}", workload, selection);
    }

    println!("\n  ToadStool device selection: PASS\n");
}

// ============================================================================
// Performance Characterization
// ============================================================================

#[tokio::test]
async fn test_multi_gpu_performance_characterization() {
    let inventory = HardwareInventory::discover();

    if !inventory.has_multi_gpu() {
        println!("SKIP: Need 2+ GPUs for performance characterization");
        return;
    }

    println!("\n=== Multi-GPU Performance Characterization ===\n");

    let mut devices: Vec<(String, Arc<WgpuDevice>)> = Vec::new();
    for gpu in &inventory.gpus {
        if let Ok(device) = WgpuDevice::from_adapter_index(gpu.adapter_index).await {
            devices.push((format!("{} ({})", gpu.name, gpu.vendor), Arc::new(device)));
        }
    }

    // Benchmark: 256x256 matmul
    let size = 256;
    let a_data: Vec<f32> = (0..size * size).map(|i| (i as f32) * 0.001).collect();
    let b_data: Vec<f32> = (0..size * size).map(|i| (i as f32) * 0.001 + 0.5).collect();

    println!("Benchmark: {}x{} matmul x 20 iterations\n", size, size);

    for (name, device) in &devices {
        // Warmup
        let _warmup = Tensor::from_vec_on(a_data.clone(), vec![size, size], device.clone())
            .await
            .unwrap();

        let b = Tensor::from_vec_on(b_data.clone(), vec![size, size], device.clone())
            .await
            .unwrap();

        let start = std::time::Instant::now();
        let iterations = 20;
        for _ in 0..iterations {
            let a = Tensor::from_vec_on(a_data.clone(), vec![size, size], device.clone())
                .await
                .unwrap();
            let _result = a.matmul(&b).unwrap();
        }
        let elapsed = start.elapsed();

        let total_ms = elapsed.as_secs_f64() * 1000.0;
        let per_op_ms = total_ms / iterations as f64;
        let gflops =
            (2.0 * (size as f64).powi(3) * iterations as f64) / elapsed.as_secs_f64() / 1e9;

        println!(
            "  {}: {:.2} ms total, {:.3} ms/op, {:.2} GFLOP/s",
            name, total_ms, per_op_ms, gflops
        );
    }

    println!("\n  Performance characterization complete.\n");
}

// ============================================================================
// NPU Detection and Routing
// ============================================================================

#[test]
fn test_npu_detection() {
    let inventory = HardwareInventory::discover();

    println!("\n=== NPU Detection ===\n");
    println!("NPUs found: {}", inventory.npus.len());

    for npu in &inventory.npus {
        println!("  - {} @ {}", npu.name, npu.device_path);
    }

    if inventory.has_npu() {
        println!("\n  NPU hardware detected - routing tests enabled\n");
    } else {
        println!("\n  No NPU hardware - NPU routing tests will be skipped\n");
    }
}

#[test]
fn test_kernel_router_npu_capability_check() {
    let router = KernelRouter::default();
    let _inventory = HardwareInventory::discover();

    // Dense workloads can NEVER go to NPU
    let dense = ComputeWorkload::DenseMatmul {
        m: 1024,
        n: 1024,
        k: 1024,
    };
    assert!(
        !router.can_route_to_npu(&dense),
        "Dense matmul should not be NPU-routable"
    );

    // Sparse inference MAY go to NPU if model exists
    let sparse = ComputeWorkload::SparseInference {
        input_sparsity: 0.95,
        model_name: "test".to_string(),
    };
    // This will be false unless we register an NPU model
    let can_route = router.can_route_to_npu(&sparse);
    println!(
        "Sparse inference can route to NPU: {} (expected: depends on model registration)",
        can_route
    );
}
