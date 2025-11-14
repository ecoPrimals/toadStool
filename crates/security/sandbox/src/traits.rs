//! Sandbox manager trait definitions

use async_trait::async_trait;
use toadstool::error::ToadStoolResult;
use toadstool_security_policies::SecurityPolicy;

use super::types::*;

#[async_trait]
pub trait SandboxManager: Send + Sync {
    /// Create a new sandbox
    async fn create_sandbox(&self, spec: SandboxSpec) -> ToadStoolResult<String>;

    /// Start execution in sandbox
    async fn start_execution(&self, sandbox_id: &str) -> ToadStoolResult<()>;

    /// Stop execution in sandbox
    async fn stop_execution(&self, sandbox_id: &str) -> ToadStoolResult<()>;

    /// Destroy sandbox
    async fn destroy_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<()>;

    /// Get sandbox information
    async fn get_sandbox_info(&self, sandbox_id: &str) -> ToadStoolResult<SandboxInfo>;

    /// List all sandboxes
    async fn list_sandboxes(&self) -> ToadStoolResult<Vec<String>>;

    /// Monitor sandbox resource usage
    async fn monitor_sandbox(&self, sandbox_id: &str) -> ToadStoolResult<ResourceUsage>;

    /// Apply security policy to sandbox
    async fn apply_security_policy(
        &self,
        sandbox_id: &str,
        policy: &SecurityPolicy,
    ) -> ToadStoolResult<()>;

    /// Get sandbox logs
    async fn get_sandbox_logs(&self, sandbox_id: &str) -> ToadStoolResult<Vec<String>>;
}
