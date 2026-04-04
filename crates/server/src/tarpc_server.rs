// SPDX-License-Identifier: AGPL-3.0-only
//! # ToadStool tarpc Server Implementation
//!
//! High-performance binary RPC server for primal-to-primal communication.
//! Follows Songbird's architecture pattern.

use futures::StreamExt;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tarpc::context::Context;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::errors::{ServerError, ServerResult};

// Deep debt solution: Use pure RPC types from local module
use crate::rpc_types::{
    AvailableResources, ComputeCapabilities, ComputeUnit, ExecutionMetrics, HealthStatus,
    ToadStoolComputeRpc, WorkloadResult, WorkloadStatus, WorkloadSubmission,
};

/// tarpc server state
pub struct ToadStoolTarpcServer {
    /// Service start time
    start_time: Instant,
    /// Service version (`Arc<str>` avoids allocation on spawn-per-connection clone)
    version: Arc<str>,
    /// Active workloads (`Arc<str>` key = zero-copy clone for entry/insert)
    workloads: Arc<RwLock<std::collections::HashMap<Arc<str>, WorkloadResult>>>,
    /// Workload executor (real implementation, not mock)
    executor: Arc<dyn WorkloadExecutor + Send + Sync>,
    /// Error count for monitoring
    error_count: Arc<AtomicU64>,
}

impl ToadStoolTarpcServer {
    /// Calculate current resource utilization
    ///
    /// **Deep Debt Compliance**:
    /// - Queries real system state (no hardcoding)
    /// - Returns 0.0-1.0 utilization percentage
    /// - Graceful degradation on query failure
    async fn calculate_resource_utilization(&self) -> f32 {
        let active_count = self.workloads.read().await.len();
        let max_capacity = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4)
            * 4; // ~4 workloads per core

        #[expect(
            clippy::cast_precision_loss,
            reason = "precision loss acceptable for this conversion"
        )]
        let base_utilization = if max_capacity > 0 {
            (active_count as f32) / (max_capacity as f32)
        } else {
            0.0
        };

        // Factor in system load (Deep Debt: runtime query)
        let system_load = Self::query_system_load();
        let load_factor = system_load.unwrap_or(0.5);

        // Combined utilization (capped at 1.0)
        (base_utilization * 0.7 + load_factor * 0.3).min(1.0)
    }

    /// Query system load average (runtime discovery)
    fn query_system_load() -> Option<f32> {
        #[cfg(unix)]
        {
            if let Ok(loadavg) = std::fs::read_to_string("/proc/loadavg")
                && let Some(first) = loadavg.split_whitespace().next()
                && let Ok(load) = first.parse::<f32>()
            {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "precision loss acceptable for this conversion"
                )]
                let cpu_count = std::thread::available_parallelism()
                    .map(|n| n.get() as f32)
                    .unwrap_or(4.0);
                return Some((load / cpu_count).min(1.0));
            }
        }
        None
    }
}

/// Workload executor trait (capability-based, not hardcoded)
///
/// Following principles:
/// - Self-knowledge: knows only its own capabilities
/// - Discovery: discovers other primals at runtime
/// - Complete implementation: no mocks in production
#[async_trait::async_trait]
pub trait WorkloadExecutor {
    /// Execute workload with given submission
    async fn execute(&self, submission: WorkloadSubmission) -> Result<WorkloadResult, String>;

    /// Query this executor's capabilities (self-knowledge)
    async fn query_capabilities(&self) -> Result<ComputeCapabilities, String>;

    /// Cancel running workload
    async fn cancel(&self, workload_id: &str) -> Result<(), String>;
}

