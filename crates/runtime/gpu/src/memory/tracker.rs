// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU Memory Usage Tracker
//!
//! Tracks GPU memory allocations for:
//! - Memory leak detection
//! - Usage monitoring
//! - Out-of-memory prevention
//! - Performance optimization

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tracks GPU memory allocations
pub struct MemoryTracker {
    allocations: Arc<RwLock<HashMap<String, AllocationInfo>>>,
    stats: Arc<RwLock<MemoryStats>>,
}

/// Information about a memory allocation
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    pub id: String,
    pub size_bytes: usize,
    /// Uses `tokio::time::Instant` so tests can mock time with
    /// `tokio::time::pause()` / `tokio::time::advance()`.
    pub allocated_at: tokio::time::Instant,
    pub purpose: String,
    pub stack_trace: Option<String>,
}

/// Memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub total_allocated: u64,
    pub total_freed: u64,
    pub current_usage: u64,
    pub peak_usage: u64,
    pub allocation_count: u64,
    pub free_count: u64,
}

impl MemoryTracker {
    /// Create a new memory tracker
    pub fn new() -> Self {
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(MemoryStats::default())),
        }
    }

    /// Record an allocation
    pub async fn track_allocation(&self, id: String, size: usize, purpose: String) {
        let info = AllocationInfo {
            id: id.clone(),
            size_bytes: size,
            allocated_at: tokio::time::Instant::now(),
            purpose,
            stack_trace: None, // Could capture backtrace in debug builds
        };

        let mut allocations = self.allocations.write().await;
        allocations.insert(id, info);

        let mut stats = self.stats.write().await;
        stats.total_allocated += size as u64;
        stats.current_usage += size as u64;
        stats.allocation_count += 1;

        if stats.current_usage > stats.peak_usage {
            stats.peak_usage = stats.current_usage;
        }

        tracing::debug!("GPU memory allocated: {} bytes (total: {} MB)", 
            size, stats.current_usage / (1024 * 1024));
    }

    /// Record a deallocation
    pub async fn track_deallocation(&self, id: &str) -> Option<usize> {
        let mut allocations = self.allocations.write().await;
        if let Some(info) = allocations.remove(id) {
            let size = info.size_bytes;

            let mut stats = self.stats.write().await;
            stats.total_freed += size as u64;
            stats.current_usage -= size as u64;
            stats.free_count += 1;

            tracing::debug!("GPU memory freed: {} bytes (total: {} MB)", 
                size, stats.current_usage / (1024 * 1024));

            Some(size)
        } else {
            tracing::warn!("Attempted to free unknown allocation: {}", id);
            None
        }
    }

    /// Get current memory statistics
    pub async fn stats(&self) -> MemoryStats {
        self.stats.read().await.clone()
    }

    /// Get active allocations
    pub async fn active_allocations(&self) -> Vec<AllocationInfo> {
        self.allocations.read().await.values().cloned().collect()
    }

    /// Check for memory leaks
    ///
    /// Returns allocations that have been alive for longer than threshold
    pub async fn check_leaks(&self, threshold: std::time::Duration) -> Vec<AllocationInfo> {
        let allocations = self.allocations.read().await;
        let now = tokio::time::Instant::now();

        allocations
            .values()
            .filter(|info| now.duration_since(info.allocated_at) > threshold)
            .cloned()
            .collect()
    }

    /// Check if memory usage is above threshold
    pub async fn is_over_threshold(&self, threshold_percent: f32) -> bool {
        let stats = self.stats.read().await;
        if stats.peak_usage == 0 {
            return false;
        }
        let usage_percent = (stats.current_usage as f32 / stats.peak_usage as f32) * 100.0;
        usage_percent > threshold_percent
    }

    /// Get memory pressure level
    pub async fn memory_pressure(&self) -> MemoryPressure {
        let stats = self.stats.read().await;
        if stats.peak_usage == 0 {
            return MemoryPressure::Low;
        }

        let usage_percent = (stats.current_usage as f32 / stats.peak_usage as f32) * 100.0;

        match usage_percent {
            p if p < 50.0 => MemoryPressure::Low,
            p if p < 75.0 => MemoryPressure::Medium,
            p if p < 90.0 => MemoryPressure::High,
            _ => MemoryPressure::Critical,
        }
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressure {
    /// < 50% of peak usage
    Low,
    /// 50-75% of peak usage
    Medium,
    /// 75-90% of peak usage
    High,
    /// > 90% of peak usage - consider freeing cached buffers
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_tracker_basic() {
        let tracker = MemoryTracker::new();

        tracker.track_allocation("buf1".to_string(), 1024, "test".to_string()).await;
        tracker.track_allocation("buf2".to_string(), 2048, "test".to_string()).await;

        let stats = tracker.stats().await;
        assert_eq!(stats.current_usage, 3072);
        assert_eq!(stats.allocation_count, 2);

        tracker.track_deallocation("buf1").await;

        let stats = tracker.stats().await;
        assert_eq!(stats.current_usage, 2048);
        assert_eq!(stats.free_count, 1);
    }

    // Leak detection test uses paused tokio time so no real wall-clock time
    // elapses. allocated_at uses tokio::time::Instant, so advance() is precise.
    #[tokio::test(start_paused = true)]
    async fn test_memory_leak_detection() {
        let tracker = MemoryTracker::new();

        tracker
            .track_allocation("buf1".to_string(), 1024, "test".to_string())
            .await;

        // Check immediately — allocation is fresh (1s threshold not exceeded).
        let leaks = tracker
            .check_leaks(std::time::Duration::from_secs(1))
            .await;
        assert_eq!(leaks.len(), 0);

        // Advance time past the threshold — no sleep required.
        tokio::time::advance(std::time::Duration::from_millis(100)).await;
        let leaks = tracker
            .check_leaks(std::time::Duration::from_millis(50))
            .await;
        assert_eq!(leaks.len(), 1, "Allocation should now be detected as a leak");
    }

    #[tokio::test]
    async fn test_memory_pressure() {
        let tracker = MemoryTracker::new();

        // Low pressure
        tracker.track_allocation("buf1".to_string(), 1000, "test".to_string()).await;
        assert_eq!(tracker.memory_pressure().await, MemoryPressure::Low);

        // Build up to high pressure
        tracker.track_allocation("buf2".to_string(), 8000, "test".to_string()).await;
        
        let pressure = tracker.memory_pressure().await;
        assert!(matches!(pressure, MemoryPressure::High | MemoryPressure::Critical));
    }
}

