//! Multi-GPU Workload Distribution
//!
//! Provides load balancing and parallel execution across multiple GPUs.
//! Works with NVIDIA and AMD via wgpu's vendor-agnostic API.
//!
//! # Features
//!
//! - **GpuPool**: Basic round-robin load balancing
//! - **MultiDevicePool**: Advanced device selection with quotas and requirements
//! - **DeviceRequirements**: Specify minimum VRAM, preferred vendor, etc.
//! - **ResourceQuota integration**: Per-task VRAM budget enforcement
//!
//! # Example
//!
//! ```ignore
//! use barracuda::multi_gpu::{MultiDevicePool, DeviceRequirements};
//! use barracuda::resource_quota::ResourceQuota;
//!
//! // Create a pool with all available GPUs
//! let pool = MultiDevicePool::new().await?;
//! println!("{}", pool.summary());
//!
//! // Acquire a device with specific requirements
//! let reqs = DeviceRequirements::new()
//!     .with_min_vram_gb(8)
//!     .prefer_nvidia();
//!
//! let lease = pool.acquire(&reqs).await?;
//! // Use lease.device() for operations
//! // Device automatically released when lease is dropped
//! ```
//!
//! # Deep Debt Compliance
//!
//! - Modern idiomatic Rust (builder patterns, no global state mutation)
//! - Zero unsafe code
//! - Capability-based device discovery
//! - Proper error handling (Result types, no panics)

mod topology;

pub use topology::{GpuDriver, GpuInfo, GpuVendor, WorkloadType};

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use crate::resource_quota::{QuotaTracker, ResourceQuota};
#[allow(unused_imports)]
use crate::tensor::Tensor;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

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
    devices: Vec<Arc<WgpuDevice>>,
    info: Vec<GpuInfo>,
    semaphore: Arc<Semaphore>,
}

impl GpuPool {
    pub async fn new() -> Result<Self> {
        Self::with_config(WorkloadConfig::default()).await
    }

