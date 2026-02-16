# Universal Unified Memory Architecture Specification

**Date**: January 2, 2026  
**Version**: 1.0.0  
**Status**: 🎯 SPECIFICATION - Ready for Implementation  
**Philosophy**: Vendor-agnostic, sovereignty-first, zero-copy compute

---

## 🎯 Executive Summary

### Mission

Provide a **pure Rust, vendor-agnostic unified memory abstraction** that enables zero-copy compute across Intel, AMD, and NVIDIA GPUs through open standards (Vulkan, OpenCL, WebGPU).

### Core Principles

1. **No Vendor Lock-in**: Works on Intel iGPU, AMD APU, NVIDIA discrete via universal APIs
2. **Sovereignty First**: WebGPU (pure Rust) is primary, vendor APIs are fallbacks
3. **Zero-Copy Native**: Unified memory eliminates CPU↔GPU transfers
4. **Deep Solutions**: No technical debt, async-native, fully concurrent
5. **Self-Knowledge**: Runtime detection and capability-based selection

### Key Innovation

**ToadStool can run CUDA workloads on AMD/Intel GPUs** through our kernel translation layer + unified memory abstraction.

---

## 🏗️ Architecture Overview

### Layered Design

```
┌───────────────────────────────────────────────────────────────┐
│              ToadStool Application Layer                      │
│  (User writes once, runs on Intel/AMD/NVIDIA/Apple)          │
└───────────────────────────────────────────────────────────────┘
                              ▼
┌───────────────────────────────────────────────────────────────┐
│         Universal Unified Memory API (Pure Rust)              │
│  • UniversalUnifiedMemory: High-level async interface        │
│  • UnifiedBuffer: Zero-copy CPU/GPU buffer                   │
│  • MemoryRegion: Typed memory views                          │
│  • AutoSync: Smart synchronization                           │
└───────────────────────────────────────────────────────────────┘
                              ▼
┌───────────────────────────────────────────────────────────────┐
│            Backend Abstraction Layer (Trait-Based)            │
│  Trait: UnifiedMemoryBackend                                  │
│    • allocate_unified()                                       │
│    • map_cpu_ptr()                                            │
│    • get_device_ptr()                                         │
│    • sync_cpu_to_device()                                     │
│    • sync_device_to_cpu()                                     │
└───────────────────────────────────────────────────────────────┘
                              ▼
┌─────────────┬─────────────┬─────────────┬─────────────────────┐
│   Vulkan    │   OpenCL    │   WebGPU    │   CPU Fallback      │
│   Backend   │   Backend   │   Backend   │   (memmap2)         │
│             │             │             │                     │
│ Intel ✅    │ Intel ✅    │ All ✅      │ Always ✅           │
│ AMD ✅      │ AMD ✅      │             │                     │
│ NVIDIA ✅   │ NVIDIA ✅   │             │                     │
└─────────────┴─────────────┴─────────────┴─────────────────────┘
                              ▼
┌───────────────────────────────────────────────────────────────┐
│                 Hardware Layer                                │
│  Intel iGPU  │  AMD APU  │  NVIDIA GPU  │  Apple Metal       │
└───────────────────────────────────────────────────────────────┘
```

---

## 📐 Technical Specification

### Core Types

#### 1. UniversalUnifiedMemory (Main API)

**Purpose**: Primary interface for unified memory allocation and management

**Design**: Async-native, concurrent, zero-copy

```rust
/// Universal unified memory manager (vendor-agnostic)
pub struct UniversalUnifiedMemory {
    /// Active backend (Vulkan, OpenCL, WebGPU, or CPU)
    backend: Arc<dyn UnifiedMemoryBackend>,
    
    /// Memory allocations tracker
    allocations: Arc<DashMap<BufferId, UnifiedBufferMetadata>>,
    
    /// Performance metrics
    metrics: Arc<RwLock<UnifiedMemoryMetrics>>,
    
    /// Configuration
    config: UnifiedMemoryConfig,
}

impl UniversalUnifiedMemory {
    /// Initialize with automatic backend selection (sovereignty-first)
    pub async fn new() -> ToadStoolResult<Self>;
    
    /// Initialize with specific strategy
    pub async fn with_strategy(strategy: BackendStrategy) -> ToadStoolResult<Self>;
    
    /// Allocate unified buffer (accessible from CPU and GPU)
    pub async fn allocate(&self, size: usize) -> ToadStoolResult<UnifiedBuffer>;
    
    /// Allocate with specific memory flags
    pub async fn allocate_with_flags(
        &self,
        size: usize,
        flags: MemoryFlags,
    ) -> ToadStoolResult<UnifiedBuffer>;
    
    /// Get memory statistics
    pub async fn get_stats(&self) -> UnifiedMemoryStats;
    
    /// Optimize memory layout for workload
    pub async fn optimize_for(&self, workload_type: WorkloadType) -> ToadStoolResult<()>;
}
```

