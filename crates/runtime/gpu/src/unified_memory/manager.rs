// SPDX-License-Identifier: AGPL-3.0-only
//! Unified memory manager - High-level API

use crate::unified_memory::{
    backend::{BackendInitializer, UnifiedMemoryBackend},
    buffer::UnifiedBuffer,
    types::{
        BackendStrategy, BackendType, BufferId, BufferIdGenerator, MemoryFlags,
        UnifiedBufferMetadata, UnifiedMemoryCapabilities, UnifiedMemoryConfig, UnifiedMemoryStats,
    },
};
use std::collections::HashMap;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU64, Ordering},
};
use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Universal unified memory manager
///
/// This is the main entry point for unified memory allocation and management.
/// It automatically selects the best available backend and provides a
/// high-level, async-native API for zero-copy GPU compute.
///
/// # Example
///
/// ```no_run
/// use toadstool_runtime_gpu::unified_memory::*;
///
/// # async fn example() -> toadstool::error::ToadStoolResult<()> {
/// // Initialize with automatic backend selection
/// let memory = UniversalUnifiedMemory::new().await?;
///
/// // Allocate unified buffer
/// let buffer = memory.allocate(4096).await?;
///
/// println!("Backend: {}", memory.backend_name());
/// println!("Allocated {} bytes", buffer.size());
/// # Ok(())
/// # }
/// ```
pub struct UniversalUnifiedMemory {
    /// Active backend
    backend: Arc<dyn UnifiedMemoryBackend>,

    /// Buffer ID generator
    id_generator: BufferIdGenerator,

    /// Active allocations tracking
    allocations: Arc<RwLock<HashMap<BufferId, UnifiedBufferMetadata>>>,

    /// Performance metrics
    metrics: Arc<RwLock<UnifiedMemoryStats>>,

    /// Configuration
    config: UnifiedMemoryConfig,

    /// Total allocated bytes (atomic)
    total_allocated: Arc<AtomicU64>,

    /// Peak allocated bytes (atomic)
    peak_allocated: Arc<AtomicU64>,
}

impl UniversalUnifiedMemory {
    /// Initialize with automatic backend selection (sovereignty-first)
    ///
    /// Priority order:
    /// 1. `WebGPU` (pure Rust, sovereign)
    /// 2. Vulkan (cross-vendor, modern)
    /// 3. `OpenCL` (cross-vendor, legacy)
    /// 4. CPU (always works)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use toadstool_runtime_gpu::unified_memory::*;
    /// # async fn example() -> toadstool::error::ToadStoolResult<()> {
    /// let memory = UniversalUnifiedMemory::new().await?;
    /// println!("Using backend: {}", memory.backend_name());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns when no suitable backend could be initialized.
    pub async fn new() -> ToadStoolResult<Self> {
        Self::with_strategy(BackendStrategy::Automatic).await
    }

    /// Initialize with specific backend strategy
    ///
    /// # Arguments
    ///
    /// * `strategy` - Backend selection strategy
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use toadstool_runtime_gpu::unified_memory::*;
    /// # async fn example() -> toadstool::error::ToadStoolResult<()> {
    /// // Force Vulkan backend
    /// let memory = UniversalUnifiedMemory::with_strategy(
    ///     BackendStrategy::Specific(BackendType::Vulkan)
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns when backend selection or initialization fails.
    pub async fn with_strategy(strategy: BackendStrategy) -> ToadStoolResult<Self> {
        let config = UnifiedMemoryConfig {
            backend_strategy: strategy.clone(),
            ..Default::default()
        };

        Self::with_config(config).await
    }

