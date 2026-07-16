// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core types for unified memory system

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Unique buffer identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BufferId(u64);

impl BufferId {
    /// Create a new buffer ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for BufferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Buffer#{}", self.0)
    }
}

/// Buffer ID generator (thread-safe)
pub struct BufferIdGenerator {
    next_id: AtomicU64,
}

impl BufferIdGenerator {
    /// Create a new ID generator
    pub const fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
        }
    }

    /// Generate the next unique ID
    pub fn next(&self) -> BufferId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        BufferId(id)
    }
}

impl Default for BufferIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory allocation flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryFlags {
    /// Prefer CPU-optimized memory
    pub prefer_cpu: bool,

    /// Prefer GPU-optimized memory
    pub prefer_gpu: bool,

    /// Memory should be coherent (no explicit sync needed)
    pub coherent: bool,

    /// Memory should be cached on CPU side
    pub cached: bool,
}

impl MemoryFlags {
    /// Default flags (balanced CPU/GPU access)
    pub const fn balanced() -> Self {
        Self {
            prefer_cpu: false,
            prefer_gpu: false,
            coherent: true,
            cached: false,
        }
    }

    /// CPU-optimized flags
    pub const fn cpu_optimized() -> Self {
        Self {
            prefer_cpu: true,
            prefer_gpu: false,
            coherent: true,
            cached: true,
        }
    }

    /// GPU-optimized flags
    pub const fn gpu_optimized() -> Self {
        Self {
            prefer_cpu: false,
            prefer_gpu: true,
            coherent: false,
            cached: false,
        }
    }
}

impl Default for MemoryFlags {
    fn default() -> Self {
        Self::balanced()
    }
}

/// Synchronization state of a buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SyncState {
    /// Buffer is synchronized between CPU and GPU
    #[default]
    Synced,

    /// CPU has modifications, GPU needs sync
    CpuModified,

    /// GPU has modifications, CPU needs sync
    GpuModified,

    /// Both modified (conflict - needs resolution)
    Conflict,
}

impl fmt::Display for SyncState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Synced => write!(f, "synced"),
            Self::CpuModified => write!(f, "cpu_modified"),
            Self::GpuModified => write!(f, "gpu_modified"),
            Self::Conflict => write!(f, "conflict"),
        }
    }
}

/// Synchronization target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncTarget {
    /// Synchronize to CPU
    Cpu,

    /// Synchronize to GPU/Device
    Device,
}

/// Backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendType {
    /// Vulkan backend
    Vulkan,

    /// OpenCL-class backend (not implemented in-tree; use `gpu.dispatch.opencl` capability provider). **DEPRECATED S198** stub for config/serialization.
    OpenCL,

    /// `WebGPU` backend
    WebGpu,

    /// CPU fallback
    Cpu,
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vulkan => write!(f, "Vulkan"),
            Self::OpenCL => write!(f, "OpenCL"),
            Self::WebGpu => write!(f, "WebGPU"),
            Self::Cpu => write!(f, "CPU"),
        }
    }
}

/// Backend selection strategy
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BackendStrategy {
    /// Automatic: Sovereignty-first (`WebGPU` > Vulkan > CPU)
    #[default]
    Automatic,

    /// Sovereignty only: `WebGPU` or fail
    SovereignOnly,

    /// Performance: Prefer fastest backend (Vulkan > `WebGPU` > CPU)
    Performance,

    /// Specific backend
    Specific(BackendType),
}

/// Unified memory capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMemoryCapabilities {
    /// Backend type
    pub backend_type: BackendType,

    /// Maximum allocation size (bytes)
    pub max_allocation_size: usize,

    /// Whether memory is truly unified (zero-copy)
    pub zero_copy: bool,

    /// Whether memory is coherent (no explicit sync)
    pub coherent: bool,

    /// Whether CPU access is fast
    pub cpu_fast_access: bool,

    /// Whether GPU access is fast
    pub gpu_fast_access: bool,

    /// Alignment requirement (bytes)
    pub alignment_requirement: usize,
}

impl UnifiedMemoryCapabilities {
    /// Check if this backend is truly unified (zero-copy)
    pub const fn is_truly_unified(&self) -> bool {
        self.zero_copy && (self.cpu_fast_access || self.gpu_fast_access)
    }

    /// Check if explicit synchronization is needed
    pub const fn needs_explicit_sync(&self) -> bool {
        !self.coherent
    }
}