#### 2. UnifiedBuffer (Zero-Copy Buffer)

**Purpose**: CPU/GPU accessible buffer with automatic synchronization

**Design**: Safe Rust wrapper over unified memory pointer

```rust
/// Zero-copy buffer accessible from both CPU and GPU
pub struct UnifiedBuffer {
    /// Buffer ID
    id: BufferId,
    
    /// Size in bytes
    size: usize,
    
    /// CPU-accessible pointer (safe wrapper)
    cpu_view: Arc<RwLock<CpuMemoryView>>,
    
    /// GPU device pointer (opaque handle)
    gpu_handle: GpuMemoryHandle,
    
    /// Backend reference
    backend: Arc<dyn UnifiedMemoryBackend>,
    
    /// Synchronization state
    sync_state: Arc<AtomicSyncState>,
}

impl UnifiedBuffer {
    /// Write data from CPU (async, no blocking)
    pub async fn write_async(&mut self, offset: usize, data: &[u8]) -> ToadStoolResult<()>;
    
    /// Read data to CPU (async, no blocking)
    pub async fn read_async(&self, offset: usize, len: usize) -> ToadStoolResult<Vec<u8>>;
    
    /// Get typed view (zero-copy)
    pub fn view<T: Pod>(&self) -> ToadStoolResult<TypedView<T>>;
    
    /// Get mutable typed view (zero-copy)
    pub fn view_mut<T: Pod>(&mut self) -> ToadStoolResult<TypedViewMut<T>>;
    
    /// Get GPU device pointer for kernel execution
    pub fn device_ptr(&self) -> *const u8;
    
    /// Ensure CPU changes are visible to GPU
    pub async fn sync_to_device(&self) -> ToadStoolResult<()>;
    
    /// Ensure GPU changes are visible to CPU
    pub async fn sync_to_cpu(&self) -> ToadStoolResult<()>;
    
    /// Get current synchronization state
    pub fn sync_state(&self) -> SyncState;
}
```

#### 3. UnifiedMemoryBackend (Trait)

**Purpose**: Abstraction over vendor-specific unified memory APIs

**Design**: Async trait with default implementations

```rust
#[async_trait]
pub trait UnifiedMemoryBackend: Send + Sync {
    /// Backend name
    fn name(&self) -> &'static str;
    
    /// Allocate unified memory
    async fn allocate_unified(
        &self,
        size: usize,
        flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation>;
    
    /// Free unified memory
    async fn free_unified(&self, allocation: BackendAllocation) -> ToadStoolResult<()>;
    
    /// Map to CPU pointer
    async fn map_cpu_ptr(
        &self,
        allocation: &BackendAllocation,
    ) -> ToadStoolResult<*mut u8>;
    
    /// Unmap CPU pointer
    async fn unmap_cpu_ptr(&self, allocation: &BackendAllocation) -> ToadStoolResult<()>;
    
    /// Get GPU device pointer
    fn get_device_ptr(&self, allocation: &BackendAllocation) -> *const u8;
    
    /// Synchronize CPU → GPU (if needed)
    async fn sync_cpu_to_device(
        &self,
        allocation: &BackendAllocation,
    ) -> ToadStoolResult<()>;
    
    /// Synchronize GPU → CPU (if needed)
    async fn sync_device_to_cpu(
        &self,
        allocation: &BackendAllocation,
    ) -> ToadStoolResult<()>;
    
    /// Query backend capabilities
    fn capabilities(&self) -> &UnifiedMemoryCapabilities;
    
    /// Optimize for specific access pattern
    async fn optimize_for_pattern(
        &self,
        allocation: &BackendAllocation,
        pattern: AccessPattern,
    ) -> ToadStoolResult<()>;
}
```

---

## 🔧 Backend Implementations

### 1. Vulkan Backend (Priority 1 - Universal)

**Rationale**: Works on Intel, AMD, NVIDIA, cross-platform, modern API

**Implementation**: `crates/runtime/gpu/src/unified_memory/backends/vulkan.rs`

