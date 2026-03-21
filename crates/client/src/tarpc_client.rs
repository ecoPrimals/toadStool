// SPDX-License-Identifier: AGPL-3.0-only
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
use thiserror::Error;

/// Error type for ToadStool tarpc client operations.
#[derive(Debug, Error)]
pub enum TarpcClientError {
    /// Failed to establish transport connection (Unix socket or TCP)
    #[error("Connection failed: {0}")]
    Connection(String),
    /// RPC transport-layer error from tarpc
    #[error("RPC transport error: {0}")]
    Transport(#[from] tarpc::client::RpcError),
    /// Service-level error returned by the remote handler
    #[error("Service error: {0}")]
    Service(String),
    /// Capability-based discovery failed to locate a compute service
    #[error("Discovery failed: {0}")]
    Discovery(String),
}
use std::path::{Path, PathBuf};
use tarpc::{client, context, tokio_serde::formats::Json};
use tokio::net::{TcpStream, UnixStream};
use tracing::{info, warn};

use toadstool_integration_protocols::tarpc_service::{
    ComputeCapabilities, HealthStatus, ToadStoolComputeRpcClient, WorkloadResult,
    WorkloadSubmission,
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
            Self::UnixSocket(path) => write!(f, "unix://{}", path.display()),
            Self::Tcp(addr) => write!(f, "tcp://{}", addr),
        }
    }
}

