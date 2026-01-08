//! Vector Addition Showcase
//!
//! Demonstrates the simplest possible GPU workload across multiple backends:
//! - OpenCL (vendor-agnostic)
//! - Vulkan (vendor-agnostic)
//! - CUDA (NVIDIA-only, for comparison)
//!
//! This is the "Hello World" of GPU computing, perfect for:
//! - Benchmarking overhead
//! - Comparing backends
//! - Testing ZLUDA/SCALE compatibility

use anyhow::Result;

/// Vector addition result with timing information
#[derive(Debug, Clone)]
pub struct VectorAddResult {
    pub backend: String,
    pub size: usize,
    pub compute_time_us: f64,
    pub total_time_us: f64,
    pub throughput_gb_s: f64,
    pub correct: bool,
}

impl VectorAddResult {
    pub fn display(&self) {
        println!("  Backend:    {}", self.backend);
        println!("  Size:       {} elements", self.size);
        println!("  Compute:    {:.3} μs", self.compute_time_us);
        println!("  Total:      {:.3} μs", self.total_time_us);
        println!("  Throughput: {:.2} GB/s", self.throughput_gb_s);
        println!("  Correct:    {}", if self.correct { "✅" } else { "❌" });
    }
}

/// CPU reference implementation
pub fn vector_add_cpu(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
}

/// Verify GPU result against CPU reference
pub fn verify_result(gpu_result: &[f32], cpu_result: &[f32], tolerance: f32) -> bool {
    if gpu_result.len() != cpu_result.len() {
        return false;
    }
    
    gpu_result.iter()
        .zip(cpu_result.iter())
        .all(|(g, c)| (g - c).abs() < tolerance)
}

/// OpenCL implementation
#[cfg(feature = "opencl")]
pub mod opencl {
    use super::*;
    use anyhow::Context;
    use ocl::{Buffer, Context as OclContext, Device, Kernel, Platform, Program, Queue};
    use std::time::Instant;

    const OPENCL_KERNEL: &str = r#"
__kernel void vector_add(
    __global const float* a,
    __global const float* b,
    __global float* c,
    const int n
) {
    int i = get_global_id(0);
    if (i < n) {
        c[i] = a[i] + b[i];
    }
}
"#;

    pub fn vector_add_opencl(a: &[f32], b: &[f32]) -> Result<VectorAddResult> {
        let size = a.len();
        let start_total = Instant::now();

        // Setup OpenCL - find a platform with GPU devices
        let platforms = Platform::list();
        let mut selected_device = None;
        let mut selected_platform = None;
        
        for platform in platforms {
            if let Ok(devices) = Device::list_all(platform) {
                for device in devices {
                    // Check if it's a GPU device
                    if let Ok(device_type) = device.info(ocl::core::DeviceInfo::Type) {
                        use ocl::core::{DeviceInfoResult, DeviceType};
                        if let DeviceInfoResult::Type(DeviceType::GPU) = device_type {
                            selected_device = Some(device);
                            selected_platform = Some(platform);
                            break;
                        }
                    }
                }
                if selected_device.is_some() {
                    break;
                }
            }
        }
        
        let device = selected_device.context("No OpenCL GPU device found")?;
        let platform = selected_platform.context("No OpenCL platform found")?;
        
        let context = OclContext::builder()
            .platform(platform)
            .devices(device)
            .build()
            .context("Failed to create OpenCL context")?;
        let queue = Queue::new(&context, device, None)
            .context("Failed to create command queue")?;

        // Compile kernel
        let program = Program::builder()
            .src(OPENCL_KERNEL)
            .devices(device)
            .build(&context)
            .context("Failed to build OpenCL program")?;

        // Create buffers
        let a_buf = Buffer::builder()
            .queue(queue.clone())
            .len(size)
            .copy_host_slice(a)
            .build()
            .context("Failed to create buffer A")?;

        let b_buf = Buffer::builder()
            .queue(queue.clone())
            .len(size)
            .copy_host_slice(b)
            .build()
            .context("Failed to create buffer B")?;

        let c_buf: Buffer<f32> = Buffer::builder()
            .queue(queue.clone())
            .len(size)
            .build()
            .context("Failed to create buffer C")?;

        // Execute kernel
        let kernel = Kernel::builder()
            .program(&program)
            .name("vector_add")
            .queue(queue.clone())
            .global_work_size(size)
            .arg(&a_buf)
            .arg(&b_buf)
            .arg(&c_buf)
            .arg(size as i32)
            .build()
            .context("Failed to build kernel")?;

        let start_compute = Instant::now();
        unsafe {
            kernel.enq().context("Failed to execute kernel")?;
        }
        queue.finish().context("Failed to finish queue")?;
        let compute_time = start_compute.elapsed();

        // Read results
        let mut result = vec![0.0f32; size];
        c_buf.read(&mut result).enq()
            .context("Failed to read results")?;

        let total_time = start_total.elapsed();

        // Verify
        let cpu_result = super::vector_add_cpu(a, b);
        let correct = super::verify_result(&result, &cpu_result, 1e-5);

        // Calculate throughput (3 arrays * 4 bytes * 2 for read+write)
        let bytes_transferred = (size * 4 * 3) as f64;
        let throughput_gb_s = bytes_transferred / (total_time.as_secs_f64() * 1e9);

        Ok(VectorAddResult {
            backend: format!("OpenCL ({})", device.name().unwrap_or_else(|_| "Unknown".to_string())),
            size,
            compute_time_us: compute_time.as_micros() as f64,
            total_time_us: total_time.as_micros() as f64,
            throughput_gb_s,
            correct,
        })
    }
}