```rust
/// Vulkan unified memory backend (works on all vendors)
pub struct VulkanUnifiedBackend {
    /// Vulkan instance
    instance: Arc<ash::Instance>,
    
    /// Physical device
    physical_device: vk::PhysicalDevice,
    
    /// Logical device
    device: Arc<ash::Device>,
    
    /// Unified memory type index
    unified_memory_type: u32,
    
    /// Capabilities
    capabilities: UnifiedMemoryCapabilities,
    
    /// Allocator for memory management
    allocator: Arc<Mutex<VulkanAllocator>>,
}

impl VulkanUnifiedBackend {
    /// Initialize Vulkan unified memory
    pub async fn new() -> ToadStoolResult<Self> {
        // 1. Create Vulkan instance
        let entry = ash::Entry::linked();
        let instance = Self::create_instance(&entry).await?;
        
        // 2. Select physical device
        let (physical_device, queue_family_index) = 
            Self::select_device(&instance).await?;
        
        // 3. Query memory properties
        let mem_props = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };
        
        // 4. Find unified memory type
        let unified_memory_type = Self::find_unified_memory_type(&mem_props)?;
        
        // 5. Create logical device
        let device = Self::create_device(&instance, physical_device, queue_family_index).await?;
        
        // 6. Query capabilities
        let capabilities = Self::query_capabilities(&instance, physical_device, &mem_props)?;
        
        Ok(Self {
            instance: Arc::new(instance),
            physical_device,
            device: Arc::new(device),
            unified_memory_type,
            capabilities,
            allocator: Arc::new(Mutex::new(VulkanAllocator::new())),
        })
    }
    
    /// Find memory type with HOST_VISIBLE + DEVICE_LOCAL flags
    /// This is unified memory that works across vendors!
    fn find_unified_memory_type(
        props: &vk::PhysicalDeviceMemoryProperties,
    ) -> ToadStoolResult<u32> {
        // Look for unified memory: HOST_VISIBLE + DEVICE_LOCAL
        for i in 0..props.memory_type_count {
            let memory_type = &props.memory_types[i as usize];
            
            if memory_type.property_flags.contains(
                vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::DEVICE_LOCAL
            ) {
                return Ok(i);
            }
        }
        
        // Fallback: HOST_VISIBLE + HOST_COHERENT (slower but works)
        for i in 0..props.memory_type_count {
            let memory_type = &props.memory_types[i as usize];
            
            if memory_type.property_flags.contains(
                vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT
            ) {
                tracing::warn!("Using fallback unified memory (may be slower)");
                return Ok(i);
            }
        }
        
        Err(ToadStoolError::runtime(
            "No unified memory type available on this device"
        ))
    }
}

#[async_trait]
impl UnifiedMemoryBackend for VulkanUnifiedBackend {
    fn name(&self) -> &'static str {
        "Vulkan"
    }
    
    async fn allocate_unified(
        &self,
        size: usize,
        _flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation> {
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(size as u64)
            .memory_type_index(self.unified_memory_type);
        
        let memory = unsafe {
            self.device
                .allocate_memory(&alloc_info, None)
                .map_err(|e| ToadStoolError::runtime(
                    format!("Vulkan allocation failed: {}", e)
                ))?
        };
        
        // Map memory immediately (persistent mapping)
        let cpu_ptr = unsafe {
            self.device
                .map_memory(memory, 0, size as u64, vk::MemoryMapFlags::empty())
                .map_err(|e| ToadStoolError::runtime(
                    format!("Vulkan map failed: {}", e)
                ))?
        };
        
        Ok(BackendAllocation::Vulkan(VulkanAllocation {
            memory,
            size,
            cpu_ptr: cpu_ptr as *mut u8,
        }))
    }
    
    // ... other trait implementations
}
```

### 2. OpenCL Backend (Priority 2 - Legacy Universal)

**Rationale**: Works on Intel, AMD, NVIDIA, widely supported, legacy compatibility

**Implementation**: `crates/runtime/gpu/src/unified_memory/backends/opencl.rs`

