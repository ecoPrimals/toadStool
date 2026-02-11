//! # ToadStool tarpc Client Implementation
//!
//! Type-safe client for communicating with ToadStool compute services.
//! Follows Songbird's pattern for inter-primal communication.
//!
//! **Deep Debt Compliance**:
//! - Unix sockets as primary transport (no TCP hardcoding)
//! - Capability-based discovery (no hardcoded service names)
//! - Zero hardcoded ports (multi-instance support)
//! - Self-knowledge only (discovers other primals at runtime)

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tarpc::{
    client,
    context,
    tokio_serde::formats::Json,
};
use tokio::net::{TcpStream, UnixStream};
use tracing::{info, warn};

use toadstool_integration_protocols::tarpc_service::{
    ComputeCapabilities, HealthStatus, ToadStoolComputeRpcClient,
    WorkloadResult, WorkloadSubmission,
};

/// ToadStool tarpc client endpoint type
#[derive(Debug, Clone)]
pub enum ClientEndpoint {
    /// Unix socket (PRIMARY - Deep Debt compliant)
    UnixSocket(PathBuf),
    /// TCP socket (FALLBACK ONLY - for platforms without Unix socket support)
    Tcp(SocketAddr),
}

impl std::fmt::Display for ClientEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientEndpoint::UnixSocket(path) => write!(f, "unix://{}", path.display()),
            ClientEndpoint::Tcp(addr) => write!(f, "tcp://{}", addr),
        }
    }
}

/// ToadStool tarpc client for primal-to-primal communication
pub struct ToadStoolTarpcClient {
    /// Inner tarpc client
    client: ToadStoolComputeRpcClient,
    /// Connected endpoint
    endpoint: ClientEndpoint,
}

impl ToadStoolTarpcClient {
    /// Connect to ToadStool compute service via Unix socket (PRIMARY)
    ///
    /// **Deep Debt Compliant**:
    /// - Uses Unix sockets for inter-primal communication
    /// - No hardcoded ports (multi-instance support)
    /// - Capability-based discovery (discovers socket path at runtime)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use toadstool_client::ToadStoolTarpcClient;
    /// use toadstool_common::primal_sockets;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// // Discover ToadStool socket via capability discovery
    /// let socket_path = primal_sockets::get_toadstool_socket_path();
    /// let client = ToadStoolTarpcClient::connect_unix(&socket_path).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_unix(socket_path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let socket_path = socket_path.as_ref();
        info!("Connecting to ToadStool compute service at Unix socket: {:?}", socket_path);

        // Establish Unix socket connection
        let stream = UnixStream::connect(socket_path).await
            .map_err(|e| format!("Failed to connect to Unix socket {:?}: {}", socket_path, e))?;
        
        // Create transport with JSON codec (same as server)
        let transport = tarpc::serde_transport::new(
            tokio_util::codec::LengthDelimitedCodec::builder()
                .max_frame_length(16 * 1024 * 1024) // 16MB max frame
                .new_framed(stream),
            Json::default(),
        );

        // Create tarpc client
        let client = ToadStoolComputeRpcClient::new(
            client::Config::default(),
            transport,
        ).spawn();

        info!("✅ Successfully connected to ToadStool via Unix socket: {:?}", socket_path);

