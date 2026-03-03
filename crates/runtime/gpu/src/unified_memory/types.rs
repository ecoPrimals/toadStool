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
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub fn as_u64(self) -> u64 {
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
    pub fn new() -> Self {
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
    pub fn balanced() -> Self {
        Self {
            prefer_cpu: false,
            prefer_gpu: false,
            coherent: true,
            cached: false,
        }
    }

    /// CPU-optimized flags
    pub fn cpu_optimized() -> Self {
        Self {
            prefer_cpu: true,
            prefer_gpu: false,
            coherent: true,
            cached: true,
        }
    }

    /// GPU-optimized flags
    pub fn gpu_optimized() -> Self {
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

    /// OpenCL backend
    OpenCL,

    /// WebGPU backend
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
    /// Automatic: Sovereignty-first (WebGPU > Vulkan > OpenCL > CPU)
    #[default]
    Automatic,

    /// Sovereignty only: WebGPU or fail
    SovereignOnly,

    /// Performance: Prefer fastest backend (Vulkan > OpenCL > WebGPU > CPU)
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
    pub fn is_truly_unified(&self) -> bool {
        self.zero_copy && (self.cpu_fast_access || self.gpu_fast_access)
    }

    /// Check if explicit synchronization is needed
    pub fn needs_explicit_sync(&self) -> bool {
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
    pub fn update_peak(&mut self, current: u64) {
        if current > self.peak_allocated {
            self.peak_allocated = current;
        }
    }

    /// Calculate pool hit rate
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_id_generator() {
        let gen = BufferIdGenerator::new();
        let id1 = gen.next();
        let id2 = gen.next();
        let id3 = gen.next();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_eq!(id1.as_u64(), 1);
        assert_eq!(id2.as_u64(), 2);
        assert_eq!(id3.as_u64(), 3);
    }

    #[test]
    fn test_memory_flags() {
        let balanced = MemoryFlags::balanced();
        assert!(!balanced.prefer_cpu);
        assert!(!balanced.prefer_gpu);
        assert!(balanced.coherent);

        let cpu = MemoryFlags::cpu_optimized();
        assert!(cpu.prefer_cpu);
        assert!(!cpu.prefer_gpu);
        assert!(cpu.cached);

        let gpu = MemoryFlags::gpu_optimized();
        assert!(!gpu.prefer_cpu);
        assert!(gpu.prefer_gpu);
        assert!(!gpu.coherent);
    }

    #[test]
    fn test_sync_state() {
        let state = SyncState::default();
        assert_eq!(state, SyncState::Synced);

        let modified = SyncState::CpuModified;
        assert_ne!(modified, SyncState::Synced);
    }

    #[test]
    fn test_backend_type_display() {
        assert_eq!(BackendType::Vulkan.to_string(), "Vulkan");
        assert_eq!(BackendType::OpenCL.to_string(), "OpenCL");
        assert_eq!(BackendType::WebGpu.to_string(), "WebGPU");
        assert_eq!(BackendType::Cpu.to_string(), "CPU");
    }

    #[test]
    fn test_unified_memory_capabilities() {
        let caps = UnifiedMemoryCapabilities {
            backend_type: BackendType::Vulkan,
            max_allocation_size: 1024 * 1024 * 1024,
            zero_copy: true,
            coherent: true,
            cpu_fast_access: true,
            gpu_fast_access: true,
            alignment_requirement: 64,
        };

        assert!(caps.is_truly_unified());
        assert!(!caps.needs_explicit_sync());
    }

    #[test]
    fn test_buffer_metadata() {
        let id = BufferId::new(1);
        let mut metadata = UnifiedBufferMetadata::new(id, 4096, MemoryFlags::default());

        assert_eq!(metadata.id, id);
        assert_eq!(metadata.size, 4096);
        assert_eq!(metadata.access_count, 0);

        metadata.record_access();
        assert_eq!(metadata.access_count, 1);

        metadata.record_access();
        assert_eq!(metadata.access_count, 2);
    }

    #[test]
    fn test_unified_memory_stats() {
        let mut stats = UnifiedMemoryStats::new("Vulkan".to_string());

        stats.total_allocated = 1024;
        stats.update_peak(1024);
        assert_eq!(stats.peak_allocated, 1024);

        stats.total_allocated = 2048;
        stats.update_peak(2048);
        assert_eq!(stats.peak_allocated, 2048);

        stats.total_allocated = 1024;
        stats.update_peak(1024);
        assert_eq!(stats.peak_allocated, 2048); // Peak stays at max

        stats.calculate_pool_hit_rate(80, 20);
        assert!((stats.pool_hit_rate - 0.8).abs() < 0.01);
    }
}
