//! Multi-GPU Workload Distribution
//!
//! Provides load balancing and parallel execution across multiple GPUs.
//! Works with NVIDIA and AMD via wgpu's vendor-agnostic API.
//!
//! # Example
//!
//! ```ignore
//! use barracuda::multi_gpu::{GpuPool, WorkloadConfig};
//!
//! let pool = GpuPool::new().await?;
//! let result = pool.parallel_matmul(&matrices, WorkloadConfig::default()).await?;
//! ```

use crate::device::WgpuDevice;
#[allow(unused_imports)]
use crate::tensor::Tensor;
use crate::error::Result;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// GPU information for load balancing
#[derive(Debug, Clone)]
pub struct GpuInfo {
    /// Adapter index
    pub index: usize,
    /// Device name
    pub name: String,
    /// Vendor
    pub vendor: GpuVendor,
    /// Estimated GFLOPS
    pub gflops: f64,
    /// Currently busy
    pub busy: bool,
}

/// GPU vendor
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Software,
    Unknown,
}

impl GpuVendor {
    fn from_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        
        // Check for software renderers FIRST (they may contain GPU brand names)
        // SSE2/SSE4/AVX in name indicates CPU-based rendering
        if lower.contains("llvmpipe") 
            || lower.contains("software") 
            || lower.contains("sse2")
            || lower.contains("sse4")
            || lower.contains("avx")
            || lower.contains("swiftshader")
            || lower.contains("cpu")
        {
            return Self::Software;
        }
        
        // Now check for actual GPU vendors
        if lower.contains("nvidia") || lower.contains("geforce") || lower.contains("rtx") || lower.contains("gtx") {
            Self::Nvidia
        } else if lower.contains("amd") || lower.contains("radeon") || lower.contains("radv") {
            Self::Amd
        } else if lower.contains("intel") || lower.contains("iris") {
            Self::Intel
        } else {
            Self::Unknown
        }
    }
}

/// Workload distribution configuration
#[derive(Debug, Clone)]
pub struct WorkloadConfig {
    /// Maximum parallel tasks
    pub max_parallel: usize,
    /// Prefer discrete GPUs
    pub prefer_discrete: bool,
    /// Exclude software renderer
    pub exclude_software: bool,
    /// Minimum GFLOPS to include
    pub min_gflops: f64,
}

impl Default for WorkloadConfig {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            prefer_discrete: true,
            exclude_software: true,
            min_gflops: 100.0,
        }
    }
}

/// Pool of GPU devices for parallel execution
pub struct GpuPool {
    /// Available devices
    devices: Vec<Arc<WgpuDevice>>,
    /// Device info
    info: Vec<GpuInfo>,
    /// Semaphore for limiting concurrency
    semaphore: Arc<Semaphore>,
}

impl GpuPool {
    /// Create a new GPU pool from all available devices
    pub async fn new() -> Result<Self> {
        Self::with_config(WorkloadConfig::default()).await
    }

    /// Create with specific configuration
    pub async fn with_config(config: WorkloadConfig) -> Result<Self> {
        let adapters = WgpuDevice::enumerate_adapters();
        
        let mut devices = Vec::new();
        let mut info = Vec::new();

        for (idx, adapter) in adapters.iter().enumerate() {
            let vendor = GpuVendor::from_name(&adapter.name);
            
            // Skip software renderer if configured
            if config.exclude_software && vendor == GpuVendor::Software {
                continue;
            }

            // Estimate GFLOPS based on device type and vendor
            let gflops = if vendor == GpuVendor::Software {
                // Software renderers are very slow regardless of device_type reporting
                10.0
            } else {
                match adapter.device_type {
                    wgpu::DeviceType::DiscreteGpu => 1000.0, // Conservative estimate
                    wgpu::DeviceType::IntegratedGpu => 200.0,
                    wgpu::DeviceType::Cpu => 50.0,
                    _ => 100.0,
                }
            };

            if gflops < config.min_gflops {
                continue;
            }

            // Create device
            if let Ok(device) = WgpuDevice::from_adapter_index(idx).await {
                info.push(GpuInfo {
                    index: idx,
                    name: adapter.name.clone(),
                    vendor,
                    gflops,
                    busy: false,
                });
                devices.push(Arc::new(device));
            }
        }

        // Sort by GFLOPS (highest first)
        let mut indices: Vec<usize> = (0..devices.len()).collect();
        indices.sort_by(|&a, &b| {
            info[b].gflops.partial_cmp(&info[a].gflops).unwrap_or(std::cmp::Ordering::Equal)
        });

        let sorted_devices: Vec<_> = indices.iter().map(|&i| devices[i].clone()).collect();
        let sorted_info: Vec<_> = indices.iter().map(|&i| info[i].clone()).collect();

        tracing::info!("GPU pool initialized with {} devices", sorted_devices.len());
        for gi in &sorted_info {
            tracing::info!("  - {} ({:?}, ~{:.0} GFLOPS)", gi.name, gi.vendor, gi.gflops);
        }

        let max_parallel = config.max_parallel.min(sorted_devices.len()).max(1);
        
        Ok(Self {
            devices: sorted_devices,
            info: sorted_info,
            semaphore: Arc::new(Semaphore::new(max_parallel)),
        })
    }