```rust
/// OpenCL SVM (Shared Virtual Memory) backend
pub struct OpenClSvmBackend {
    /// OpenCL context
    context: ocl::Context,
    
    /// OpenCL device
    device: ocl::Device,
    
    /// SVM capabilities
    svm_capabilities: SvmCapabilities,
    
    /// Memory allocations
    allocations: Arc<DashMap<u64, OpenClSvmAllocation>>,
}

impl OpenClSvmBackend {
    /// Initialize OpenCL SVM backend
    pub async fn new() -> ToadStoolResult<Self> {
        // 1. Get platform and device
        let platform = ocl::Platform::default();
        let device = ocl::Device::first(platform)
            .map_err(|e| ToadStoolError::runtime(
                format!("No OpenCL device found: {}", e)
            ))?;
        
        // 2. Check SVM support (OpenCL 2.0+)
        let svm_capabilities = Self::query_svm_capabilities(&device)?;
        
        if !svm_capabilities.has_svm {
            return Err(ToadStoolError::runtime(
                "Device does not support OpenCL SVM (requires OpenCL 2.0+)"
            ));
        }
        
        // 3. Create context
        let context = ocl::Context::builder()
            .platform(platform)
            .devices(device)
            .build()
            .map_err(|e| ToadStoolError::runtime(
                format!("Failed to create OpenCL context: {}", e)
            ))?;
        
        Ok(Self {
            context,
            device,
            svm_capabilities,
            allocations: Arc::new(DashMap::new()),
        })
    }
    
    /// Query SVM capabilities
    fn query_svm_capabilities(device: &ocl::Device) -> ToadStoolResult<SvmCapabilities> {
        let version = device.version()
            .map_err(|e| ToadStoolError::runtime(
                format!("Failed to query OpenCL version: {}", e)
            ))?;
        
        // SVM requires OpenCL 2.0+
        let has_svm = version.major() >= 2;
        
        Ok(SvmCapabilities {
            has_svm,
            fine_grain_buffer: has_svm,
            fine_grain_system: false, // Conservative default
            atomics: has_svm,
        })
    }
}

#[async_trait]
impl UnifiedMemoryBackend for OpenClSvmBackend {
    fn name(&self) -> &'static str {
        "OpenCL SVM"
    }
    
    async fn allocate_unified(
        &self,
        size: usize,
        flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation> {
        // Allocate SVM buffer
        let svm_flags = Self::convert_flags(flags);
        
        let svm_ptr = unsafe {
            // OpenCL SVM allocation (unified memory)
            ocl::ffi::clSVMAlloc(
                self.context.as_ptr(),
                svm_flags,
                size,
                0, // alignment (0 = default)
            )
        };
        
        if svm_ptr.is_null() {
            return Err(ToadStoolError::runtime("OpenCL SVM allocation failed"));
        }
        
        let allocation = OpenClSvmAllocation {
            ptr: svm_ptr as *mut u8,
            size,
            context: self.context.clone(),
        };
        
        Ok(BackendAllocation::OpenCL(allocation))
    }
    
    // ... other trait implementations
}
```

### 3. WebGPU Backend (Priority 0 - Future Primary)

**Rationale**: Pure Rust, vendor-agnostic, future of GPU compute

**Implementation**: `crates/runtime/gpu/src/unified_memory/backends/webgpu.rs`

```rust
/// WebGPU unified memory backend (pure Rust)
pub struct WebGpuUnifiedBackend {
    /// WebGPU adapter
    adapter: Arc<wgpu::Adapter>,
    
    /// WebGPU device
    device: Arc<wgpu::Device>,
    
    /// Command queue
    queue: Arc<wgpu::Queue>,
    
    /// Capabilities
    capabilities: UnifiedMemoryCapabilities,
}

impl WebGpuUnifiedBackend {
    /// Initialize WebGPU backend
    pub async fn new() -> ToadStoolResult<Self> {
        // 1. Create instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // 2. Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| ToadStoolError::runtime("No WebGPU adapter available"))?;
        
        // 3. Request device
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("ToadStool WebGPU Device"),
                    required_features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .map_err(|e| ToadStoolError::runtime(
                format!("Failed to create WebGPU device: {}", e)
            ))?;
        
        Ok(Self {
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
            capabilities: Self::query_capabilities(&device),
        })
    }
}

#[async_trait]
impl UnifiedMemoryBackend for WebGpuUnifiedBackend {
    fn name(&self) -> &'static str {
        "WebGPU"
    }
    
    async fn allocate_unified(
        &self,
        size: usize,
        _flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation> {
        // Create mappable buffer
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Unified Memory Buffer"),
            size: size as u64,
            usage: wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        });
        
        Ok(BackendAllocation::WebGpu(WebGpuAllocation {
            buffer: Arc::new(buffer),
            size,
        }))
    }
    
    // ... other trait implementations
}
```

### 4. CPU Fallback Backend (Always Available)

**Rationale**: Always works, development/testing, graceful degradation

**Implementation**: `crates/runtime/gpu/src/unified_memory/backends/cpu.rs`

