//! ToadStool Primal Provider implementation

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, info};

use toadstool_config::env_config::EnvironmentConfig;

use crate::ToadStoolResult;

use super::requests::{PrimalEndpoints, PrimalRequest, PrimalResponse, ResponseStatus};
use super::traits::UniversalPrimalProvider;
use super::types::{PrimalCapability, PrimalContext, PrimalHealth, PrimalType};

/// `ToadStool` primal provider implementation
pub struct ToadStoolPrimalProvider {
    /// Context
    context: PrimalContext,
    /// Health status
    health_status: Arc<RwLock<PrimalHealth>>,
}

impl ToadStoolPrimalProvider {
    /// Create new `ToadStool` primal provider
    #[must_use]
    pub fn new(context: PrimalContext) -> Self {
        Self {
            context,
            health_status: Arc::new(RwLock::new(PrimalHealth::Healthy)),
        }
    }
}

#[async_trait]
impl UniversalPrimalProvider for ToadStoolPrimalProvider {
    fn primal_id(&self) -> &'static str {
        "toadstool"
    }

    fn instance_id(&self) -> &'static str {
        "toadstool-main"
    }

    fn context(&self) -> &PrimalContext {
        &self.context
    }

    fn primal_type(&self) -> PrimalType {
        PrimalType::Compute
    }

    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![
            PrimalCapability::NativeExecution {
                architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
            },
            PrimalCapability::ContainerRuntime {
                orchestrators: vec!["docker".to_string(), "podman".to_string()],
            },
            PrimalCapability::WasmExecution { wasi_support: true },
            PrimalCapability::ServerlessExecution {
                languages: vec![
                    "rust".to_string(),
                    "python".to_string(),
                    "javascript".to_string(),
                ],
            },
            PrimalCapability::LoadBalancing {
                algorithms: vec!["round_robin".to_string(), "least_connections".to_string()],
            },
            PrimalCapability::AutoScaling {
                metrics: vec![
                    "cpu".to_string(),
                    "memory".to_string(),
                    "requests".to_string(),
                ],
            },
        ]
    }

    async fn health_check(&self) -> PrimalHealth {
        self.health_status.read().await.clone()
    }

    fn endpoints(&self) -> PrimalEndpoints {
        let config = EnvironmentConfig::from_env();
        let host = &config.network.bind_address;
        let port = config.network.toadstool_port;

        PrimalEndpoints {
            primary: format!("http://{host}:{port}"),
            health: format!("http://{host}:{port}/health"),
            metrics: Some(format!("http://{host}:{port}/metrics")),
            admin: Some(format!("http://{host}:{port}/admin")),
            websocket: Some(format!("ws://{host}:{port}/ws")),
            custom: HashMap::new(),
        }
    }

    async fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> ToadStoolResult<PrimalResponse> {
        debug!("Handling primal request: {:?}", request.request_type);

        Ok(PrimalResponse {
            request_id: request.id,
            status: ResponseStatus::Success,
            payload: serde_json::json!({
                "message": "Request processed successfully",
                "request_type": request.request_type
            }),
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        })
    }

    async fn initialize(&mut self, _config: serde_json::Value) -> ToadStoolResult<()> {
        info!("ToadStool primal provider initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("ToadStool primal provider shutting down");
        Ok(())
    }

    fn can_serve_context(&self, context: &PrimalContext) -> bool {
        // ToadStool can serve any context with appropriate security level
        context.security_level <= self.context.security_level
    }
}