/// ToadStool tarpc client for primal-to-primal communication
#[derive(Debug)]
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
    /// use toadstool_client::{ToadStoolTarpcClient, TarpcClientError};
    /// use toadstool_common::primal_sockets;
    ///
    /// # async fn example() -> Result<(), TarpcClientError> {
    /// // Discover ToadStool socket via capability discovery
    /// let socket_path = primal_sockets::get_toadstool_socket_path();
    /// let client = ToadStoolTarpcClient::connect_unix(&socket_path).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect_unix(socket_path: impl AsRef<Path>) -> Result<Self, TarpcClientError> {
        let socket_path = socket_path.as_ref();
        info!(
            "Connecting to ToadStool compute service at Unix socket: {:?}",
            socket_path
        );

        // Establish Unix socket connection
        let stream = UnixStream::connect(socket_path).await.map_err(|e| {
            TarpcClientError::Connection(format!(
                "Failed to connect to Unix socket {:?}: {}",
                socket_path, e
            ))
        })?;

        // Create transport with JSON codec (same as server)
        let transport = tarpc::serde_transport::new(
            tokio_util::codec::LengthDelimitedCodec::builder()
                .max_frame_length(16 * 1024 * 1024) // 16MB max frame
                .new_framed(stream),
            Json::default(),
        );

        // Create tarpc client
        let client = ToadStoolComputeRpcClient::new(client::Config::default(), transport).spawn();

        info!(
            "✅ Successfully connected to ToadStool via Unix socket: {:?}",
            socket_path
        );

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
    /// use toadstool_client::{ToadStoolTarpcClient, TarpcClientError};
    /// use toadstool_common::primal_sockets;
    ///
    /// # async fn example() -> Result<(), TarpcClientError> {
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
    pub async fn connect(addr: SocketAddr) -> Result<Self, TarpcClientError> {
        warn!("⚠️  TCP mode is DEPRECATED - violates Deep Debt principles");
        warn!("⚠️  Use connect_unix() for production");
        info!("Connecting to ToadStool compute service at TCP: {}", addr);

        // Establish TCP connection
        let stream = TcpStream::connect(addr).await.map_err(|e| {
            TarpcClientError::Connection(format!("Failed to connect to TCP {}: {}", addr, e))
        })?;

        // Create transport with JSON codec
        let transport = tarpc::serde_transport::new(
            tokio_util::codec::LengthDelimitedCodec::builder()
                .max_frame_length(16 * 1024 * 1024) // 16MB max frame
                .new_framed(stream),
            Json::default(),
        );

        // Create tarpc client
        let client = ToadStoolComputeRpcClient::new(client::Config::default(), transport).spawn();

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
    /// use toadstool_client::{ToadStoolTarpcClient, TarpcClientError};
    ///
    /// # async fn example() -> Result<(), TarpcClientError> {
    /// // Discovers ANY compute service providing ToadStool capabilities
    /// let client = ToadStoolTarpcClient::discover().await?;
    /// let caps = client.query_capabilities().await?;
    /// println!("Connected to: {}", caps.service_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover() -> Result<Self, TarpcClientError> {
        // Use capability-based discovery to find compute service
        // Falls back to standard ToadStool socket if discovery unavailable
        let socket_path = toadstool_common::primal_sockets::get_toadstool_socket_path();
        Self::connect_unix(socket_path)
            .await
            .map_err(|e| TarpcClientError::Discovery(e.to_string()))
    }

    /// Submit workload for execution
    pub async fn submit_workload(
        &self,
        submission: WorkloadSubmission,
    ) -> Result<WorkloadResult, TarpcClientError> {
        self.client
            .submit_workload(context::current(), submission)
            .await
            .map_err(TarpcClientError::from)?
            .map_err(TarpcClientError::Service)
    }

    /// Query workload status
    pub async fn query_status(
        &self,
        workload_id: String,
    ) -> Result<WorkloadResult, TarpcClientError> {
        self.client
            .query_status(context::current(), workload_id)
            .await
            .map_err(TarpcClientError::from)?
            .map_err(TarpcClientError::Service)
    }

    /// Cancel workload
    pub async fn cancel_workload(&self, workload_id: String) -> Result<(), TarpcClientError> {
        self.client
            .cancel_workload(context::current(), workload_id)
            .await
            .map_err(TarpcClientError::from)?
            .map_err(TarpcClientError::Service)
    }

    /// List workloads
    pub async fn list_workloads(
        &self,
        filter: Option<std::collections::HashMap<String, String>>,
    ) -> Result<Vec<WorkloadResult>, TarpcClientError> {
        self.client
            .list_workloads(context::current(), filter)
            .await
            .map_err(TarpcClientError::from)?
            .map_err(TarpcClientError::Service)
    }

    /// Query service capabilities (runtime discovery of primal's abilities)
    ///
    /// This is how we discover what a primal can do - NO hardcoded knowledge!
    pub async fn query_capabilities(&self) -> Result<ComputeCapabilities, TarpcClientError> {
        self.client
            .query_capabilities(context::current())
            .await
            .map_err(TarpcClientError::from)?
            .map_err(TarpcClientError::Service)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthStatus, TarpcClientError> {
        self.client
            .health_check(context::current())
            .await
            .map_err(TarpcClientError::from)?
            .map_err(TarpcClientError::Service)
    }

    /// Get connected endpoint
    pub const fn endpoint(&self) -> &ClientEndpoint {
        &self.endpoint
    }

    /// Get connected address (deprecated - use endpoint() instead)
    #[deprecated(since = "0.2.0", note = "Use endpoint() for Unix socket support")]
    pub const fn address(&self) -> Option<SocketAddr> {
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
    use std::sync::Arc;
    use toadstool_integration_protocols::tarpc_service::{ResourceRequirements, WorkloadPriority};

    // Integration tests require actual server running
    // These are example test structures

    #[tokio::test]
    async fn test_client_unix_connection_no_server() {
        use std::path::PathBuf;

        let socket_path = PathBuf::from("/tmp/toadstool-nonexistent-test.sock");
        let result = ToadStoolTarpcClient::connect_unix(&socket_path).await;

        let err = result.expect_err("should fail when no server is listening");
        assert!(
            matches!(err, TarpcClientError::Connection(_)),
            "expected Connection error, got: {err}"
        );
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_client_tcp_connection_no_server() {
        let addr: SocketAddr = "127.0.0.1:1".parse().expect("valid addr");
        let result = ToadStoolTarpcClient::connect(addr).await;

        let err = result.expect_err("should fail when no server is listening");
        assert!(
            matches!(err, TarpcClientError::Connection(_)),
            "expected Connection error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_client_discovery_no_server() {
        let result = ToadStoolTarpcClient::discover().await;

        let err = result.expect_err("should fail when no server is listening");
        assert!(
            matches!(err, TarpcClientError::Discovery(_)),
            "expected Discovery error, got: {err}"
        );
    }

    #[test]
    fn test_workload_submission_structure() {
        let submission = WorkloadSubmission {
            workload_id: Arc::from("work-test-123"),
            workload_type: Arc::from("gpu_compute"),
            data: bytes::Bytes::from(vec![1, 2, 3, 4]),
            metadata: HashMap::new(),
            priority: WorkloadPriority::Normal,
            requirements: ResourceRequirements {
                cpu_cores: Some(4),
                memory_bytes: Some(1024 * 1024 * 1024),
                gpu_memory_bytes: Some(512 * 1024 * 1024),
                timeout_secs: Some(300),
            },
        };

        assert_eq!(&*submission.workload_id, "work-test-123");
        assert_eq!(&*submission.workload_type, "gpu_compute");
    }
}
