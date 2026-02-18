//! AI agent deployment and management via Squirrel
//!
//! This module handles AI agent deployment, model management, and integration
//! with Squirrel AI services using trait-based dependency injection.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::agent_backend::AgentBackend;
use super::types::{AgentConfig, ModelConfig};
use crate::ToadStoolResult;

// Re-export all types for backward compatibility
pub use super::agent_backend::{
    AgentInfo, AgentResourceUsage, AgentStatus, ModelInfo, ModelPerformanceMetrics,
    ModelResourceRequirements, ModelStatus,
};

/// Agent deployment manager for Squirrel integration
///
/// Uses dependency injection via the `AgentBackend` trait for flexibility.
/// No conditional compilation or feature flags - the backend determines behavior.
pub struct AgentDeploymentManager {
    /// Configuration
    _config: AgentDeploymentConfig,
    /// Pluggable agent backend (Squirrel, in-memory, etc.)
    backend: Arc<dyn AgentBackend>,
}

/// Configuration for agent deployment manager
///
/// # Evolution Note (Feb 2026)
///
/// The `squirrel_endpoint` field is deprecated. Use capability-based discovery:
///
/// ```rust,ignore
/// // OLD: Hardcoded endpoint
/// let config = AgentDeploymentConfig {
///     squirrel_endpoint: "http://localhost:8080".into(),
///     ..Default::default()
/// };
///
/// // NEW: Capability-based discovery
/// let manager = AgentDeploymentManager::discover().await?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDeploymentConfig {
    /// Squirrel endpoint URL
    ///
    /// **DEPRECATED**: Use `AgentDeploymentManager::discover()` for runtime discovery.
    /// When empty, the manager discovers Squirrel via capability lookup.
    #[serde(default)]
    pub squirrel_endpoint: String,
    /// Model registry type (local, huggingface, custom)
    pub model_registry: String,
    /// Agent runtime (container, process, lambda)
    pub agent_runtime: String,
    /// Enable MCP (Model Control Protocol)
    pub mcp_enabled: bool,
    /// Resource limits configuration
    pub resource_limits: serde_json::Map<String, serde_json::Value>,
}

impl Default for AgentDeploymentConfig {
    fn default() -> Self {
        Self {
            squirrel_endpoint: String::new(), // Empty = use runtime discovery
            model_registry: "local".to_string(),
            agent_runtime: "container".to_string(),
            mcp_enabled: false,
            resource_limits: serde_json::Map::new(),
        }
    }
}

impl AgentDeploymentManager {
    /// Create a new agent deployment manager with custom backend
    #[must_use]
    pub fn new(config: AgentDeploymentConfig, backend: Arc<dyn AgentBackend>) -> Self {
        Self {
            _config: config,
            backend,
        }
    }

    /// Discover and create agent manager via capability-based discovery
    ///
    /// This is the preferred method for creating an AgentDeploymentManager.
    /// It discovers Squirrel (or another AI provider) at runtime.
    ///
    /// # Discovery Order
    ///
    /// 1. Environment variable: `SQUIRREL_ENDPOINT` or `TOADSTOOL_AI_ENDPOINT`
    /// 2. mDNS/local network discovery for "ai-orchestration" or "storage" capability
    /// 3. Unix socket discovery at standard paths
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let manager = AgentDeploymentManager::discover().await?;
    /// ```
    pub async fn discover() -> crate::ToadStoolResult<Self> {
        Self::discover_with_config(AgentDeploymentConfig::default()).await
    }