    /// Initialize with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Unified memory configuration
    ///
    /// # Errors
    ///
    /// Returns when backend selection or initialization fails.
    pub async fn with_config(config: UnifiedMemoryConfig) -> ToadStoolResult<Self> {
        let backend = Self::select_backend(&config.backend_strategy).await?;
        let backend_name = backend.name().to_string();

        tracing::info!(
            "✅ Initialized unified memory with {} backend",
            backend_name
        );

        Ok(Self {
            backend,
            id_generator: BufferIdGenerator::new(),
            allocations: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(UnifiedMemoryStats::new(backend_name))),
            config,
            total_allocated: Arc::new(AtomicU64::new(0)),
            peak_allocated: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Select backend based on strategy
    async fn select_backend(
        strategy: &BackendStrategy,
    ) -> ToadStoolResult<Arc<dyn UnifiedMemoryBackend>> {
        match strategy {
            BackendStrategy::Automatic => Self::select_automatic().await,
            BackendStrategy::SovereignOnly => Self::select_sovereign().await,
            BackendStrategy::Performance => Self::select_performance().await,
            BackendStrategy::Specific(backend_type) => Self::select_specific(*backend_type).await,
        }
    }

    /// Automatic selection (sovereignty-first)
    async fn select_automatic() -> ToadStoolResult<Arc<dyn UnifiedMemoryBackend>> {
        // Priority 1: WebGPU (sovereignty)
        #[cfg(feature = "webgpu")]
        {
            use crate::unified_memory::backends::webgpu::WebGpuBackend;
            if let Ok(backend) = WebGpuBackend::try_init().await {
                tracing::info!("🎯 Selected WebGPU backend (pure Rust, sovereign)");
                return Ok(Arc::new(backend));
            }
        }

        // Priority 2: Vulkan (universal, modern)
        #[cfg(feature = "vulkan")]
        {
            use crate::unified_memory::backends::vulkan::VulkanBackend;
            if let Ok(backend) = VulkanBackend::try_init().await {
                tracing::info!("🎯 Selected Vulkan backend (cross-vendor)");
                return Ok(Arc::new(backend));
            }
        }

        // Priority 3: OpenCL (universal, legacy)
        #[cfg(feature = "opencl")]
        {
            use crate::unified_memory::backends::opencl::OpenClBackend;
            if let Ok(backend) = OpenClBackend::try_init().await {
                tracing::info!("🎯 Selected OpenCL backend (cross-vendor)");
                return Ok(Arc::new(backend));
            }
        }

        // Priority 4: CPU (always works)
        use crate::unified_memory::backends::cpu::CpuBackend;
        let backend = CpuBackend::try_init().await?;
        tracing::warn!("⚠️  Using CPU backend (no GPU unified memory available)");
        Ok(Arc::new(backend))
    }

    /// Sovereign-only selection (`WebGPU` or fail)
    async fn select_sovereign() -> ToadStoolResult<Arc<dyn UnifiedMemoryBackend>> {
        #[cfg(feature = "webgpu")]
        {
            use crate::unified_memory::backends::webgpu::WebGpuBackend;
            let backend = WebGpuBackend::try_init().await?;
            Ok(Arc::new(backend))
        }

        #[cfg(not(feature = "webgpu"))]
        {
            Err(ToadStoolError::runtime(
                "WebGPU feature not enabled (sovereignty-only mode requires webgpu feature)",
            ))
        }
    }

    /// Performance selection (fastest first)
    async fn select_performance() -> ToadStoolResult<Arc<dyn UnifiedMemoryBackend>> {
        // Priority 1: Vulkan (fastest modern)
        #[cfg(feature = "vulkan")]
        {
            use crate::unified_memory::backends::vulkan::VulkanBackend;
            if let Ok(backend) = VulkanBackend::try_init().await {
                return Ok(Arc::new(backend));
            }
        }

        // Priority 2: OpenCL (legacy fast)
        #[cfg(feature = "opencl")]
        {
            use crate::unified_memory::backends::opencl::OpenClBackend;
            if let Ok(backend) = OpenClBackend::try_init().await {
                return Ok(Arc::new(backend));
            }
        }

        // Priority 3: WebGPU (pure Rust)
        #[cfg(feature = "webgpu")]
        {
            use crate::unified_memory::backends::webgpu::WebGpuBackend;
            if let Ok(backend) = WebGpuBackend::try_init().await {
                return Ok(Arc::new(backend));
            }
        }

        // Fallback: CPU
        use crate::unified_memory::backends::cpu::CpuBackend;
        let backend = CpuBackend::try_init().await?;
        Ok(Arc::new(backend))
    }

    /// Select specific backend
    async fn select_specific(
        backend_type: BackendType,
    ) -> ToadStoolResult<Arc<dyn UnifiedMemoryBackend>> {
        match backend_type {
            BackendType::Vulkan => {
                #[cfg(feature = "vulkan")]
                {
                    use crate::unified_memory::backends::vulkan::VulkanBackend;
                    let backend = VulkanBackend::try_init().await?;
                    Ok(Arc::new(backend))
                }
                #[cfg(not(feature = "vulkan"))]
                {
                    Err(ToadStoolError::runtime("Vulkan feature not enabled"))
                }
            }
            BackendType::OpenCL => {
                #[cfg(feature = "opencl")]
                {
                    use crate::unified_memory::backends::opencl::OpenClBackend;
                    let backend = OpenClBackend::try_init().await?;
                    Ok(Arc::new(backend))
                }
                #[cfg(not(feature = "opencl"))]
                {
                    Err(ToadStoolError::runtime("OpenCL feature not enabled"))
                }
            }
            BackendType::WebGpu => {
                #[cfg(feature = "webgpu")]
                {
                    use crate::unified_memory::backends::webgpu::WebGpuBackend;
                    let backend = WebGpuBackend::try_init().await?;
                    Ok(Arc::new(backend))
                }
                #[cfg(not(feature = "webgpu"))]
                {
                    Err(ToadStoolError::runtime("WebGPU feature not enabled"))
                }
            }
            BackendType::Cpu => {
                use crate::unified_memory::backends::cpu::CpuBackend;
                let backend = CpuBackend::try_init().await?;
                Ok(Arc::new(backend))
            }
        }
    }

    /// Allocate unified buffer with default flags
    ///
    /// # Arguments
    ///
    /// * `size` - Size in bytes
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use toadstool_runtime_gpu::unified_memory::*;
    /// # async fn example(memory: &UniversalUnifiedMemory) -> toadstool::error::ToadStoolResult<()> {
    /// let buffer = memory.allocate(4096).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns when size is invalid or backend allocation fails.
    pub async fn allocate(&self, size: usize) -> ToadStoolResult<UnifiedBuffer> {
        self.allocate_with_flags(size, self.config.default_flags)
            .await
    }

    /// Allocate unified buffer with specific flags
    ///
    /// # Arguments
    ///
    /// * `size` - Size in bytes
    /// * `flags` - Memory allocation flags
    ///
    /// # Errors
    ///
    /// Returns when size is invalid, exceeds backend limits, or allocation fails.
    pub async fn allocate_with_flags(
        &self,
        size: usize,
        flags: MemoryFlags,
    ) -> ToadStoolResult<UnifiedBuffer> {
        // Validate size
        if size == 0 {
            return Err(ToadStoolError::runtime("Cannot allocate 0 bytes"));
        }

        let caps = self.backend.capabilities();
        if size > caps.max_allocation_size {
            return Err(ToadStoolError::runtime(format!(
                "Allocation size {} exceeds backend maximum {}",
                size, caps.max_allocation_size
            )));
        }

        // Generate buffer ID
        let id = self.id_generator.next();

        // Allocate via backend
        let allocation = self.backend.allocate_unified(size, flags).await?;

        // Get CPU pointer
        let cpu_ptr = self.backend.map_cpu_ptr(&allocation).await?;

        // Get device pointer
        let device_ptr = self.backend.get_device_ptr(&allocation);

        // Track allocation
        let metadata = UnifiedBufferMetadata::new(id, size, flags);
        self.allocations
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(id, metadata);

        // Update metrics
        let prev = self
            .total_allocated
            .fetch_add(size as u64, Ordering::Relaxed);
        let new_total = prev + size as u64;

        // Update peak (lock-free)
        let mut peak = self.peak_allocated.load(Ordering::Relaxed);
        while new_total > peak {
            match self.peak_allocated.compare_exchange_weak(
                peak,
                new_total,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(current) => peak = current,
            }
        }

        // Update stats
        if self.config.enable_metrics {
            let mut stats = self
                .metrics
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            stats.allocation_count += 1;
            stats.active_allocations += 1;
            stats.total_allocated = new_total;
            stats.update_peak(new_total);
        }

        tracing::debug!("Allocated buffer {} ({} bytes)", id, size);

        Ok(UnifiedBuffer::new(
            id,
            size,
            cpu_ptr,
            device_ptr,
            allocation,
            Arc::clone(&self.backend),
            Arc::clone(&self.allocations),
            Arc::clone(&self.total_allocated),
            Arc::clone(&self.metrics),
        ))
    }

    /// Get backend name
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    /// Get backend type
    pub fn backend_type(&self) -> BackendType {
        self.backend.backend_type()
    }

    /// Get backend capabilities
    pub fn capabilities(&self) -> &UnifiedMemoryCapabilities {
        self.backend.capabilities()
    }

    /// Get current statistics
    pub fn stats(&self) -> UnifiedMemoryStats {
        self.metrics
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Get number of active allocations
    pub fn active_allocations(&self) -> usize {
        self.allocations
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    /// Get total allocated bytes
    pub fn total_allocated(&self) -> u64 {
        self.total_allocated.load(Ordering::Relaxed)
    }

    /// Get peak allocated bytes
    pub fn peak_allocated(&self) -> u64 {
        self.peak_allocated.load(Ordering::Relaxed)
    }
}

// Implement Clone for Arc-based sharing
impl Clone for UniversalUnifiedMemory {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
            id_generator: BufferIdGenerator::new(), // New generator for cloned instance
            allocations: Arc::clone(&self.allocations),
            metrics: Arc::clone(&self.metrics),
            config: self.config.clone(),
            total_allocated: Arc::clone(&self.total_allocated),
            peak_allocated: Arc::clone(&self.peak_allocated),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_initialization() {
        // CPU backend should always work
        let memory =
            UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
                .await;

        assert!(memory.is_ok());
        let memory = memory.unwrap();
        assert_eq!(memory.backend_type(), BackendType::Cpu);
        assert_eq!(memory.backend_name(), "CPU");
    }

    #[tokio::test]
    async fn test_allocation_validation() {
        let memory =
            UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
                .await
                .unwrap();

        // Zero size should fail
        let result = memory.allocate(0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let memory =
            UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
                .await
                .unwrap();

        assert_eq!(memory.total_allocated(), 0);
        assert_eq!(memory.active_allocations(), 0);

        let _buffer1 = memory.allocate(1024).await.unwrap();
        assert_eq!(memory.total_allocated(), 1024);
        assert_eq!(memory.active_allocations(), 1);

        let _buffer2 = memory.allocate(2048).await.unwrap();
        assert_eq!(memory.total_allocated(), 3072);
        assert_eq!(memory.active_allocations(), 2);
    }
}