```rust
/// CPU shared memory fallback (always available)
pub struct CpuSharedBackend {
    /// Memory allocations using memmap2
    allocations: Arc<DashMap<u64, CpuAllocation>>,
}

impl CpuSharedBackend {
    pub fn new() -> ToadStoolResult<Self> {
        Ok(Self {
            allocations: Arc::new(DashMap::new()),
        })
    }
}

#[async_trait]
impl UnifiedMemoryBackend for CpuSharedBackend {
    fn name(&self) -> &'static str {
        "CPU Shared Memory"
    }
    
    async fn allocate_unified(
        &self,
        size: usize,
        _flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation> {
        // Use memmap2 for shared memory
        let mmap = memmap2::MmapMut::map_anon(size)
            .map_err(|e| ToadStoolError::runtime(
                format!("Failed to allocate CPU memory: {}", e)
            ))?;
        
        Ok(BackendAllocation::Cpu(CpuAllocation {
            mmap: Arc::new(Mutex::new(mmap)),
            size,
        }))
    }
    
    // ... other trait implementations (no-op sync)
}
```

---

## 🔄 Backend Selection Strategy

### Selection Priority

```rust
pub enum BackendStrategy {
    /// Automatic: Sovereignty-first, then performance
    /// Priority: WebGPU > Vulkan > OpenCL > CPU
    Automatic,
    
    /// Sovereignty only: WebGPU or fail
    SovereignOnly,
    
    /// Performance: Prefer fastest backend
    /// Priority: Vulkan > OpenCL > WebGPU > CPU
    Performance,
    
    /// Specific backend
    Specific(BackendType),
}

impl UniversalUnifiedMemory {
    async fn select_backend(strategy: BackendStrategy) -> ToadStoolResult<Arc<dyn UnifiedMemoryBackend>> {
        match strategy {
            BackendStrategy::Automatic => {
                // Priority 1: WebGPU (sovereignty)
                #[cfg(feature = "webgpu")]
                if let Ok(backend) = WebGpuUnifiedBackend::new().await {
                    info!("✅ Using WebGPU unified memory (pure Rust)");
                    return Ok(Arc::new(backend));
                }
                
                // Priority 2: Vulkan (universal, modern)
                if let Ok(backend) = VulkanUnifiedBackend::new().await {
                    info!("✅ Using Vulkan unified memory (cross-vendor)");
                    return Ok(Arc::new(backend));
                }
                
                // Priority 3: OpenCL SVM (universal, legacy)
                if let Ok(backend) = OpenClSvmBackend::new().await {
                    info!("✅ Using OpenCL SVM (cross-vendor)");
                    return Ok(Arc::new(backend));
                }
                
                // Fallback: CPU (always works)
                info!("⚠️  Using CPU shared memory (no GPU unified memory available)");
                Ok(Arc::new(CpuSharedBackend::new()?))
            }
            
            BackendStrategy::SovereignOnly => {
                #[cfg(feature = "webgpu")]
                {
                    let backend = WebGpuUnifiedBackend::new().await?;
                    Ok(Arc::new(backend))
                }
                #[cfg(not(feature = "webgpu"))]
                {
                    Err(ToadStoolError::runtime(
                        "WebGPU feature not enabled (sovereignty-only mode)"
                    ))
                }
            }
            
            BackendStrategy::Performance => {
                // Try backends in performance order
                if let Ok(backend) = VulkanUnifiedBackend::new().await {
                    return Ok(Arc::new(backend));
                }
                if let Ok(backend) = OpenClSvmBackend::new().await {
                    return Ok(Arc::new(backend));
                }
                #[cfg(feature = "webgpu")]
                if let Ok(backend) = WebGpuUnifiedBackend::new().await {
                    return Ok(Arc::new(backend));
                }
                Ok(Arc::new(CpuSharedBackend::new()?))
            }
            
            BackendStrategy::Specific(backend_type) => {
                Self::create_specific_backend(backend_type).await
            }
        }
    }
}
```

---

## 🚀 Async-Native Design

### Concurrency Model

**All operations are async and concurrent-safe:**

```rust
/// Example: Concurrent buffer operations
async fn concurrent_unified_memory_example() -> ToadStoolResult<()> {
    let memory = UniversalUnifiedMemory::new().await?;
    
    // Allocate multiple buffers concurrently
    let buffers = futures::future::try_join_all(
        (0..10).map(|i| {
            let mem = memory.clone();
            async move {
                mem.allocate(1024 * 1024 * i).await
            }
        })
    ).await?;
    
    // Write to buffers concurrently
    futures::future::try_join_all(
        buffers.iter().enumerate().map(|(i, buffer)| async move {
            let data = vec![i as u8; 1024];
            buffer.write_async(0, &data).await
        })
    ).await?;
    
    // Sync all buffers to GPU concurrently
    futures::future::try_join_all(
        buffers.iter().map(|buffer| buffer.sync_to_device())
    ).await?;
    
    Ok(())
}
```