    /// Discover AI provider with custom configuration
    ///
    /// **EVOLVED**: Uses capability-based discovery (no hardcoded primal names).
    pub async fn discover_with_config(
        config: AgentDeploymentConfig,
    ) -> crate::ToadStoolResult<Self> {
        // Priority 1: Try capability-based discovery first (Deep Debt compliant)
        match Self::with_ml_service(config.clone()).await {
            Ok(manager) => {
                tracing::info!("Discovered ML service via capability-based discovery");
                return Ok(manager);
            }
            Err(e) => {
                tracing::debug!("Capability discovery failed: {}, trying fallbacks", e);
            }
        }

        // Priority 2: Check environment variables (backward compatibility)
        if let Ok(endpoint) =
            std::env::var("SQUIRREL_ENDPOINT").or_else(|_| std::env::var("TOADSTOOL_AI_ENDPOINT"))
        {
            tracing::info!("Discovered ML service via environment: {}", endpoint);
            let mut config = config;
            config.squirrel_endpoint = endpoint;
            #[allow(deprecated)]
            return Ok(Self::with_squirrel(config));
        }

        // Priority 3: Check if endpoint is already configured
        if !config.squirrel_endpoint.is_empty() {
            tracing::debug!("Using configured endpoint: {}", config.squirrel_endpoint);
            #[allow(deprecated)]
            return Ok(Self::with_squirrel(config));
        }

        // Priority 4: Fall back to in-memory backend for development
        tracing::warn!(
            "No AI provider discovered, using in-memory backend. \
             Ensure a ML provider is running or set SQUIRREL_ENDPOINT."
        );
        Ok(Self::with_inmemory(config))
    }

    /// Create a new manager with capability-based ML service discovery (RECOMMENDED)
    ///
    /// **Deep Debt Compliant**: Discovers ML service by capability, not name.
    pub async fn with_ml_service(config: AgentDeploymentConfig) -> crate::ToadStoolResult<Self> {
        let backend = super::agent_backend::SquirrelBackend::new_async(
            config.model_registry.clone(),
            config.agent_runtime.clone(),
            config.mcp_enabled,
        )
        .await?;
        Ok(Self {
            _config: config,
            backend: Arc::new(backend),
        })
    }