    pub async fn with_config(config: WorkloadConfig) -> Result<Self> {
        let adapters = WgpuDevice::enumerate_adapters();
        let mut devices = Vec::new();
        let mut info = Vec::new();

        for (idx, adapter) in adapters.iter().enumerate() {
            let vendor = GpuVendor::from_name(&adapter.name);
            if config.exclude_software && vendor == GpuVendor::Software {
                continue;
            }

            let gflops = if vendor == GpuVendor::Software {
                10.0
            } else {
                match adapter.device_type {
                    wgpu::DeviceType::DiscreteGpu => 1000.0,
                    wgpu::DeviceType::IntegratedGpu => 200.0,
                    wgpu::DeviceType::Cpu => 50.0,
                    _ => 100.0,
                }
            };

            if gflops < config.min_gflops {
                continue;
            }

            if let Ok(device) = WgpuDevice::from_adapter_index(idx).await {
                let driver = GpuDriver::from_adapter_info(
                    &adapter.name,
                    &adapter.driver,
                    &adapter.driver_info,
                );
                info.push(GpuInfo {
                    index: idx,
                    name: adapter.name.clone(),
                    vendor,
                    driver,
                    gflops,
                    busy: false,
                });
                devices.push(Arc::new(device));
            }
        }

        let mut indices: Vec<usize> = (0..devices.len()).collect();
        indices.sort_by(|&a, &b| {
            info[b]
                .gflops
                .partial_cmp(&info[a].gflops)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let sorted_devices: Vec<_> = indices.iter().map(|&i| devices[i].clone()).collect();
        let sorted_info: Vec<_> = indices.iter().map(|&i| info[i].clone()).collect();

        tracing::info!("GPU pool initialized with {} devices", sorted_devices.len());
        for gi in &sorted_info {
            tracing::info!(
                "  - {} ({:?}, {:?}, ~{:.0} GFLOPS)",
                gi.name,
                gi.vendor,
                gi.driver,
                gi.gflops
            );
        }

        let max_parallel = config.max_parallel.min(sorted_devices.len()).max(1);

        Ok(Self {
            devices: sorted_devices,
            info: sorted_info,
            semaphore: Arc::new(Semaphore::new(max_parallel)),
        })
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn devices(&self) -> &[GpuInfo] {
        &self.info
    }

    pub fn device(&self, index: usize) -> Option<Arc<WgpuDevice>> {
        self.devices.get(index).cloned()
    }

    pub fn route(&self, workload: WorkloadType) -> Option<(Arc<WgpuDevice>, GpuInfo)> {
        if self.devices.is_empty() {
            return None;
        }
        match workload {
            WorkloadType::Streaming => self
                .devices
                .first()
                .map(|d| (d.clone(), self.info[0].clone())),
            WorkloadType::Iterative => {
                for (i, gi) in self.info.iter().enumerate() {
                    if gi.is_compute_capable() && gi.supports_f64_builtins() {
                        return Some((self.devices[i].clone(), self.info[i].clone()));
                    }
                }
                self.devices
                    .first()
                    .map(|d| (d.clone(), self.info[0].clone()))
            }
            WorkloadType::F64Builtins => {
                for (i, gi) in self.info.iter().enumerate() {
                    if gi.supports_f64_builtins() {
                        let d: Arc<WgpuDevice> = Arc::clone(&self.devices[i]);
                        return Some((d, self.info[i].clone()));
                    }
                }
                tracing::warn!("No GPU with f64 builtin support found - workload may fail on NVK");
                self.devices
                    .first()
                    .map(|d| (d.clone(), self.info[0].clone()))
            }
        }
    }

    pub async fn route_acquire(
        &self,
        workload: WorkloadType,
    ) -> Result<(Arc<WgpuDevice>, GpuInfo, tokio::sync::OwnedSemaphorePermit)> {
        let permit = Arc::clone(&self.semaphore)
            .acquire_owned()
            .await
            .map_err(|e| BarracudaError::device(format!("Semaphore error: {e}")))?;

        let (device, info) = self
            .route(workload)
            .ok_or_else(|| BarracudaError::device_not_found("No GPU available for workload"))?;

        Ok((device, info, permit))
    }

    pub async fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(Arc<WgpuDevice>) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let _permit =
            self.semaphore.acquire().await.map_err(|e| {
                crate::error::BarracudaError::device(format!("Semaphore error: {e}"))
            })?;

        let device =
            self.devices.first().cloned().ok_or_else(|| {
                crate::error::BarracudaError::device_not_found("No GPU available")
            })?;

        tokio::task::spawn_blocking(move || f(device))
            .await
            .map_err(|e| crate::error::BarracudaError::device(format!("Task error: {e}")))?
    }

    pub async fn parallel_map<T, R, F>(&self, data: Vec<T>, f: F) -> Result<Vec<R>>
    where
        T: Send + 'static,
        R: Send + 'static,
        F: Fn(Arc<WgpuDevice>, T) -> Result<R> + Send + Sync + Clone + 'static,
    {
        use futures::future::join_all;

        let num_devices = self.devices.len().max(1);
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
                Ok(Err(e)) => {
                    return Err(crate::error::BarracudaError::device(format!(
                        "Task error: {e}"
                    )))
                }
                Err(e) => {
                    return Err(crate::error::BarracudaError::device(format!(
                        "Join error: {e}"
                    )))
                }
            }
        }
        Ok(output)
    }

    pub fn summary(&self) -> String {
        let total_gflops: f64 = self.info.iter().map(|g| g.gflops).sum();
        let nvidia_count = self
            .info
            .iter()
            .filter(|g| g.vendor == GpuVendor::Nvidia)
            .count();
        let amd_count = self
            .info
            .iter()
            .filter(|g| g.vendor == GpuVendor::Amd)
            .count();

        format!(
            "{} GPUs ({} NVIDIA, {} AMD), ~{:.0} GFLOPS total",
            self.devices.len(),
            nvidia_count,
            amd_count,
            total_gflops
        )
    }
}

// ============================================================================
// MultiDevicePool - Advanced Device Pool with Quotas
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct DeviceRequirements {
    pub min_vram_bytes: Option<u64>,
    pub preferred_vendor: Option<GpuVendor>,
    pub exclude_software: bool,
    pub require_discrete: bool,
    pub min_gflops: Option<f64>,
}