    /// Get number of available devices
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Get device info
    pub fn devices(&self) -> &[GpuInfo] {
        &self.info
    }

    /// Get a specific device
    pub fn device(&self, index: usize) -> Option<Arc<WgpuDevice>> {
        self.devices.get(index).cloned()
    }

    /// Execute a closure on the best available device
    pub async fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(Arc<WgpuDevice>) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            crate::error::BarracudaError::device(format!("Semaphore error: {e}"))
        })?;

        // Use first available device (already sorted by performance)
        let device = self.devices.first().cloned().ok_or_else(|| {
            crate::error::BarracudaError::device_not_found("No GPU available")
        })?;

        // Execute in blocking task for CPU-bound work
        tokio::task::spawn_blocking(move || f(device))
            .await
            .map_err(|e| crate::error::BarracudaError::device(format!("Task error: {e}")))?
    }

    /// Parallel map over data chunks using multiple GPUs
    pub async fn parallel_map<T, R, F>(&self, data: Vec<T>, f: F) -> Result<Vec<R>>
    where
        T: Send + 'static,
        R: Send + 'static,
        F: Fn(Arc<WgpuDevice>, T) -> Result<R> + Send + Sync + Clone + 'static,
    {
        use futures::future::join_all;

        let num_devices = self.devices.len().max(1);
        let _chunk_size = (data.len() + num_devices - 1) / num_devices;

        let mut handles = Vec::new();

        for (i, chunk) in data.into_iter().enumerate() {
            let device = self.devices[i % num_devices].clone();
            let f = f.clone();
            let semaphore = self.semaphore.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await;
                tokio::task::spawn_blocking(move || f(device, chunk)).await
            });

            handles.push(handle);
        }

        let results: Vec<_> = join_all(handles).await;
        
        let mut output = Vec::new();
        for result in results {
            match result {
                Ok(Ok(Ok(value))) => output.push(value),
                Ok(Ok(Err(e))) => return Err(e),
                Ok(Err(e)) => return Err(crate::error::BarracudaError::device(format!("Task error: {e}"))),
                Err(e) => return Err(crate::error::BarracudaError::device(format!("Join error: {e}"))),
            }
        }

        Ok(output)
    }

    /// Get summary of pool capabilities
    pub fn summary(&self) -> String {
        let total_gflops: f64 = self.info.iter().map(|g| g.gflops).sum();
        let nvidia_count = self.info.iter().filter(|g| g.vendor == GpuVendor::Nvidia).count();
        let amd_count = self.info.iter().filter(|g| g.vendor == GpuVendor::Amd).count();

        format!(
            "{} GPUs ({} NVIDIA, {} AMD), ~{:.0} GFLOPS total",
            self.devices.len(),
            nvidia_count,
            amd_count,
            total_gflops
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpu_pool_creation() {
        let pool = GpuPool::new().await;
        if let Ok(pool) = pool {
            println!("Pool: {}", pool.summary());
            for device in pool.devices() {
                println!("  - {} ({:?})", device.name, device.vendor);
            }
        }
    }

    #[test]
    fn test_vendor_detection() {
        assert_eq!(GpuVendor::from_name("NVIDIA GeForce RTX 3090"), GpuVendor::Nvidia);
        assert_eq!(GpuVendor::from_name("AMD Radeon RX 6950 XT (RADV NAVI21)"), GpuVendor::Amd);
        assert_eq!(GpuVendor::from_name("llvmpipe"), GpuVendor::Software);
    }
}