### Lock-Free Data Structures

**Use modern concurrent Rust patterns:**

```rust
use dashmap::DashMap;
use parking_lot::{RwLock, Mutex};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

/// Lock-free allocation tracker
pub struct AllocationTracker {
    allocations: Arc<DashMap<BufferId, UnifiedBufferMetadata>>,
    total_allocated: AtomicU64,
    peak_allocated: AtomicU64,
}

impl AllocationTracker {
    pub fn track(&self, id: BufferId, size: usize, metadata: UnifiedBufferMetadata) {
        self.allocations.insert(id, metadata);
        
        // Atomic updates (lock-free)
        let prev = self.total_allocated.fetch_add(size as u64, Ordering::Relaxed);
        let new_total = prev + size as u64;
        
        // Update peak (lock-free compare-exchange loop)
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
    }
}
```

---

## 🛡️ Safety & Error Handling

### Zero Unwraps in Production

**All errors are properly handled:**

```rust
impl UnifiedBuffer {
    /// Safe write with proper error handling
    pub async fn write_async(&mut self, offset: usize, data: &[u8]) -> ToadStoolResult<()> {
        // Validate bounds
        if offset + data.len() > self.size {
            return Err(ToadStoolError::invalid_input(
                format!(
                    "Write would overflow buffer: offset={}, len={}, size={}",
                    offset, data.len(), self.size
                )
            ));
        }
        
        // Acquire CPU view lock
        let mut cpu_view = self.cpu_view.write().await;
        
        // Validate pointer is still valid
        if cpu_view.ptr.is_null() {
            return Err(ToadStoolError::runtime("Buffer has been freed"));
        }
        
        // SAFETY: Pointer validated, bounds checked, write lock held
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                cpu_view.ptr.add(offset),
                data.len(),
            );
        }
        
        // Update sync state
        self.sync_state.mark_cpu_modified();
        
        Ok(())
    }
}
```

### Comprehensive Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum UnifiedMemoryError {
    #[error("Backend initialization failed: {0}")]
    BackendInitFailed(String),
    
    #[error("Allocation failed: size={size}, reason={reason}")]
    AllocationFailed { size: usize, reason: String },
    
    #[error("Invalid buffer access: {0}")]
    InvalidAccess(String),
    
    #[error("Synchronization failed: {0}")]
    SyncFailed(String),
    
    #[error("Backend not supported: {backend}")]
    BackendNotSupported { backend: String },
}
```

---

## 📊 Performance Optimization

### Smart Synchronization

**Only sync when necessary:**

```rust
#[derive(Debug, Clone, Copy)]
pub enum SyncState {
    /// Buffer is synchronized
    Synced,
    
    /// CPU has modifications, GPU needs sync
    CpuModified,
    
    /// GPU has modifications, CPU needs sync
    GpuModified,
    
    /// Both modified (conflict - resolve with strategy)
    Conflict,
}

impl UnifiedBuffer {
    /// Auto-sync: Only synchronize if needed
    pub async fn auto_sync(&self, target: SyncTarget) -> ToadStoolResult<()> {
        match (self.sync_state(), target) {
            (SyncState::Synced, _) => {
                // Already synced, no-op
                Ok(())
            }
            (SyncState::CpuModified, SyncTarget::Device) => {
                self.sync_to_device().await
            }
            (SyncState::GpuModified, SyncTarget::Cpu) => {
                self.sync_to_cpu().await
            }
            (SyncState::Conflict, _) => {
                // Apply conflict resolution strategy
                self.resolve_conflict(target).await
            }
            _ => Ok(()), // No sync needed
        }
    }
}
```

### Memory Pooling

**Reduce allocation overhead:**

```rust
/// Memory pool for frequent allocations
pub struct UnifiedMemoryPool {
    backend: Arc<dyn UnifiedMemoryBackend>,
    
    /// Pools by size class (64B, 256B, 1KB, 4KB, etc.)
    pools: Vec<Pool>,
    
    /// Large allocation cache
    large_cache: Arc<DashMap<usize, Vec<UnifiedBuffer>>>,
}

impl UnifiedMemoryPool {
    /// Allocate from pool (fast path)
    pub async fn allocate(&self, size: usize) -> ToadStoolResult<UnifiedBuffer> {
        let size_class = Self::size_to_class(size);
        
        // Try to get from pool first
        if let Some(buffer) = self.pools[size_class].try_pop() {
            return Ok(buffer);
        }
        
        // Pool empty, allocate new
        self.backend.allocate_unified(size, MemoryFlags::default()).await
    }
    