impl DeviceRequirements {
    pub fn new() -> Self {
        Self {
            exclude_software: true,
            ..Self::default()
        }
    }

    pub fn with_min_vram_bytes(mut self, bytes: u64) -> Self {
        self.min_vram_bytes = Some(bytes);
        self
    }

    pub fn with_min_vram_gb(self, gb: u64) -> Self {
        self.with_min_vram_bytes(gb * 1024 * 1024 * 1024)
    }

    pub fn prefer_nvidia(mut self) -> Self {
        self.preferred_vendor = Some(GpuVendor::Nvidia);
        self
    }

    pub fn prefer_amd(mut self) -> Self {
        self.preferred_vendor = Some(GpuVendor::Amd);
        self
    }

    pub fn require_discrete(mut self) -> Self {
        self.require_discrete = true;
        self
    }

    pub fn with_min_gflops(mut self, gflops: f64) -> Self {
        self.min_gflops = Some(gflops);
        self
    }

    fn score(&self, info: &DeviceInfo) -> Option<i64> {
        if self.exclude_software && info.vendor == GpuVendor::Software {
            return None;
        }
        if self.require_discrete && !info.is_discrete {
            return None;
        }
        if let Some(min_vram) = self.min_vram_bytes {
            if info.vram_bytes < min_vram {
                return None;
            }
        }
        if let Some(min_gflops) = self.min_gflops {
            if info.estimated_gflops < min_gflops {
                return None;
            }
        }

        let mut score: i64 = 0;
        if let Some(pref) = self.preferred_vendor {
            if info.vendor == pref {
                score += 1000;
            }
        }
        score += (info.vram_bytes / (1024 * 1024 * 1024)) as i64;
        score += (info.estimated_gflops / 100.0) as i64;
        if info.is_discrete {
            score += 100;
        }
        if !info.is_busy() {
            score += 50;
        }
        Some(score)
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub index: usize,
    pool_index: usize,
    pub name: String,
    pub vendor: GpuVendor,
    pub driver: GpuDriver,
    pub vram_bytes: u64,
    pub estimated_gflops: f64,
    pub is_discrete: bool,
    allocations: Arc<AtomicUsize>,
    allocated_bytes: Arc<AtomicU64>,
    busy: Arc<AtomicBool>,
}

impl DeviceInfo {
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.load(Ordering::Relaxed)
    }

    pub fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes.load(Ordering::Relaxed)
    }

    pub fn available_vram_bytes(&self) -> u64 {
        self.vram_bytes.saturating_sub(self.allocated_bytes())
    }

    pub fn usage_percent(&self) -> f64 {
        if self.vram_bytes == 0 {
            return 0.0;
        }
        (self.allocated_bytes() as f64 / self.vram_bytes as f64) * 100.0
    }

    pub fn supports_f64_builtins(&self) -> bool {
        !matches!(self.driver, GpuDriver::Nvk | GpuDriver::Software)
    }
}

