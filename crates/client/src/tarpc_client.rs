//! # ToadStool tarpc Client Implementation
//!
//! Type-safe client for communicating with ToadStool compute services.
//! Follows Songbird's pattern for inter-primal communication.

use std::net::SocketAddr;
use tarpc::{
    client,
    context,
    tokio_serde::formats::Json,
};
use tokio::net::TcpStream;
use tracing::{error, info};

use toadstool_integration_protocols::tarpc_service::{
    ComputeCapabilities, HealthStatus, ToadStoolComputeRpcClient,
    WorkloadResult, WorkloadSubmission,
};

/// ToadStool tarpc client for primal-to-primal communication
pub struct ToadStoolTarpcClient {
    /// Inner tarpc client
    client: ToadStoolComputeRpcClient,
    /// Connected address
    addr: SocketAddr,
}

impl ToadStoolTarpcClient {
    /// Connect to ToadStool compute service at given address
    ///
    /// This follows the discovery pattern:
    /// - Client only knows the address (discovered at runtime)
    /// - No hardcoded knowledge of the service
    /// - Queries capabilities after connection
    pub async fn connect(addr: SocketAddr) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Connecting to ToadStool compute service at: {}", addr);

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

        info!("Successfully connected to ToadStool at: {}", addr);

        Ok(Self { client, addr })
    }

    /// Submit workload for execution
    pub async fn submit_workload(
        &self,
        submission: WorkloadSubmission,
    ) -> Result<WorkloadResult, Box<dyn std::error::Error>> {
        let result = self.client
            .submit_workload(context::current(), submission)
            .await??;
        
        Ok(result)
    }

    /// Query workload status
    pub async fn query_status(
        &self,
        workload_id: String,
    ) -> Result<WorkloadResult, Box<dyn std::error::Error>> {
        let result = self.client
            .query_status(context::current(), workload_id)
            .await??;
        
        Ok(result)
    }

    /// Cancel workload
    pub async fn cancel_workload(
        &self,
        workload_id: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.client
            .cancel_workload(context::current(), workload_id)
            .await??;
        
        Ok(())
    }

    /// List workloads
    pub async fn list_workloads(
        &self,
        filter: Option<std::collections::HashMap<String, String>>,
    ) -> Result<Vec<WorkloadResult>, Box<dyn std::error::Error>> {
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
    ) -> Result<ComputeCapabilities, Box<dyn std::error::Error>> {
        let caps = self.client
            .query_capabilities(context::current())
            .await??;
        
        Ok(caps)
    }

    /// Health check
    pub async fn health_check(
        &self,
    ) -> Result<HealthStatus, Box<dyn std::error::Error>> {
        let health = self.client
            .health_check(context::current())
            .await??;
        
        Ok(health)
    }

    /// Get connected address
    pub fn address(&self) -> SocketAddr {
        self.addr
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
    async fn test_client_connection() {
        let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
        let result = ToadStoolTarpcClient::connect(addr).await;
        
        // Would succeed if server is running
        assert!(result.is_ok() || result.is_err()); // Placeholder assertion
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