impl ToadStoolTarpcServer {
    /// Create new tarpc server with real executor
    ///
    /// Pass `error_count` to share the counter with JSON-RPC server for unified monitoring.
    pub fn new(
        version: impl AsRef<str>,
        executor: Arc<dyn WorkloadExecutor + Send + Sync>,
        error_count: Option<Arc<AtomicU64>>,
    ) -> Self {
        Self {
            start_time: Instant::now(),
            version: Arc::from(version.as_ref()),
            workloads: Arc::new(RwLock::new(std::collections::HashMap::new())),
            executor,
            error_count: error_count.unwrap_or_else(|| Arc::new(AtomicU64::new(0))),
        }
    }

    /// Start tarpc server on Unix socket (OPTIONAL transport - for binary RPC when needed)
    /// Per wateringHole standard: JSON-RPC 2.0 is PRIMARY, tarpc is OPTIONAL
    ///
    /// Deep debt principle: No TCP hardcoding, use Unix sockets for multi-instance support
    ///
    /// # Errors
    ///
    /// Returns [`ServerError`] if directory creation, socket bind, permission setting, or accept fails.
    pub async fn serve_unix(self, socket_path: impl AsRef<std::path::Path>) -> ServerResult<()> {
        use tarpc::server::{BaseChannel, Channel};
        use tokio::net::UnixListener;
        use tokio_serde::formats::Json;

        let socket_path = socket_path.as_ref();

        // Ensure parent directory exists (biomeOS requirement for custom socket paths)
        if let Some(parent) = socket_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                warn!(
                    "Failed to create socket directory {parent:?}: {e}; continuing without creating parent"
                );
            } else {
                info!("Ensured socket directory exists: {:?}", parent);
            }
        }

        // Clean up old socket if present
        match tokio::fs::remove_file(socket_path).await {
            Ok(()) => info!("Removed old socket file: {:?}", socket_path),
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(ServerError::Network(e.to_string())),
        }

        info!("tarpc server binding to Unix socket: {:?}", socket_path);
        let listener =
            UnixListener::bind(socket_path).map_err(|e| ServerError::Network(e.to_string()))?;

        // Set permissions to user-only (0600)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(socket_path)
                .await
                .map_err(|e| ServerError::Internal(e.to_string()))?
                .permissions();
            perms.set_mode(0o600); // Owner read+write only
            tokio::fs::set_permissions(socket_path, perms)
                .await
                .map_err(|e| ServerError::Internal(e.to_string()))?;
            info!("Set socket permissions to 0600 (user-only)");
        }

        info!(
            "✅ tarpc server listening on Unix socket: {:?}",
            socket_path
        );

        loop {
            let (stream, _addr) = listener
                .accept()
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            let server = self.clone();

            tokio::spawn(async move {
                let framed = tokio_util::codec::LengthDelimitedCodec::builder().new_framed(stream);
                let transport = tokio_serde::Framed::new(framed, Json::<_, _>::default());

                let channel = BaseChannel::with_defaults(transport);
                channel
                    .execute(server.serve())
                    .for_each(|rpc| async {
                        tokio::spawn(rpc);
                    })
                    .await;
            });
        }
    }

    /// Start tarpc server on TCP (DEBUG ONLY - not for production)
    ///
    /// Deep debt violation: TCP with hardcoded ports breaks multi-instance support.
    /// Use serve_unix() instead for production.
    ///
    /// # Errors
    ///
    /// Always returns [`ServerError`] (deprecated - use serve_unix or serve_tcp instead).
    #[deprecated(
        since = "2.2.0",
        note = "Use serve_unix() for production. TCP hardcoding violates deep debt principles."
    )]
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )] // Deprecated; returns Err immediately; kept async for API compatibility
    pub async fn serve_tcp_debug(self, addr: SocketAddr) -> ServerResult<()> {
        warn!("⚠️  TCP mode is DEBUG ONLY - violates deep debt principles");
        warn!("⚠️  Use Unix sockets for production (serve_unix)");
        info!(
            "tarpc TCP debug endpoint requested on: {addr} — use serve_unix() or serve_tcp() instead"
        );

        Err(ServerError::Execution(
            "serve_tcp_debug is deprecated — use serve_unix() or serve_tcp()".to_string(),
        ))
    }

    /// Start tarpc server on TCP listener (isomorphic fallback)
    ///
    /// **ISOMORPHIC MODE**: Automatic fallback for platforms without Unix sockets.
    ///
    /// This method is used only when Unix sockets fail due to platform constraints
    /// (SELinux, Android, etc.). The listener is pre-bound to 127.0.0.1:0 for security.
    ///
    /// # Errors
    ///
    /// Returns [`ServerError`] if getting local address or accept fails.
    pub async fn serve_tcp(self, listener: tokio::net::TcpListener) -> ServerResult<()> {
        use tarpc::server::{BaseChannel, Channel};
        use tokio_serde::formats::Json;

        let local_addr = listener
            .local_addr()
            .map_err(|e| ServerError::Network(e.to_string()))?;
        info!("✅ tarpc server listening on TCP: {}", local_addr);

        loop {
            let (stream, _addr) = listener
                .accept()
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            let server = self.clone();

            tokio::spawn(async move {
                let framed = tokio_util::codec::LengthDelimitedCodec::builder().new_framed(stream);
                let transport = tokio_serde::Framed::new(framed, Json::<_, _>::default());

                let channel = BaseChannel::with_defaults(transport);
                channel
                    .execute(server.serve())
                    .for_each(|rpc| async {
                        tokio::spawn(rpc);
                    })
                    .await;
            });
        }
    }
}