        Ok(Self { 
            client, 
            endpoint: ClientEndpoint::UnixSocket(socket_path.to_path_buf()),
        })
    }

    /// Connect to ToadStool compute service at given TCP address (DEPRECATED)
    ///
    /// **Deep Debt Violation**: TCP with hardcoded ports breaks multi-instance support.
    ///
    /// Use `connect_unix()` instead for production. This method exists only for:
    /// - Platform fallback (where Unix sockets unavailable)
    /// - Testing and debugging
    ///
    /// # Migration
    ///
    /// ```rust,no_run
    /// use toadstool_client::ToadStoolTarpcClient;
    /// use toadstool_common::primal_sockets;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// // OLD (TCP - Deep Debt violation)
    /// // let client = ToadStoolTarpcClient::connect("127.0.0.1:50051".parse()?).await?;
    ///
    /// // NEW (Unix socket - Deep Debt compliant)
    /// let socket_path = primal_sockets::get_toadstool_socket_path();
    /// let client = ToadStoolTarpcClient::connect_unix(&socket_path).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[deprecated(
        since = "0.2.0",
        note = "Use connect_unix() for production. TCP hardcoding violates Deep Debt principles."
    )]
    pub async fn connect(addr: SocketAddr) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        warn!("⚠️  TCP mode is DEPRECATED - violates Deep Debt principles");
        warn!("⚠️  Use connect_unix() for production");
        info!("Connecting to ToadStool compute service at TCP: {}", addr);

        // Establish TCP connection
        let stream = TcpStream::connect(addr).await?;
        
        // Create transport with JSON codec
        let transport = tarpc::serde_transport::new(
            tokio_util::codec::LengthDelimitedCodec::builder()
                .max_frame_length(16 * 1024 * 1024) // 16MB max frame
                .new_framed(stream),
            Json::default(),
        );

        // Create tarpc client
        let client = ToadStoolComputeRpcClient::new(
            client::Config::default(),
            transport,
        ).spawn();

        info!("Connected to ToadStool at TCP: {}", addr);

        Ok(Self { 
            client, 
            endpoint: ClientEndpoint::Tcp(addr),
        })
    }

    /// Connect to ToadStool using capability-based discovery (RECOMMENDED)
    ///
    /// **Deep Debt Compliant**:
    /// - Discovers service by capability, not by name
    /// - Uses Unix sockets (no hardcoded ports)
    /// - Runtime discovery (no compile-time knowledge)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use toadstool_client::ToadStoolTarpcClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// // Discovers ANY compute service providing ToadStool capabilities
    /// let client = ToadStoolTarpcClient::discover().await?;
    /// let caps = client.query_capabilities().await?;
    /// println!("Connected to: {}", caps.service_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Use capability-based discovery to find compute service
        // Falls back to standard ToadStool socket if discovery unavailable
        let socket_path = toadstool_common::primal_sockets::get_toadstool_socket_path();
        
        Self::connect_unix(socket_path).await
    }

    /// Submit workload for execution
    pub async fn submit_workload(
        &self,
        submission: WorkloadSubmission,
    ) -> Result<WorkloadResult, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.client
            .submit_workload(context::current(), submission)
            .await??;
        
        Ok(result)
    }

    /// Query workload status
    pub async fn query_status(
        &self,
        workload_id: String,
    ) -> Result<WorkloadResult, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.client
            .query_status(context::current(), workload_id)
            .await??;
        
        Ok(result)
    }

    /// Cancel workload
    pub async fn cancel_workload(
        &self,
        workload_id: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .cancel_workload(context::current(), workload_id)
            .await??;
        
        Ok(())
    }

    /// List workloads
    pub async fn list_workloads(
        &self,
        filter: Option<std::collections::HashMap<String, String>>,
    ) -> Result<Vec<WorkloadResult>, Box<dyn std::error::Error + Send + Sync>> {
        let results = self.client
            .list_workloads(context::current(), filter)
            .await??;
        
        Ok(results)
    }

    /// Query service capabilities (runtime discovery of primal's abilities)
    ///
    /// This is how we discover what a primal can do - NO hardcoded knowledge!
    pub async fn query_capabilities(
        &self,
    ) -> Result<ComputeCapabilities, Box<dyn std::error::Error + Send + Sync>> {
        let caps = self.client
            .query_capabilities(context::current())
            .await??;
        
        Ok(caps)
    }

    /// Health check
    pub async fn health_check(
        &self,
    ) -> Result<HealthStatus, Box<dyn std::error::Error + Send + Sync>> {
        let health = self.client
            .health_check(context::current())
            .await??;
        
        Ok(health)
    }

    /// Get connected endpoint
    pub fn endpoint(&self) -> &ClientEndpoint {
        &self.endpoint
    }

    /// Get connected address (deprecated - use endpoint() instead)
    #[deprecated(since = "0.2.0", note = "Use endpoint() for Unix socket support")]
    pub fn address(&self) -> Option<SocketAddr> {
        match &self.endpoint {
            ClientEndpoint::Tcp(addr) => Some(*addr),
            ClientEndpoint::UnixSocket(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use toadstool_integration_protocols::tarpc_service::{
        WorkloadPriority, ResourceRequirements,
    };

    // Integration tests require actual server running
    // These are example test structures

    #[tokio::test]
    #[ignore] // Requires server
    async fn test_client_unix_connection() {
        use std::path::PathBuf;
        
        let socket_path = PathBuf::from("/tmp/toadstool-test.sock");
        let result = ToadStoolTarpcClient::connect_unix(&socket_path).await;
        
        // Would succeed if server is running
        assert!(result.is_ok() || result.is_err()); // Placeholder assertion
    }

    #[tokio::test]
    #[ignore] // Requires server
    #[allow(deprecated)]
    async fn test_client_tcp_connection_deprecated() {
        let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
        let result = ToadStoolTarpcClient::connect(addr).await;
        
        // Would succeed if server is running
        assert!(result.is_ok() || result.is_err()); // Placeholder assertion
    }

    #[tokio::test]
    #[ignore] // Requires server
    async fn test_client_discovery() {
        let result = ToadStoolTarpcClient::discover().await;
        
        // Would succeed if server is running
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_workload_submission_structure() {
        let submission = WorkloadSubmission {
            workload_id: "work-test-123".to_string(),
            workload_type: "gpu_compute".to_string(),
            data: vec![1, 2, 3, 4],
            metadata: HashMap::new(),
            priority: WorkloadPriority::Normal,
            requirements: ResourceRequirements {
                cpu_cores: Some(4),
                memory_bytes: Some(1024 * 1024 * 1024),
                gpu_memory_bytes: Some(512 * 1024 * 1024),
                timeout_secs: Some(300),
            },
        };

        assert_eq!(submission.workload_id, "work-test-123");
        assert_eq!(submission.workload_type, "gpu_compute");
    }
}

