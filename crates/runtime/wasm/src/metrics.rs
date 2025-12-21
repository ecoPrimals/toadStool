//! WebAssembly runtime metrics and monitoring
//!
//! Provides comprehensive metrics for runtime monitoring,
//! performance tracking, and resource usage analysis.

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

use toadstool::security::SecurityContext;

/// Active execution tracking metadata
#[derive(Clone, Debug)]
pub struct ExecutionHandle {
    /// Unique execution identifier
    pub id: Uuid,
    
    /// Module cache key
    pub module_key: String,
    
    /// Execution start time
    pub start_time: Instant,
    
    /// Security context
    pub security_context: SecurityContext,
}

impl ExecutionHandle {
    /// Create a new execution handle
    pub fn new(
        id: Uuid,
        module_key: String,
        security_context: SecurityContext,
    ) -> Self {
        Self {
            id,
            module_key,
            start_time: Instant::now(),
            security_context,
        }
    }

    /// Get execution duration in milliseconds
    pub fn duration_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

/// WASM execution resource usage
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// Memory used in bytes
    pub used_bytes: u64,
    
    /// Peak memory usage in bytes
    pub peak_bytes: u64,
}

impl ResourceUsage {
    /// Create a new resource usage record
    pub const fn new(used_bytes: u64, peak_bytes: u64) -> Self {
        Self {
            used_bytes,
            peak_bytes,
        }
    }

    /// Check if usage exceeds limit
    pub const fn exceeds_limit(&self, limit_bytes: u64) -> bool {
        self.used_bytes > limit_bytes || self.peak_bytes > limit_bytes
    }
}

/// Metrics collector for runtime monitoring
pub struct MetricsCollector {
    /// Active executions
    active: Arc<RwLock<std::collections::HashMap<Uuid, ExecutionHandle>>>,
    
    /// Total executions counter
    total_executions: Arc<RwLock<u64>>,
    
    /// Successful executions counter
    successful_executions: Arc<RwLock<u64>>,
    
    /// Failed executions counter
    failed_executions: Arc<RwLock<u64>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            active: Arc::new(RwLock::new(std::collections::HashMap::new())),
            total_executions: Arc::new(RwLock::new(0)),
            successful_executions: Arc::new(RwLock::new(0)),
            failed_executions: Arc::new(RwLock::new(0)),
        }
    }

    /// Register a new execution
    pub async fn register_execution(&self, handle: ExecutionHandle) {
        let id = handle.id;
        self.active.write().await.insert(id, handle);
        *self.total_executions.write().await += 1;
    }

    /// Mark execution as complete
    pub async fn complete_execution(&self, id: Uuid, success: bool) {
        self.active.write().await.remove(&id);
        
        if success {
            *self.successful_executions.write().await += 1;
        } else {
            *self.failed_executions.write().await += 1;
        }
    }

    /// Get number of active executions
    pub async fn active_count(&self) -> usize {
        self.active.read().await.len()
    }

    /// Get total number of executions
    pub async fn total_count(&self) -> u64 {
        *self.total_executions.read().await
    }

    /// Get success rate (0.0 to 1.0)
    pub async fn success_rate(&self) -> f64 {
        let total = *self.total_executions.read().await;
        if total == 0 {
            return 1.0;
        }
        
        let successful = *self.successful_executions.read().await;
        successful as f64 / total as f64
    }

    /// Get active execution handles
    pub async fn active_executions(&self) -> Vec<ExecutionHandle> {
        self.active
            .read()
            .await
            .values()
            .cloned()
            .collect()
    }

    /// Clear all metrics (for testing)
    pub async fn clear(&self) {
        self.active.write().await.clear();
        *self.total_executions.write().await = 0;
        *self.successful_executions.write().await = 0;
        *self.failed_executions.write().await = 0;
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::security::IsolationLevel;

    #[test]
    fn test_execution_handle_creation() {
        let id = Uuid::new_v4();
        let security = SecurityContext::new(IsolationLevel::Full);
        let handle = ExecutionHandle::new(id, "test_module".to_string(), security);
        
        assert_eq!(handle.id, id);
        assert_eq!(handle.module_key, "test_module");
    }

    #[test]
    fn test_execution_handle_duration() {
        let id = Uuid::new_v4();
        let security = SecurityContext::new(IsolationLevel::Full);
        let handle = ExecutionHandle::new(id, "test".to_string(), security);
        
        // ✅ MODERNIZED: This is a synchronous test, sleep is minimal and acceptable
        // In a real scenario, metrics would update immediately
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        assert!(handle.duration_ms() >= 10);
    }

    #[test]
    fn test_resource_usage() {
        let usage = ResourceUsage::new(1024, 2048);
        
        assert_eq!(usage.used_bytes, 1024);
        assert_eq!(usage.peak_bytes, 2048);
        assert!(!usage.exceeds_limit(3000));
        assert!(usage.exceeds_limit(1000));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        
        assert_eq!(collector.active_count().await, 0);
        assert_eq!(collector.total_count().await, 0);
        assert_eq!(collector.success_rate().await, 1.0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_metrics_collector_registration() {
        let collector = MetricsCollector::new();
        let id = Uuid::new_v4();
        let security = SecurityContext::new(IsolationLevel::Full);
        let handle = ExecutionHandle::new(id, "test".to_string(), security);
        
        collector.register_execution(handle).await;
        
        assert_eq!(collector.active_count().await, 1);
        assert_eq!(collector.total_count().await, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_metrics_collector_completion() {
        let collector = MetricsCollector::new();
        let id = Uuid::new_v4();
        let security = SecurityContext::new(IsolationLevel::Full);
        let handle = ExecutionHandle::new(id, "test".to_string(), security);
        
        collector.register_execution(handle).await;
        collector.complete_execution(id, true).await;
        
        assert_eq!(collector.active_count().await, 0);
        assert_eq!(collector.success_rate().await, 1.0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_metrics_collector_success_rate() {
        let collector = MetricsCollector::new();
        
        for i in 0..10 {
            let id = Uuid::new_v4();
            let security = SecurityContext::new(IsolationLevel::Full);
            let handle = ExecutionHandle::new(id, format!("test_{i}"), security);
            
            collector.register_execution(handle).await;
            collector.complete_execution(id, i < 8).await; // 80% success
        }
        
        let rate = collector.success_rate().await;
        assert!((rate - 0.8).abs() < 0.01);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_metrics_collector_clear() {
        let collector = MetricsCollector::new();
        let id = Uuid::new_v4();
        let security = SecurityContext::new(IsolationLevel::Full);
        let handle = ExecutionHandle::new(id, "test".to_string(), security);
        
        collector.register_execution(handle).await;
        collector.clear().await;
        
        assert_eq!(collector.active_count().await, 0);
        assert_eq!(collector.total_count().await, 0);
    }
}