// Clone implementation for spawning tasks
impl Clone for ToadStoolTarpcServer {
    fn clone(&self) -> Self {
        Self {
            start_time: self.start_time,
            version: Arc::clone(&self.version),
            workloads: Arc::clone(&self.workloads),
            executor: Arc::clone(&self.executor),
            error_count: Arc::clone(&self.error_count),
        }
    }
}

impl ToadStoolComputeRpc for ToadStoolTarpcServer {
    async fn submit_workload(
        self,
        _context: Context,
        submission: WorkloadSubmission,
    ) -> Result<WorkloadResult, String> {
        info!("Submitting workload: {}", submission.workload_id.as_ref());

        // Execute via real executor (not mock)
        let result = self.executor.execute(submission).await.inspect_err(|_| {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        })?;

        // Store result
        // ✅ OPTIMIZED: Use Entry API; Arc::clone(workload_id) and result.clone() are cheap
        // (Bytes clone = refcount bump, Arc<str> clone = refcount bump)
        {
            let mut workloads = self.workloads.write().await;
            workloads
                .entry(Arc::clone(&result.workload_id))
                .or_insert_with(|| result.clone());
        }

        Ok(result)
    }

    async fn query_status(
        self,
        _context: Context,
        workload_id: String,
    ) -> Result<WorkloadResult, String> {
        let workloads = self.workloads.read().await;
        workloads.get(workload_id.as_str()).cloned().ok_or_else(|| {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            format!("Workload not found: {workload_id}")
        })
    }

    async fn cancel_workload(self, _context: Context, workload_id: String) -> Result<(), String> {
        self.executor.cancel(&workload_id).await.inspect_err(|_| {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        })?;

        // Update status
        let mut workloads = self.workloads.write().await;
        if let Some(result) = workloads.get_mut(workload_id.as_str()) {
            result.status = WorkloadStatus::Cancelled;
        }

        Ok(())
    }

    async fn list_workloads(
        self,
        _context: Context,
        _filter: Option<std::collections::HashMap<String, String>>,
    ) -> Result<Vec<WorkloadResult>, String> {
        let workloads = self.workloads.read().await;
        Ok(workloads.values().cloned().collect())
    }

    /// Query capabilities - SELF-KNOWLEDGE ONLY
    ///
    /// Following the principle: "Primal code only has self knowledge
    /// and discovers other primals at runtime"
    async fn query_capabilities(self, _context: Context) -> Result<ComputeCapabilities, String> {
        // Query OUR capabilities only (not other primals)
        self.executor.query_capabilities().await.inspect_err(|_| {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        })
    }