pub struct DeviceLease {
    device: Arc<WgpuDevice>,
    info: DeviceInfo,
    pool: Arc<MultiDevicePoolInner>,
    quota_tracker: Option<Arc<QuotaTracker>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl DeviceLease {
    pub fn device(&self) -> &Arc<WgpuDevice> {
        &self.device
    }

    pub fn info(&self) -> &DeviceInfo {
        &self.info
    }

    pub fn quota_tracker(&self) -> Option<&Arc<QuotaTracker>> {
        self.quota_tracker.as_ref()
    }

    pub fn track_allocation(&self, bytes: u64) -> Result<()> {
        if let Some(tracker) = &self.quota_tracker {
            tracker.try_allocate(bytes)?;
        }
        self.info
            .allocated_bytes
            .fetch_add(bytes, Ordering::Relaxed);
        self.info.allocations.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn track_deallocation(&self, bytes: u64) {
        if let Some(tracker) = &self.quota_tracker {
            tracker.deallocate(bytes);
        }
        self.info
            .allocated_bytes
            .fetch_sub(bytes, Ordering::Relaxed);
        self.info.allocations.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Drop for DeviceLease {
    fn drop(&mut self) {
        self.pool.release_device(self.info.pool_index);
    }
}

struct MultiDevicePoolInner {
    devices: Vec<Arc<WgpuDevice>>,
    info: Vec<DeviceInfo>,
    semaphore: Arc<Semaphore>,
    device_busy: Vec<Arc<AtomicBool>>,
    selection_lock: Mutex<()>,
}

impl MultiDevicePoolInner {
    fn release_device(&self, index: usize) {
        if let Some(busy) = self.device_busy.get(index) {
            busy.store(false, Ordering::Release);
        }
    }
}

pub struct MultiDevicePool {
    inner: Arc<MultiDevicePoolInner>,
}

impl MultiDevicePool {
    pub async fn new() -> Result<Self> {
        Self::with_config(WorkloadConfig::default()).await
    }

    pub async fn with_config(config: WorkloadConfig) -> Result<Self> {
        let adapters = WgpuDevice::enumerate_adapters();
        let mut devices = Vec::new();
        let mut info = Vec::new();
        let mut device_busy = Vec::new();

        for (idx, adapter) in adapters.iter().enumerate() {
            let vendor = GpuVendor::from_name(&adapter.name);
            if config.exclude_software && vendor == GpuVendor::Software {
                continue;
            }

            let is_likely_discrete = adapter.device_type == wgpu::DeviceType::DiscreteGpu
                || (adapter.device_type == wgpu::DeviceType::Other
                    && (vendor == GpuVendor::Nvidia || vendor == GpuVendor::Amd));

            let (estimated_gflops, estimated_vram) = if vendor == GpuVendor::Software {
                (10.0, 0u64)
            } else if is_likely_discrete {
                let gflops = match vendor {
                    GpuVendor::Nvidia => 5000.0,
                    GpuVendor::Amd => 4000.0,
                    _ => 1000.0,
                };
                let vram = match vendor {
                    GpuVendor::Nvidia => 12 * 1024 * 1024 * 1024,
                    GpuVendor::Amd => 16 * 1024 * 1024 * 1024,
                    _ => 8 * 1024 * 1024 * 1024,
                };
                (gflops, vram)
            } else {
                match adapter.device_type {
                    wgpu::DeviceType::IntegratedGpu => (200.0, 2 * 1024 * 1024 * 1024),
                    wgpu::DeviceType::Cpu => (50.0, 0),
                    _ => (100.0, 4 * 1024 * 1024 * 1024),
                }
            };

            if estimated_gflops < config.min_gflops {
                continue;
            }

            tracing::debug!(
                "Attempting to create device for adapter {}: {} ({:?})",
                idx,
                adapter.name,
                adapter.device_type
            );

            match WgpuDevice::from_adapter_index(idx).await {
                Ok(device) => {
                    tracing::info!(
                        "Successfully created device for adapter {}: {}",
                        idx,
                        adapter.name
                    );
                    let busy = Arc::new(AtomicBool::new(false));
                    let allocations = Arc::new(AtomicUsize::new(0));
                    let allocated_bytes = Arc::new(AtomicU64::new(0));

                    let driver = GpuDriver::from_adapter_info(
                        &adapter.name,
                        &adapter.driver,
                        &adapter.driver_info,
                    );

                    info.push(DeviceInfo {
                        index: idx,
                        pool_index: 0,
                        name: adapter.name.clone(),
                        vendor,
                        driver,
                        vram_bytes: estimated_vram,
                        estimated_gflops,
                        is_discrete: is_likely_discrete,
                        allocations: allocations.clone(),
                        allocated_bytes: allocated_bytes.clone(),
                        busy: busy.clone(),
                    });
                    device_busy.push(busy);
                    devices.push(Arc::new(device));
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to create device for adapter {}: {} - {}",
                        idx,
                        adapter.name,
                        e
                    );
                }
            }
        }

        if devices.is_empty() {
            return Err(BarracudaError::device_not_found(
                "No suitable GPU devices found",
            ));
        }

        let mut indices: Vec<usize> = (0..devices.len()).collect();
        indices.sort_by(|&a, &b| {
            info[b]
                .estimated_gflops
                .partial_cmp(&info[a].estimated_gflops)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let sorted_devices: Vec<_> = indices.iter().map(|&i| devices[i].clone()).collect();
        let mut sorted_info: Vec<_> = indices.iter().map(|&i| info[i].clone()).collect();
        let sorted_busy: Vec<_> = indices.iter().map(|&i| device_busy[i].clone()).collect();

        for (pool_idx, di) in sorted_info.iter_mut().enumerate() {
            di.pool_index = pool_idx;
        }

        tracing::info!(
            "MultiDevicePool initialized with {} devices",
            sorted_devices.len()
        );
        for di in &sorted_info {
            tracing::info!(
                "  - {} ({:?}, ~{:.0} GFLOPS, ~{} GB VRAM)",
                di.name,
                di.vendor,
                di.estimated_gflops,
                di.vram_bytes / (1024 * 1024 * 1024)
            );
        }

        let max_parallel = config.max_parallel.min(sorted_devices.len()).max(1);

        Ok(Self {
            inner: Arc::new(MultiDevicePoolInner {
                devices: sorted_devices,
                info: sorted_info,
                semaphore: Arc::new(Semaphore::new(max_parallel)),
                device_busy: sorted_busy,
                selection_lock: Mutex::new(()),
            }),
        })
    }

    pub fn device_count(&self) -> usize {
        self.inner.devices.len()
    }

    pub fn devices(&self) -> &[DeviceInfo] {
        &self.inner.info
    }

    pub async fn acquire(&self, requirements: &DeviceRequirements) -> Result<DeviceLease> {
        self.acquire_with_quota(requirements, None).await
    }

    pub async fn acquire_with_quota(
        &self,
        requirements: &DeviceRequirements,
        quota: Option<ResourceQuota>,
    ) -> Result<DeviceLease> {
        let permit = self
            .inner
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| BarracudaError::device(format!("Semaphore error: {e}")))?;

        let _lock = self.inner.selection_lock.lock().await;

        let mut best_idx = None;
        let mut best_score = i64::MIN;

        for (i, info) in self.inner.info.iter().enumerate() {
            if self.inner.device_busy[i].load(Ordering::Acquire) {
                continue;
            }
            if let Some(score) = requirements.score(info) {
                if score > best_score {
                    best_score = score;
                    best_idx = Some(i);
                }
            }
        }

        let idx = best_idx
            .ok_or_else(|| BarracudaError::device_not_found("No device matches requirements"))?;

        self.inner.device_busy[idx].store(true, Ordering::Release);

        let quota_tracker = quota.map(|q| Arc::new(QuotaTracker::new(q)));

        Ok(DeviceLease {
            device: self.inner.devices[idx].clone(),
            info: self.inner.info[idx].clone(),
            pool: self.inner.clone(),
            quota_tracker,
            _permit: permit,
        })
    }

    pub async fn acquire_any(&self) -> Result<DeviceLease> {
        self.acquire(&DeviceRequirements::new()).await
    }

    pub fn device(&self, index: usize) -> Option<Arc<WgpuDevice>> {
        self.inner.devices.get(index).cloned()
    }

    pub async fn execute<F, T>(&self, requirements: &DeviceRequirements, f: F) -> Result<T>
    where
        F: FnOnce(Arc<WgpuDevice>) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let lease = self.acquire(requirements).await?;
        let device = lease.device().clone();

        tokio::task::spawn_blocking(move || f(device))
            .await
            .map_err(|e| BarracudaError::device(format!("Task error: {e}")))?
    }

    pub fn summary(&self) -> String {
        let total_vram: u64 = self.inner.info.iter().map(|d| d.vram_bytes).sum();
        let allocated_vram: u64 = self.inner.info.iter().map(|d| d.allocated_bytes()).sum();
        let total_gflops: f64 = self.inner.info.iter().map(|d| d.estimated_gflops).sum();

        let nvidia_count = self
            .inner
            .info
            .iter()
            .filter(|d| d.vendor == GpuVendor::Nvidia)
            .count();
        let amd_count = self
            .inner
            .info
            .iter()
            .filter(|d| d.vendor == GpuVendor::Amd)
            .count();
        let busy_count = self
            .inner
            .device_busy
            .iter()
            .filter(|b| b.load(Ordering::Relaxed))
            .count();

        format!(
            "{} GPUs ({} NVIDIA, {} AMD), ~{:.0} GFLOPS, ~{} GB total VRAM ({} GB allocated), {} busy",
            self.inner.devices.len(),
            nvidia_count,
            amd_count,
            total_gflops,
            total_vram / (1024 * 1024 * 1024),
            allocated_vram / (1024 * 1024 * 1024),
            busy_count
        )
    }

    pub fn device_status(&self) -> Vec<String> {
        self.inner
            .info
            .iter()
            .enumerate()
            .map(|(i, info)| {
                let busy = self.inner.device_busy[i].load(Ordering::Relaxed);
                format!(
                    "[{}] {} ({:?}): {:.1}% used, {} allocations, {}",
                    i,
                    info.name,
                    info.vendor,
                    info.usage_percent(),
                    info.allocation_count(),
                    if busy { "BUSY" } else { "available" }
                )
            })
            .collect()
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
        assert_eq!(
            GpuVendor::from_name("NVIDIA GeForce RTX 3090"),
            GpuVendor::Nvidia
        );
        assert_eq!(
            GpuVendor::from_name("AMD Radeon RX 6950 XT (RADV NAVI21)"),
            GpuVendor::Amd
        );
        assert_eq!(GpuVendor::from_name("llvmpipe"), GpuVendor::Software);
    }

    #[tokio::test]
    async fn test_multi_device_pool_creation() {
        let pool = MultiDevicePool::new().await;
        match pool {
            Ok(pool) => {
                println!("MultiDevicePool: {}", pool.summary());
                for status in pool.device_status() {
                    println!("  {}", status);
                }
            }
            Err(e) => {
                println!("No GPU available: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_device_requirements() {
        let pool = MultiDevicePool::new().await;
        if let Ok(pool) = pool {
            let reqs = DeviceRequirements::new().prefer_nvidia();
            if let Ok(lease) = pool.acquire(&reqs).await {
                println!(
                    "Acquired: {} ({:?})",
                    lease.info().name,
                    lease.info().vendor
                );
            }
            let reqs = DeviceRequirements::new().with_min_vram_gb(100);
            let result = pool.acquire(&reqs).await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_device_lease_tracking() {
        let pool = MultiDevicePool::new().await;
        if let Ok(pool) = pool {
            let quota = ResourceQuota::new().with_max_vram_mb(100);
            if let Ok(lease) = pool
                .acquire_with_quota(&DeviceRequirements::new(), Some(quota))
                .await
            {
                assert!(lease.track_allocation(50 * 1024 * 1024).is_ok());
                assert!(lease.track_allocation(50 * 1024 * 1024).is_ok());
                assert!(lease.track_allocation(1).is_err());
                lease.track_deallocation(50 * 1024 * 1024);
                assert!(lease.track_allocation(1).is_ok());
            }
        }
    }

    #[test]
    fn test_device_requirements_scoring() {
        let reqs = DeviceRequirements::new()
            .prefer_nvidia()
            .with_min_vram_gb(8);

        let nvidia_info = DeviceInfo {
            index: 0,
            pool_index: 0,
            name: "RTX 4070".to_string(),
            vendor: GpuVendor::Nvidia,
            driver: GpuDriver::NvidiaProprietary,
            vram_bytes: 12 * 1024 * 1024 * 1024,
            estimated_gflops: 5000.0,
            is_discrete: true,
            allocations: Arc::new(AtomicUsize::new(0)),
            allocated_bytes: Arc::new(AtomicU64::new(0)),
            busy: Arc::new(AtomicBool::new(false)),
        };

        let amd_info = DeviceInfo {
            index: 1,
            pool_index: 1,
            name: "RX 6800".to_string(),
            vendor: GpuVendor::Amd,
            driver: GpuDriver::Radv,
            vram_bytes: 16 * 1024 * 1024 * 1024,
            estimated_gflops: 4000.0,
            is_discrete: true,
            allocations: Arc::new(AtomicUsize::new(0)),
            allocated_bytes: Arc::new(AtomicU64::new(0)),
            busy: Arc::new(AtomicBool::new(false)),
        };

        let nvidia_score = reqs.score(&nvidia_info).unwrap();
        let amd_score = reqs.score(&amd_info).unwrap();
        assert!(nvidia_score > amd_score);
    }
}