    /// Return buffer to pool
    pub async fn release(&self, buffer: UnifiedBuffer) {
        let size_class = Self::size_to_class(buffer.size());
        self.pools[size_class].push(buffer);
    }
}
```

---

## 🧪 Testing Strategy

### Multi-Backend Testing

**Test on all backends:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_unified_memory_all_backends() {
        let backends = vec![
            BackendStrategy::Specific(BackendType::Vulkan),
            BackendStrategy::Specific(BackendType::OpenCL),
            #[cfg(feature = "webgpu")]
            BackendStrategy::Specific(BackendType::WebGpu),
            BackendStrategy::Specific(BackendType::Cpu),
        ];
        
        for strategy in backends {
            if let Ok(memory) = UniversalUnifiedMemory::with_strategy(strategy).await {
                test_backend(&memory).await;
            }
        }
    }
    
    async fn test_backend(memory: &UniversalUnifiedMemory) {
        // Allocate buffer
        let mut buffer = memory.allocate(4096).await.unwrap();
        
        // Write from CPU
        let data = vec![42u8; 1024];
        buffer.write_async(0, &data).await.unwrap();
        
        // Sync to GPU
        buffer.sync_to_device().await.unwrap();
        
        // Verify GPU pointer is valid
        assert!(!buffer.device_ptr().is_null());
        
        // Sync back to CPU
        buffer.sync_to_cpu().await.unwrap();
        
        // Read from CPU
        let read_data = buffer.read_async(0, 1024).await.unwrap();
        assert_eq!(data, read_data);
    }
}
```

### Concurrent Safety Tests

```rust
#[tokio::test]
async fn test_concurrent_allocations() {
    let memory = UniversalUnifiedMemory::new().await.unwrap();
    
    // Spawn 100 concurrent allocations
    let handles: Vec<_> = (0..100).map(|i| {
        let mem = memory.clone();
        tokio::spawn(async move {
            mem.allocate(1024 * i).await
        })
    }).collect();
    
    // All should succeed
    let results = futures::future::join_all(handles).await;
    assert!(results.iter().all(|r| r.is_ok()));
}
```

---

## 📈 Metrics & Monitoring

### Runtime Metrics

```rust
#[derive(Debug, Clone)]
pub struct UnifiedMemoryMetrics {
    /// Total bytes allocated
    pub total_allocated: u64,
    
    /// Peak allocation
    pub peak_allocated: u64,
    
    /// Number of allocations
    pub allocation_count: u64,
    
    /// Number of syncs (CPU → GPU)
    pub cpu_to_gpu_syncs: u64,
    
    /// Number of syncs (GPU → CPU)
    pub gpu_to_cpu_syncs: u64,
    
    /// Bytes synced
    pub bytes_synced: u64,
    
    /// Average sync latency
    pub avg_sync_latency_us: f64,
    
    /// Backend in use
    pub backend: String,
    
    /// Pool hit rate
    pub pool_hit_rate: f64,
}

impl UniversalUnifiedMemory {
    /// Get real-time metrics
    pub async fn metrics(&self) -> UnifiedMemoryMetrics {
        self.metrics.read().await.clone()
    }
}
```

---

## 🔗 Integration with Existing ToadStool

### Update MemoryCapabilities Detection

```rust
// crates/runtime/gpu/src/universal.rs (UPDATE)

impl UniversalComputeResource {
    pub async fn detect_memory_capabilities(&self) -> MemoryCapabilities {
        // NEW: Use unified memory detector
        let unified_info = UnifiedMemoryDetector::detect_for_device(&self.device).await;
        
        MemoryCapabilities {
            total_bytes: self.query_total_memory(),
            bandwidth_bytes_per_sec: self.query_bandwidth(),
            
            // ✅ Accurately detected across vendors!
            unified_memory: unified_info.has_unified_memory,
            zero_copy: unified_info.supports_zero_copy,
            
            cache_levels: self.detect_cache_hierarchy(),
            access_patterns: vec![
                MemoryAccessPattern::Sequential,
                MemoryAccessPattern::Coalesced,
            ],
        }
    }
}
```

### Scheduler Integration