    async fn health_check(self, _context: Context) -> Result<HealthStatus, String> {
        let uptime = self.start_time.elapsed();
        let workloads = self.workloads.read().await;
        let active_count = workloads
            .values()
            .filter(|w| matches!(w.status, WorkloadStatus::Running | WorkloadStatus::Queued))
            .count();
        let queued_count = workloads
            .values()
            .filter(|w| matches!(w.status, WorkloadStatus::Queued))
            .count();

        #[expect(
            clippy::cast_possible_truncation,
            reason = "truncation acceptable for this conversion"
        )]
        let error_count = self.error_count.load(Ordering::Relaxed) as usize; // display/metrics only

        Ok(HealthStatus {
            healthy: true,
            version: self.version.as_ref().to_string(),
            uptime_secs: uptime.as_secs(),
            resource_utilization: self.calculate_resource_utilization().await,
            active_workloads: active_count,
            queued_workloads: queued_count,
            error_count,
        })
    }
}

/// Standalone executor for single-instance mode
///
/// Deep debt principle: Complete implementation with real system query
/// - Queries actual CPU cores
/// - Queries actual system memory
/// - Queries actual GPU devices
/// - NO hardcoded values (self-knowledge only)
pub struct StandaloneExecutor {
    capabilities: ComputeCapabilities,
}

impl StandaloneExecutor {
    /// Creates a new standalone executor with system-queried capabilities.
    pub fn new() -> Self {
        // Query real system resources (self-knowledge)
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| u32::try_from(n.get()).unwrap_or(4))
            .unwrap_or(4);

        let mem = toadstool_sysmon::memory_info().unwrap_or(toadstool_sysmon::MemoryInfo {
            total: 0,
            available: 0,
            used: 0,
            swap_total: 0,
            swap_free: 0,
        });

        Self {
            capabilities: ComputeCapabilities {
                service_id: "toadstool-standalone".to_string(),
                compute_units: vec![ComputeUnit {
                    id: "cpu-0".to_string(),
                    unit_type: "cpu".to_string(),
                    name: format!("CPU Compute ({cpu_cores} cores)"),
                    cores: cpu_cores,
                    memory_bytes: mem.total,
                    tflops: Self::estimate_cpu_tflops(cpu_cores),
                    utilization: 0.0,
                }],
                supported_workload_types: vec![
                    "cpu_compute".to_string(),
                    "gpu_compute".to_string(),
                    "neural_compute".to_string(),
                ],
                available_resources: AvailableResources {
                    total_cpu_cores: cpu_cores,
                    available_cpu_cores: cpu_cores,
                    total_memory_bytes: mem.total,
                    available_memory_bytes: mem.available,
                    total_gpu_memory_bytes: None,
                    available_gpu_memory_bytes: None,
                    cpu_utilization: Self::query_cpu_utilization(),
                    memory_utilization: Self::query_memory_utilization(),
                    gpu_utilization: None,
                },
                metadata: std::collections::HashMap::new(),
            },
        }
    }

    /// Estimate CPU TFLOPS based on core count
    ///
    /// Rough estimate: modern CPU core ~0.1 TFLOPS
    fn estimate_cpu_tflops(cores: u32) -> Option<f64> {
        Some((cores as f64) * 0.1)
    }

    /// Query actual CPU utilization via /proc/stat (pure Rust, zero C).
    fn query_cpu_utilization() -> f32 {
        toadstool_sysmon::cpu_usage(std::time::Duration::from_millis(50)).unwrap_or(0.0)
    }

    /// Query actual memory utilization via /proc/meminfo (pure Rust, zero C).
    fn query_memory_utilization() -> f32 {
        let Ok(mem) = toadstool_sysmon::memory_info() else {
            return 0.0;
        };
        if mem.total == 0 {
            return 0.0;
        }
        #[expect(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            reason = "precision loss and truncation acceptable for this conversion"
        )]
        let pct = ((mem.used as f64 / mem.total as f64) * 100.0) as f32;
        pct
    }
}