/// Memory access pattern (for optimization hints)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccessPattern {
    /// Primarily CPU access
    CpuDominant,

    /// Primarily GPU access
    GpuDominant,

    /// Balanced CPU/GPU access
    #[default]
    Balanced,

    /// Alternating CPU/GPU access
    Alternating,
}

/// Unified memory configuration
#[derive(Debug, Clone)]
pub struct UnifiedMemoryConfig {
    /// Backend selection strategy
    pub backend_strategy: BackendStrategy,

    /// Default memory flags
    pub default_flags: MemoryFlags,

    /// Enable memory pooling
    pub enable_pooling: bool,

    /// Pool size (number of buffers per size class)
    pub pool_size: usize,

    /// Enable performance metrics
    pub enable_metrics: bool,
}

impl Default for UnifiedMemoryConfig {
    fn default() -> Self {
        Self {
            backend_strategy: BackendStrategy::Automatic,
            default_flags: MemoryFlags::default(),
            enable_pooling: true,
            pool_size: 16,
            enable_metrics: true,
        }
    }
}

/// Unified memory statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnifiedMemoryStats {
    /// Total bytes allocated
    pub total_allocated: u64,

    /// Peak allocation
    pub peak_allocated: u64,

    /// Number of allocations
    pub allocation_count: u64,

    /// Number of deallocations
    pub deallocation_count: u64,

    /// Active allocations
    pub active_allocations: u64,

    /// Number of CPU → GPU syncs
    pub cpu_to_gpu_syncs: u64,

    /// Number of GPU → CPU syncs
    pub gpu_to_cpu_syncs: u64,

    /// Total bytes synced
    pub bytes_synced: u64,

    /// Average sync latency (microseconds)
    pub avg_sync_latency_us: f64,

    /// Backend in use
    pub backend: String,

    /// Pool hit rate (0.0 - 1.0)
    pub pool_hit_rate: f64,
}

impl UnifiedMemoryStats {
    /// Create new empty stats
    pub fn new(backend: String) -> Self {
        Self {
            backend,
            ..Default::default()
        }
    }

    /// Update peak allocation if needed
    pub const fn update_peak(&mut self, current: u64) {
        if current > self.peak_allocated {
            self.peak_allocated = current;
        }
    }

    /// Calculate pool hit rate
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )] // ratio in [0,1]
    pub fn calculate_pool_hit_rate(&mut self, hits: u64, misses: u64) {
        let total = hits + misses;
        self.pool_hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };
    }
}

/// Buffer metadata (for tracking)
#[derive(Debug, Clone)]
pub struct UnifiedBufferMetadata {
    /// Buffer ID
    pub id: BufferId,

    /// Size in bytes
    pub size: usize,

    /// Memory flags used
    pub flags: MemoryFlags,

    /// Current sync state
    pub sync_state: SyncState,

    /// Access pattern hint
    pub access_pattern: AccessPattern,

    /// Creation timestamp
    pub created_at: std::time::Instant,

    /// Last access timestamp
    pub last_accessed: std::time::Instant,

    /// Number of accesses
    pub access_count: u64,
}

impl UnifiedBufferMetadata {
    /// Create new metadata
    pub fn new(id: BufferId, size: usize, flags: MemoryFlags) -> Self {
        let now = std::time::Instant::now();
        Self {
            id,
            size,
            flags,
            sync_state: SyncState::Synced,
            access_pattern: AccessPattern::default(),
            created_at: now,
            last_accessed: now,
            access_count: 0,
        }
    }

    /// Record an access
    pub fn record_access(&mut self) {
        self.last_accessed = std::time::Instant::now();
        self.access_count = self.access_count.saturating_add(1);
    }

    /// Get age in seconds
    pub fn age_seconds(&self) -> f64 {
        self.created_at.elapsed().as_secs_f64()
    }
}

/// Errors during unified buffer construction or validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BufferError {
    /// Buffer size must be greater than zero.
    #[error("Buffer size cannot be zero")]
    ZeroSize,

    /// CPU pointer must not be null.
    #[error("CPU pointer cannot be null at buffer creation")]
    NullCpuPointer,

    /// CPU pointer must not lie in the NULL page.
    #[error("CPU pointer {ptr:#x} lies in the NULL page (must be >= 4096)")]
    NullPagePointer {
        /// Invalid pointer value.
        ptr: usize,
    },
}