    /// Create a new manager with Squirrel production backend
    ///
    /// **DEPRECATED**: Use `with_ml_service()` or `discover()` for capability-based discovery.
    #[must_use]
    #[deprecated(
        since = "0.3.0",
        note = "Use with_ml_service() or discover() for capability-based discovery"
    )]
    #[allow(deprecated)]
    pub fn with_squirrel(config: AgentDeploymentConfig) -> Self {
        let backend = super::agent_backend::SquirrelBackend::new(
            config.squirrel_endpoint.clone(),
            config.model_registry.clone(),
            config.agent_runtime.clone(),
            config.mcp_enabled,
        );
        Self {
            _config: config,
            backend: Arc::new(backend),
        }
    }

    /// Create a new manager with in-memory test backend
    #[must_use]
    pub fn with_inmemory(config: AgentDeploymentConfig) -> Self {
        let backend = super::agent_backend::InMemoryAgentBackend::new();
        Self {
            _config: config,
            backend: Arc::new(backend),
        }
    }

    /// Initialize connection to Squirrel (or test backend)
    pub async fn initialize_squirrel_connection(&self) -> ToadStoolResult<()> {
        self.backend.initialize().await
    }

    /// Deploy an AI agent from configuration
    pub async fn deploy_agent(&mut self, agent_config: &AgentConfig) -> ToadStoolResult<AgentInfo> {
        self.backend.deploy_agent(agent_config).await
    }

    /// Load a model for agent use
    pub async fn load_model(&mut self, model_config: &ModelConfig) -> ToadStoolResult<ModelInfo> {
        self.backend.load_model(model_config).await
    }

    /// Scale an agent to specified replica count
    pub async fn scale_agent(&mut self, agent_name: &str, replicas: u32) -> ToadStoolResult<()> {
        self.backend.scale_agent(agent_name, replicas).await
    }

    /// Stop an agent
    pub async fn stop_agent(&mut self, agent_name: &str) -> ToadStoolResult<()> {
        self.backend.stop_agent(agent_name).await
    }

    /// Remove an agent
    pub async fn remove_agent(&mut self, agent_name: &str) -> ToadStoolResult<()> {
        self.backend.remove_agent(agent_name).await
    }

    /// Get agent status (now properly async!)
    pub async fn get_agent_status(&self, agent_name: &str) -> ToadStoolResult<AgentStatus> {
        self.backend.get_agent_status(agent_name).await
    }

    /// List all deployed agents (now properly async!)
    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        self.backend.list_agents().await.unwrap_or_default()
    }

    /// List all loaded models (now properly async!)
    pub async fn list_models(&self) -> Vec<ModelInfo> {
        self.backend.list_models().await.unwrap_or_default()
    }

    /// Get agent resource usage (now properly async!)
    pub async fn get_agent_resources(
        &self,
        agent_name: &str,
    ) -> ToadStoolResult<AgentResourceUsage> {
        self.backend.get_agent_resources(agent_name).await
    }

    /// Unload a model
    pub async fn unload_model(&mut self, model_name: &str) -> ToadStoolResult<()> {
        self.backend.unload_model(model_name).await
    }

    /// Health check for agent manager
    pub async fn health_check(&self) -> ToadStoolResult<()> {
        self.backend.health_check().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_config() -> AgentDeploymentConfig {
        AgentDeploymentConfig {
            squirrel_endpoint: "http://localhost:8080".to_string(),
            model_registry: "local".to_string(),
            agent_runtime: "container".to_string(),
            mcp_enabled: true,
            resource_limits: serde_json::Map::new(),
        }
    }

    fn sample_agent_info() -> AgentInfo {
        let now = chrono::Utc::now();
        AgentInfo {
            name: "test-agent".to_string(),
            agent_id: "test-agent-abc123".to_string(),
            model: "test-model".to_string(),
            status: AgentStatus::Running,
            replicas: 1,
            capabilities: vec!["chat".to_string(), "reasoning".to_string()],
            resources: AgentResourceUsage {
                cpu_millicores: 500,
                memory_bytes: 1024 * 1024 * 512,
                gpu_percent: None,
                network_bytes_per_sec: 1024,
            },
            created_at: now,
            last_updated: now,
        }
    }

    #[test]
    fn test_agent_deployment_config_construction() {
        let config = test_config();
        assert_eq!(config.squirrel_endpoint, "http://localhost:8080");
        assert_eq!(config.model_registry, "local");
        assert_eq!(config.agent_runtime, "container");
        assert!(config.mcp_enabled);
        assert!(config.resource_limits.is_empty());
    }

    #[test]
    fn test_agent_deployment_config_serialization_roundtrip() {
        let config = test_config();
        let json = serde_json::to_string(&config).expect("serialize");
        let restored: AgentDeploymentConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config.squirrel_endpoint, restored.squirrel_endpoint);
    }

    #[test]
    fn test_agent_status_variants() {
        assert_eq!(AgentStatus::Deploying, AgentStatus::Deploying);
        assert_eq!(AgentStatus::Running, AgentStatus::Running);
        assert_eq!(AgentStatus::Scaling, AgentStatus::Scaling);
        assert_eq!(AgentStatus::Updating, AgentStatus::Updating);
        assert_eq!(AgentStatus::Terminating, AgentStatus::Terminating);
        assert_eq!(AgentStatus::Stopped, AgentStatus::Stopped);
        assert!(matches!(
            AgentStatus::Failed("reason".to_string()),
            AgentStatus::Failed(s) if s == "reason"
        ));
    }

    #[test]
    fn test_model_status_variants() {
        assert_eq!(ModelStatus::Loading, ModelStatus::Loading);
        assert_eq!(ModelStatus::Ready, ModelStatus::Ready);
        assert_eq!(ModelStatus::Updating, ModelStatus::Updating);
        assert_eq!(ModelStatus::Unloading, ModelStatus::Unloading);
        assert!(matches!(
            ModelStatus::Error("load failed".to_string()),
            ModelStatus::Error(s) if s == "load failed"
        ));
    }

    #[test]
    fn test_agent_info_construction() {
        let info = sample_agent_info();
        assert_eq!(info.name, "test-agent");
        assert_eq!(info.agent_id, "test-agent-abc123");
        assert_eq!(info.status, AgentStatus::Running);
        assert_eq!(info.replicas, 1);
        assert_eq!(info.capabilities.len(), 2);
        assert_eq!(info.resources.cpu_millicores, 500);
    }

    #[test]
    fn test_agent_info_serialization_roundtrip() {
        let info = sample_agent_info();
        let json = serde_json::to_string(&info).expect("serialize");
        let restored: AgentInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(info.name, restored.name);
        assert_eq!(info.status, restored.status);
    }

    #[test]
    fn test_agent_resource_usage_construction() {
        let usage = AgentResourceUsage {
            cpu_millicores: 1000,
            memory_bytes: 2 * 1024 * 1024 * 1024,
            gpu_percent: Some(50.0),
            network_bytes_per_sec: 2048,
        };
        assert_eq!(usage.cpu_millicores, 1000);
        assert_eq!(usage.memory_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(usage.gpu_percent, Some(50.0));
    }

    #[test]
    fn test_agent_resource_usage_serialization_roundtrip() {
        let usage = AgentResourceUsage {
            cpu_millicores: 500,
            memory_bytes: 1024 * 1024,
            gpu_percent: None,
            network_bytes_per_sec: 512,
        };
        let json = serde_json::to_string(&usage).expect("serialize");
        let restored: AgentResourceUsage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(usage.cpu_millicores, restored.cpu_millicores);
    }

    #[test]
    fn test_model_info_construction() {
        let now = chrono::Utc::now();
        let info = ModelInfo {
            name: "gpt-4".to_string(),
            model_id: "model-xyz".to_string(),
            model_type: "llm".to_string(),
            size_bytes: 1_000_000_000,
            status: ModelStatus::Ready,
            resource_requirements: ModelResourceRequirements {
                min_cpu_cores: 4.0,
                min_memory_gb: 8.0,
                gpu_required: true,
                min_gpu_memory_gb: Some(16.0),
            },
            performance: ModelPerformanceMetrics {
                avg_inference_time_ms: 50,
                throughput_rps: 10.0,
                success_rate: 99.5,
            },
            loaded_at: now,
        };
        assert_eq!(info.name, "gpt-4");
        assert_eq!(info.status, ModelStatus::Ready);
        assert_eq!(info.resource_requirements.min_cpu_cores, 4.0);
    }

    #[test]
    fn test_model_info_serialization_roundtrip() {
        let now = chrono::Utc::now();
        let info = ModelInfo {
            name: "model-a".to_string(),
            model_id: "id-1".to_string(),
            model_type: "type-a".to_string(),
            size_bytes: 100,
            status: ModelStatus::Loading,
            resource_requirements: ModelResourceRequirements {
                min_cpu_cores: 1.0,
                min_memory_gb: 2.0,
                gpu_required: false,
                min_gpu_memory_gb: None,
            },
            performance: ModelPerformanceMetrics {
                avg_inference_time_ms: 10,
                throughput_rps: 5.0,
                success_rate: 100.0,
            },
            loaded_at: now,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let restored: ModelInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(info.name, restored.name);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_manager_with_inmemory_backend() {
        let config = test_config();
        let mut manager = AgentDeploymentManager::with_inmemory(config);

        let agent_config = AgentConfig {
            name: "test-agent".to_string(),
            model: "test-model".to_string(),
            capabilities: vec!["chat".to_string()],
            resources: None,
            environment: HashMap::new(),
            config: HashMap::new(),
        };

        let result = manager.deploy_agent(&agent_config).await;
        assert!(result.is_ok());

        let agent_info = result.expect("Agent deployment should succeed in test");
        assert_eq!(agent_info.name, "test-agent");
        assert!(agent_info.agent_id.starts_with("test-agent-"));
        assert_eq!(agent_info.status, AgentStatus::Running);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_list_agents_returns_deployed() {
        let config = test_config();
        let mut manager = AgentDeploymentManager::with_inmemory(config);

        let agent_config = AgentConfig {
            name: "list-test-agent".to_string(),
            model: "test-model".to_string(),
            capabilities: vec!["chat".to_string()],
            resources: None,
            environment: HashMap::new(),
            config: HashMap::new(),
        };
        manager.deploy_agent(&agent_config).await.unwrap();

        let agents = manager.list_agents().await;
        assert!(!agents.is_empty());
        assert!(agents.iter().any(|a| a.name == "list-test-agent"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_get_agent_status_after_deploy() {
        let config = test_config();
        let mut manager = AgentDeploymentManager::with_inmemory(config);

        let agent_config = AgentConfig {
            name: "status-agent".to_string(),
            model: "test-model".to_string(),
            capabilities: vec!["chat".to_string()],
            resources: None,
            environment: HashMap::new(),
            config: HashMap::new(),
        };
        manager.deploy_agent(&agent_config).await.unwrap();

        let status = manager.get_agent_status("status-agent").await;
        assert!(status.is_ok());
        assert_eq!(status.unwrap(), AgentStatus::Running);
    }
}