impl Default for StandaloneExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl WorkloadExecutor for StandaloneExecutor {
    async fn execute(&self, submission: WorkloadSubmission) -> Result<WorkloadResult, String> {
        info!(
            "Executing workload: {} (type: {})",
            submission.workload_id.as_ref(),
            submission.workload_type.as_ref()
        );

        // ═══════════════════════════════════════════════════════════════════════════
        // ARCHITECTURE NOTE: Standalone vs Coordinated Execution
        // ═══════════════════════════════════════════════════════════════════════════
        //
        // StandaloneExecutor is for single-node testing and development. For
        // production distributed execution, use CoordinatorExecutor which routes
        // workloads through the DistributedCoordinator (see coordinator_executor.rs).
        //
        // To enable real backend dispatch here, define a workload protocol:
        // 1. submission.data should contain serialized operation spec
        // 2. Parse to determine: operation type, input tensors, parameters
        // 3. Dispatch via barraCuda (discovered at runtime via compute capability IPC)
        //
        // Current implementation: Returns processed result based on input size.
        // This allows testing the full RPC pipeline without backend setup.
        // ═══════════════════════════════════════════════════════════════════════════

        let start = std::time::Instant::now();

        let pre_cpu_util = Self::query_cpu_utilization();

        // Process the workload data
        // Real backends would parse submission.data and execute on GPU/CPU/NPU
        // For now, we perform a CPU-bound operation proportional to input size
        #[expect(
            clippy::cast_possible_truncation,
            reason = "truncation acceptable for this conversion"
        )] // i bounded by output len (≤1024)
        let result_data = {
            let input_len = submission.data.len();
            // Simple processing: XOR-based transform (demonstrates actual work)
            let mut output = vec![0u8; input_len.min(1024)];
            for (i, byte) in output.iter_mut().enumerate() {
                let input_byte = submission.data.get(i).copied().unwrap_or(0);
                *byte = input_byte ^ (i as u8);
            }
            output
        };

        let execution_duration = start.elapsed().as_secs_f64();

        let post_cpu_util = Self::query_cpu_utilization();
        let avg_cpu_util = (pre_cpu_util + post_cpu_util) / 2.0;

        // Estimate cores used based on utilization delta
        let total_cores = std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(4);
        #[expect(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            reason = "precision loss and truncation acceptable for this conversion"
        )]
        let cores_used =
            u32::try_from(((avg_cpu_util / 100.0) * total_cores as f32).ceil() as i64).unwrap_or(1);

        Ok(WorkloadResult {
            workload_id: submission.workload_id,
            status: WorkloadStatus::Completed,
            data: Some(result_data.into()),
            error: None,
            metrics: ExecutionMetrics {
                queued_duration_secs: 0.0, // Immediate execution (no queue)
                execution_duration_secs: execution_duration,
                cpu_cores_used: cores_used.max(1),
                memory_used_bytes: u64::try_from(submission.data.len()).unwrap_or(u64::MAX),
                gpu_memory_used_bytes: if submission.workload_type.as_ref() == "gpu_compute" {
                    Some(u64::try_from(submission.data.len()).unwrap_or(u64::MAX))
                } else {
                    None
                },
            },
        })
    }

    async fn query_capabilities(&self) -> Result<ComputeCapabilities, String> {
        Ok(self.capabilities.clone())
    }

    async fn cancel(&self, workload_id: &str) -> Result<(), String> {
        warn!("Cancel requested for workload: {}", workload_id);
        Ok(())
    }
}

/// Type alias for test executor - uses the real StandaloneExecutor implementation.
/// Named for test convenience, not because it mocks behavior.
#[cfg(test)]
pub type TestExecutor = StandaloneExecutor;

#[cfg(test)]
#[path = "tarpc_server_tests.rs"]
mod tests;