/// CUDA implementation (for comparison)
#[cfg(feature = "cuda")]
pub mod cuda {
    use super::*;
    use anyhow::Context;
    use cudarc::driver::{CudaDevice, DriverError, LaunchAsync, LaunchConfig};
    use std::time::Instant;

    const CUDA_KERNEL: &str = r#"
extern "C" __global__ void vector_add(
    const float* a,
    const float* b,
    float* c,
    int n
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        c[i] = a[i] + b[i];
    }
}
"#;

    pub fn vector_add_cuda(a: &[f32], b: &[f32]) -> Result<VectorAddResult> {
        let size = a.len();
        let start_total = Instant::now();

        // Setup CUDA
        let device = CudaDevice::new(0).context("Failed to create CUDA device")?;

        // Compile kernel
        let ptx = device.compile_ptx(CUDA_KERNEL)
            .map_err(|e| anyhow::anyhow!("Failed to compile PTX: {:?}", e))?;
        device.load_ptx(ptx, "vector_add", &["vector_add"])
            .map_err(|e| anyhow::anyhow!("Failed to load PTX: {:?}", e))?;

        // Allocate device memory
        let a_dev = device.htod_copy(a.to_vec())
            .map_err(|e| anyhow::anyhow!("Failed to copy A to device: {:?}", e))?;
        let b_dev = device.htod_copy(b.to_vec())
            .map_err(|e| anyhow::anyhow!("Failed to copy B to device: {:?}", e))?;
        let mut c_dev = device.alloc_zeros::<f32>(size)
            .map_err(|e| anyhow::anyhow!("Failed to allocate C: {:?}", e))?;

        // Launch kernel
        let block_size = 256;
        let grid_size = (size + block_size - 1) / block_size;
        let cfg = LaunchConfig {
            grid_dim: (grid_size as u32, 1, 1),
            block_dim: (block_size as u32, 1, 1),
            shared_mem_bytes: 0,
        };

        let start_compute = Instant::now();
        let func = device.get_func("vector_add", "vector_add")
            .map_err(|e| anyhow::anyhow!("Failed to get function: {:?}", e))?;
        unsafe {
            func.launch(cfg, (&a_dev, &b_dev, &mut c_dev, size as i32))
                .map_err(|e| anyhow::anyhow!("Failed to launch kernel: {:?}", e))?;
        }
        device.synchronize()
            .map_err(|e| anyhow::anyhow!("Failed to synchronize: {:?}", e))?;
        let compute_time = start_compute.elapsed();

        // Copy results back
        let result = device.dtoh_sync_copy(&c_dev)
            .map_err(|e| anyhow::anyhow!("Failed to copy result: {:?}", e))?;

        let total_time = start_total.elapsed();

        // Verify
        let cpu_result = super::vector_add_cpu(a, b);
        let correct = super::verify_result(&result, &cpu_result, 1e-5);

        // Calculate throughput
        let bytes_transferred = (size * 4 * 3) as f64;
        let throughput_gb_s = bytes_transferred / (total_time.as_secs_f64() * 1e9);

        Ok(VectorAddResult {
            backend: format!("CUDA ({})", device.name().map_err(|e| anyhow::anyhow!("{:?}", e))?),
            size,
            compute_time_us: compute_time.as_micros() as f64,
            total_time_us: total_time.as_micros() as f64,
            throughput_gb_s,
            correct,
        })
    }
}

