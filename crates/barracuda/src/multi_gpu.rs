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

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use crate::resource_quota::{QuotaTracker, ResourceQuota};
#[allow(unused_imports)]
use crate::tensor::Tensor;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

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

        // Check for actual GPU vendors FIRST (they take priority over software detection)
        // Some OpenGL drivers include "SSE2" in hardware GPU names (e.g., "NVIDIA GeForce RTX 3090/PCIe/SSE2")
        if lower.contains("nvidia")
            || lower.contains("geforce")
            || lower.contains("rtx")
            || lower.contains("gtx")
        {
            return Self::Nvidia;
        }

        if lower.contains("amd") || lower.contains("radeon") || lower.contains("radv") {
            return Self::Amd;
        }

        if lower.contains("intel") || lower.contains("iris") {
            return Self::Intel;
        }

        // Check for software renderers (only after confirming it's not a known hardware vendor)
        // SSE2/SSE4/AVX in name indicates CPU-based rendering for software rasterizers
        if lower.contains("llvmpipe")
            || lower.contains("software")
            || lower.contains("swiftshader")
            || lower.contains("cpu")
            // Only treat as software if no known GPU vendor was matched
            || lower.contains("sse2")
            || lower.contains("sse4")
            || lower.contains("avx")
        {
            return Self::Software;
        }

        Self::Unknown
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
                "  - {} ({:?}, ~{:.0} GFLOPS)",
                gi.name,
                gi.vendor,
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
        let _permit =
            self.semaphore.acquire().await.map_err(|e| {
                crate::error::BarracudaError::device(format!("Semaphore error: {e}"))
            })?;

        // Use first available device (already sorted by performance)
        let device =
            self.devices.first().cloned().ok_or_else(|| {
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
        let _chunk_size = data.len().div_ceil(num_devices);

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

    /// Get summary of pool capabilities
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

/// Requirements for device selection
#[derive(Debug, Clone, Default)]
pub struct DeviceRequirements {
    /// Minimum VRAM in bytes (None = any)
    pub min_vram_bytes: Option<u64>,

    /// Preferred GPU vendor (None = no preference)
    pub preferred_vendor: Option<GpuVendor>,

    /// Exclude software renderers
    pub exclude_software: bool,

    /// Require discrete GPU
    pub require_discrete: bool,

    /// Minimum estimated GFLOPS
    pub min_gflops: Option<f64>,
}

impl DeviceRequirements {
    /// Create new requirements with defaults
    pub fn new() -> Self {
        Self {
            exclude_software: true,
            ..Self::default()
        }
    }

    /// Set minimum VRAM in bytes
    pub fn with_min_vram_bytes(mut self, bytes: u64) -> Self {
        self.min_vram_bytes = Some(bytes);
        self
    }

    /// Set minimum VRAM in gigabytes
    pub fn with_min_vram_gb(self, gb: u64) -> Self {
        self.with_min_vram_bytes(gb * 1024 * 1024 * 1024)
    }

    /// Prefer NVIDIA GPUs
    pub fn prefer_nvidia(mut self) -> Self {
        self.preferred_vendor = Some(GpuVendor::Nvidia);
        self
    }

    /// Prefer AMD GPUs
    pub fn prefer_amd(mut self) -> Self {
        self.preferred_vendor = Some(GpuVendor::Amd);
        self
    }

    /// Require discrete GPU (no integrated)
    pub fn require_discrete(mut self) -> Self {
        self.require_discrete = true;
        self
    }

    /// Set minimum GFLOPS
    pub fn with_min_gflops(mut self, gflops: f64) -> Self {
        self.min_gflops = Some(gflops);
        self
    }

    /// Check if device info meets requirements (returns score, higher is better)
    fn score(&self, info: &DeviceInfo) -> Option<i64> {
        // Disqualifying checks
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

        // Scoring (higher is better)
        let mut score: i64 = 0;

        // Prefer requested vendor (+1000)
        if let Some(pref) = self.preferred_vendor {
            if info.vendor == pref {
                score += 1000;
            }
        }

        // Prefer more VRAM (+1 per GB)
        score += (info.vram_bytes / (1024 * 1024 * 1024)) as i64;

        // Prefer higher GFLOPS (+1 per 100 GFLOPS)
        score += (info.estimated_gflops / 100.0) as i64;

        // Prefer discrete GPUs (+100)
        if info.is_discrete {
            score += 100;
        }

        // Prefer less busy devices (+50 per free slot relative to busy devices)
        if !info.is_busy() {
            score += 50;
        }

        Some(score)
    }
}

/// Extended device information for MultiDevicePool
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Adapter index (from wgpu enumerate_adapters)
    pub index: usize,

    /// Pool index (position in MultiDevicePool.devices)
    pool_index: usize,

    /// Device name
    pub name: String,

    /// Vendor
    pub vendor: GpuVendor,

    /// Total VRAM in bytes
    pub vram_bytes: u64,

    /// Estimated GFLOPS
    pub estimated_gflops: f64,

    /// Is discrete GPU
    pub is_discrete: bool,

    /// Current allocations (tracked externally)
    allocations: Arc<AtomicUsize>,

    /// Currently allocated VRAM bytes
    allocated_bytes: Arc<AtomicU64>,

    /// Is device currently busy
    busy: Arc<AtomicBool>,
}

impl DeviceInfo {
    /// Check if device is currently busy
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }

    /// Get current allocation count
    pub fn allocation_count(&self) -> usize {
        self.allocations.load(Ordering::Relaxed)
    }

    /// Get currently allocated bytes
    pub fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes.load(Ordering::Relaxed)
    }

    /// Get available VRAM (estimated)
    pub fn available_vram_bytes(&self) -> u64 {
        self.vram_bytes.saturating_sub(self.allocated_bytes())
    }

    /// Get usage percentage
    pub fn usage_percent(&self) -> f64 {
        if self.vram_bytes == 0 {
            return 0.0;
        }
        (self.allocated_bytes() as f64 / self.vram_bytes as f64) * 100.0
    }
}