```rust
// crates/runtime/gpu/src/scheduler.rs (UPDATE)

impl UniversalGpuEngine {
    pub async fn execute_with_unified_memory(
        &self,
        workload: &UniversalWorkload,
    ) -> ToadStoolResult<ComputeResult> {
        // Find device with unified memory
        let device = self.select_device_with_unified_memory().await?;
        
        if device.has_unified_memory() {
            info!("🚀 Executing with zero-copy unified memory");
            
            // Allocate unified buffer
            let buffer = self.unified_memory
                .allocate(workload.input_size)
                .await?;
            
            // Write input (CPU)
            buffer.write_async(0, &workload.input_data).await?;
            
            // Execute on GPU (zero-copy!)
            let result = device.execute(
                &workload.kernel,
                buffer.device_ptr(),
            ).await?;
            
            // Read output (CPU)
            let output = buffer.read_async(0, result.output_size).await?;
            
            Ok(ComputeResult {
                data: output,
                zero_copy_used: true,
                backend: device.backend_name(),
            })
        } else {
            // Fallback: traditional copy-based execution
            self.execute_with_copy(workload, &device).await
        }
    }
}
```

---

## 📦 Module Structure

```
crates/runtime/gpu/src/unified_memory/
├── mod.rs                      # Public API
├── manager.rs                  # UniversalUnifiedMemory
├── buffer.rs                   # UnifiedBuffer
├── backend.rs                  # UnifiedMemoryBackend trait
├── backends/
│   ├── mod.rs
│   ├── vulkan.rs              # Vulkan backend
│   ├── opencl.rs              # OpenCL SVM backend
│   ├── webgpu.rs              # WebGPU backend
│   └── cpu.rs                 # CPU fallback
├── sync.rs                     # Synchronization logic
├── pool.rs                     # Memory pooling
├── detector.rs                 # Capability detection
├── metrics.rs                  # Performance metrics
└── types.rs                    # Common types
```

---

## 🎓 Success Criteria

### Must Have (MVP)

- ✅ Vulkan backend works on Intel, AMD, NVIDIA
- ✅ OpenCL backend works on Intel, AMD, NVIDIA
- ✅ CPU fallback always works
- ✅ Zero production unwraps
- ✅ Async-native API
- ✅ Comprehensive error handling
- ✅ Thread-safe, concurrent operations
- ✅ Unit tests for all backends
- ✅ Integration tests with real GPUs

### Should Have (Polish)

- ✅ WebGPU backend (pure Rust)
- ✅ Memory pooling optimization
- ✅ Smart synchronization
- ✅ Performance metrics
- ✅ Documentation and examples
- ✅ Benchmarks comparing backends

### Nice to Have (Future)

- ✅ Auto-tuning for workload patterns
- ✅ Multi-GPU unified memory
- ✅ Peer-to-peer GPU transfers
- ✅ NUMA-aware allocation

---

## 📝 Documentation Requirements

1. **API Documentation**: Complete rustdoc for all public APIs
2. **Architecture Guide**: Design rationale and patterns
3. **Backend Guide**: How to add new backends
4. **Performance Guide**: Optimization tips and benchmarks
5. **Migration Guide**: How to adopt unified memory in existing code
6. **Examples**: 5+ working examples covering common use cases

---

## 🔐 Security Considerations

1. **Memory Safety**: All unsafe code documented with SAFETY comments
2. **Pointer Validation**: Check pointers before dereferencing
3. **Bounds Checking**: Validate all buffer accesses
4. **Synchronization**: Prevent data races with proper locking
5. **Resource Cleanup**: RAII and Drop implementations for cleanup

---

## 🌍 Cross-Platform Support

| OS | Vulkan | OpenCL | WebGPU | CPU |
|----|--------|--------|--------|-----|
| **Linux** | ✅ | ✅ | ✅ | ✅ |
| **Windows** | ✅ | ✅ | ✅ | ✅ |
| **macOS** | ✅ | ⚠️ (deprecated) | ✅ | ✅ |
| **Android** | ✅ | ✅ | 🔄 (future) | ✅ |
| **iOS** | ⚠️ (MoltenVK) | ❌ | ✅ | ✅ |

---

## 📅 Implementation Timeline

**Total Estimate**: 2-3 weeks for complete implementation

### Week 1: Core Infrastructure
- Day 1-2: Module structure, traits, core types
- Day 3-4: Vulkan backend implementation
- Day 5: OpenCL backend implementation

### Week 2: Polish & Testing
- Day 1-2: WebGPU backend + CPU fallback
- Day 3-4: Memory pooling, synchronization
- Day 5: Unit tests and integration tests

### Week 3: Integration & Documentation
- Day 1-2: ToadStool scheduler integration
- Day 3: Performance benchmarks
- Day 4-5: Documentation and examples

---

**Status**: 🎯 Ready for Implementation  
**Next Step**: Create UNIFIED_MEMORY_ROADMAP.md at root to track progress  
**Philosophy**: Deep solutions, no debt, modern async Rust, sovereignty first 🍄