/// A lease on a device from the pool
///
/// When dropped, the device is released back to the pool.
pub struct DeviceLease {
    device: Arc<WgpuDevice>,
    info: DeviceInfo,
    pool: Arc<MultiDevicePoolInner>,
    quota_tracker: Option<Arc<QuotaTracker>>,
    /// Semaphore permit - held while device is leased
    #[allow(dead_code)] // Held for Drop semantics
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl DeviceLease {
    /// Get the leased device
    pub fn device(&self) -> &Arc<WgpuDevice> {
        &self.device
    }

    /// Get device info
    pub fn info(&self) -> &DeviceInfo {
        &self.info
    }

    /// Get quota tracker if one was assigned
    pub fn quota_tracker(&self) -> Option<&Arc<QuotaTracker>> {
        self.quota_tracker.as_ref()
    }

    /// Track an allocation against the quota (if assigned)
    pub fn track_allocation(&self, bytes: u64) -> Result<()> {
        if let Some(tracker) = &self.quota_tracker {
            tracker.try_allocate(bytes)?;
        }
        // Also update device-level tracking
        self.info.allocated_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.info.allocations.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Track a deallocation
    pub fn track_deallocation(&self, bytes: u64) {
        if let Some(tracker) = &self.quota_tracker {
            tracker.deallocate(bytes);
        }
        self.info.allocated_bytes.fetch_sub(bytes, Ordering::Relaxed);
        self.info.allocations.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Drop for DeviceLease {
    fn drop(&mut self) {
        // Release the device back to the pool (use pool_index, not adapter index)
        self.pool.release_device(self.info.pool_index);
    }
}

/// Inner pool state (separated for Arc sharing)
struct MultiDevicePoolInner {
    /// Available devices
    devices: Vec<Arc<WgpuDevice>>,

    /// Device info
    info: Vec<DeviceInfo>,

    /// Semaphore for limiting total concurrency
    semaphore: Arc<Semaphore>,

    /// Per-device busy flags
    device_busy: Vec<Arc<AtomicBool>>,

    /// Lock for device selection
    selection_lock: Mutex<()>,
}

impl MultiDevicePoolInner {
    fn release_device(&self, index: usize) {
        if let Some(busy) = self.device_busy.get(index) {
            busy.store(false, Ordering::Release);
        }
        // Release semaphore permit happens automatically when DeviceLease is dropped
    }
}

/// Advanced multi-device pool with quota support
///
/// Supports heterogeneous GPU configurations (mixed NVIDIA/AMD),
/// requirement-based device selection, and quota enforcement.
pub struct MultiDevicePool {
    inner: Arc<MultiDevicePoolInner>,
}

impl MultiDevicePool {
    /// Create a new pool from all available GPUs
    pub async fn new() -> Result<Self> {
        Self::with_config(WorkloadConfig::default()).await
    }

    /// Create with specific configuration
    pub async fn with_config(config: WorkloadConfig) -> Result<Self> {
        let adapters = WgpuDevice::enumerate_adapters();

        let mut devices = Vec::new();
        let mut info = Vec::new();
        let mut device_busy = Vec::new();

        for (idx, adapter) in adapters.iter().enumerate() {
            let vendor = GpuVendor::from_name(&adapter.name);

            // Skip software renderer if configured
            if config.exclude_software && vendor == GpuVendor::Software {
                continue;
            }

            // Estimate GFLOPS and VRAM based on device type and vendor
            // Note: Some drivers report discrete GPUs as "Other" (e.g., NVIDIA OpenGL)
            // so we also check vendor for known discrete GPU vendors
            let is_likely_discrete = adapter.device_type == wgpu::DeviceType::DiscreteGpu
                || (adapter.device_type == wgpu::DeviceType::Other
                    && (vendor == GpuVendor::Nvidia || vendor == GpuVendor::Amd));

            let (estimated_gflops, estimated_vram) = if vendor == GpuVendor::Software {
                (10.0, 0u64)
            } else if is_likely_discrete {
                // Conservative estimates - real values depend on specific GPU
                let gflops = match vendor {
                    GpuVendor::Nvidia => 5000.0, // RTX class
                    GpuVendor::Amd => 4000.0,    // RX class
                    _ => 1000.0,
                };
                // Estimate VRAM based on vendor (will be refined with actual queries)
                let vram = match vendor {
                    GpuVendor::Nvidia => 12 * 1024 * 1024 * 1024, // 12 GB estimate
                    GpuVendor::Amd => 16 * 1024 * 1024 * 1024,    // 16 GB estimate
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

            // Create device
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

                    info.push(DeviceInfo {
                    index: idx,
                    pool_index: 0, // Will be set after sorting
                    name: adapter.name.clone(),
                    vendor,
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

        // Sort by estimated GFLOPS (highest first)
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

        // Update pool_index to reflect position in sorted array
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

    /// Get number of available devices
    pub fn device_count(&self) -> usize {
        self.inner.devices.len()
    }

    /// Get device info for all devices
    pub fn devices(&self) -> &[DeviceInfo] {
        &self.inner.info
    }

    /// Acquire a device matching requirements
    ///
    /// Returns a DeviceLease that releases the device when dropped.
    pub async fn acquire(&self, requirements: &DeviceRequirements) -> Result<DeviceLease> {
        self.acquire_with_quota(requirements, None).await
    }

    /// Acquire a device with an optional quota tracker
    pub async fn acquire_with_quota(
        &self,
        requirements: &DeviceRequirements,
        quota: Option<ResourceQuota>,
    ) -> Result<DeviceLease> {
        // Acquire semaphore permit first (owned so it can be stored in DeviceLease)
        let permit = self
            .inner
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| BarracudaError::device(format!("Semaphore error: {e}")))?;

        // Lock for device selection to prevent race conditions
        let _lock = self.inner.selection_lock.lock().await;

        // Find best matching device
        let mut best_idx = None;
        let mut best_score = i64::MIN;

        for (i, info) in self.inner.info.iter().enumerate() {
            // Skip busy devices
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

        let idx = best_idx.ok_or_else(|| {
            BarracudaError::device_not_found("No device matches requirements")
        })?;

        // Mark device as busy
        self.inner.device_busy[idx].store(true, Ordering::Release);

        // Create quota tracker if quota provided
        let quota_tracker = quota.map(|q| Arc::new(QuotaTracker::new(q)));

        Ok(DeviceLease {
            device: self.inner.devices[idx].clone(),
            info: self.inner.info[idx].clone(),
            pool: self.inner.clone(),
            quota_tracker,
            _permit: permit,
        })
    }

    /// Acquire the first available device (no requirements)
    pub async fn acquire_any(&self) -> Result<DeviceLease> {
        self.acquire(&DeviceRequirements::new()).await
    }

    /// Get a specific device by index (for testing/debugging)
    pub fn device(&self, index: usize) -> Option<Arc<WgpuDevice>> {
        self.inner.devices.get(index).cloned()
    }

    /// Execute a closure on the best matching device
    pub async fn execute<F, T>(&self, requirements: &DeviceRequirements, f: F) -> Result<T>
    where
        F: FnOnce(Arc<WgpuDevice>) -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let lease = self.acquire(requirements).await?;
        let device = lease.device().clone();

        // Execute in blocking task
        tokio::task::spawn_blocking(move || f(device))
            .await
            .map_err(|e| BarracudaError::device(format!("Task error: {e}")))?
    }

    /// Get a summary of pool status
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

    /// Get detailed status of each device
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
            // Try to acquire with NVIDIA preference
            let reqs = DeviceRequirements::new().prefer_nvidia();

            if let Ok(lease) = pool.acquire(&reqs).await {
                println!("Acquired: {} ({:?})", lease.info().name, lease.info().vendor);
            }

            // Try to acquire with high VRAM requirement
            let reqs = DeviceRequirements::new().with_min_vram_gb(100); // Unrealistically high

            let result = pool.acquire(&reqs).await;
            assert!(result.is_err()); // Should fail - no device has 100 GB
        }
    }

    #[tokio::test]
    async fn test_device_lease_tracking() {
        let pool = MultiDevicePool::new().await;
        if let Ok(pool) = pool {
            let quota = ResourceQuota::new().with_max_vram_mb(100);

            if let Ok(lease) = pool.acquire_with_quota(&DeviceRequirements::new(), Some(quota)).await
            {
                // Track some allocations
                assert!(lease.track_allocation(50 * 1024 * 1024).is_ok()); // 50 MB
                assert!(lease.track_allocation(50 * 1024 * 1024).is_ok()); // 100 MB total

                // This should fail - exceeds quota
                assert!(lease.track_allocation(1).is_err());

                // Deallocate some
                lease.track_deallocation(50 * 1024 * 1024);

                // Now should succeed
                assert!(lease.track_allocation(1).is_ok());
            }
        }
    }

    #[test]
    fn test_device_requirements_scoring() {
        let reqs = DeviceRequirements::new()
            .prefer_nvidia()
            .with_min_vram_gb(8);

        // Create mock device info
        let nvidia_info = DeviceInfo {
            index: 0,
            pool_index: 0,
            name: "RTX 4070".to_string(),
            vendor: GpuVendor::Nvidia,
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
            vram_bytes: 16 * 1024 * 1024 * 1024,
            estimated_gflops: 4000.0,
            is_discrete: true,
            allocations: Arc::new(AtomicUsize::new(0)),
            allocated_bytes: Arc::new(AtomicU64::new(0)),
            busy: Arc::new(AtomicBool::new(false)),
        };

        let nvidia_score = reqs.score(&nvidia_info).unwrap();
        let amd_score = reqs.score(&amd_info).unwrap();

        // NVIDIA should score higher due to vendor preference
        assert!(nvidia_score > amd_score);
    }
}
