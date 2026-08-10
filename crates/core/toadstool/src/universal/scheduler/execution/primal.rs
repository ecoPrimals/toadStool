// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primal registry routed execution (`execute_primal`).

use std::collections::HashMap;

use bytes::Bytes;
use tracing::{debug, warn};

use crate::ToadStoolResult;
use crate::execution::{ExecutionResponse, RuntimeEngine};

use super::super::UniversalScheduler;
use super::discover::discover_self_ip_address;
use crate::universal::requests::{PrimalRequest, ResponseStatus};
use crate::universal::traits::UniversalPrimalProvider;
use crate::universal::types::{NetworkLocation, PrimalContext, SecurityLevel};

impl<P, E: RuntimeEngine> UniversalScheduler<P, E>
where
    P: UniversalPrimalProvider + Send + Sync + 'static,
{
    /// Execute a primal job (remote capability)
    pub(in crate::universal::scheduler) async fn execute_primal(
        &self,
        primal_type: &str,
        endpoint: &str,
        payload: &serde_json::Value,
    ) -> ToadStoolResult<ExecutionResponse> {
        debug!(
            "Executing primal job: {} at endpoint: {}",
            primal_type, endpoint
        );
        let start_time = std::time::Instant::now();
        let execution_id = crate::generate_uuid();

        // Find a primal provider that matches the requested type
        let providers = self.primal_registry().get_all_providers().await;
        let matching_provider = providers
            .iter()
            .find(|p| p.primal_type().as_str() == primal_type);

        if let Some(provider) = matching_provider {
            // Build and route the request through the primal registry
            let request = PrimalRequest {
                id: execution_id,
                source: "scheduler".to_string(),
                target: provider.instance_id().to_string(),
                request_type: endpoint.to_string(),
                payload: payload.clone(),
                context: PrimalContext {
                    user_id: "scheduler".to_string(),
                    device_id: "local".to_string(),
                    session_id: execution_id.to_string(),
                    network_location: NetworkLocation {
                        ip_address: discover_self_ip_address(),
                        subnet: None,
                        network_id: None,
                        geo_location: None,
                    },
                    security_level: SecurityLevel::Standard,
                    metadata: HashMap::new(),
                },
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            };

            match self.primal_registry().route_request(request).await {
                Ok(response) => {
                    let duration = start_time.elapsed();
                    let status = if response.status == ResponseStatus::Success {
                        crate::execution::ExecutionStatus::Success
                    } else {
                        let error_msg = match &response.status {
                            ResponseStatus::Error { message, .. } => message.clone(),
                            ResponseStatus::Timeout => "Request timed out".to_string(),
                            ResponseStatus::ServiceUnavailable => "Service unavailable".to_string(),
                            ResponseStatus::Success => "Unknown error".to_string(),
                        };
                        crate::execution::ExecutionStatus::Failed {
                            error: error_msg.into(),
                        }
                    };

                    Ok(ExecutionResponse {
                        execution_id,
                        status,
                        output: crate::execution::ExecutionOutput {
                            data: Bytes::from(
                                serde_json::to_vec(&response.payload).unwrap_or_default(),
                            ),
                            stdout: Some(format!("Primal '{primal_type}' executed successfully")),
                            stderr: None,
                            exit_code: Some(0),
                            format: Some("application/json".to_string()),
                            result: HashMap::new(),
                            metadata: HashMap::new(),
                        },
                        metrics: crate::RuntimeMetrics::default(),
                        duration,
                        runtime_used: crate::execution::RuntimeType::Native,
                        warnings: Vec::new(),
                    })
                }
                Err(e) => {
                    let duration = start_time.elapsed();
                    let error_msg = format!("Primal '{primal_type}' execution failed: {e}");
                    warn!("{}", error_msg);
                    Ok(ExecutionResponse {
                        execution_id,
                        status: crate::execution::ExecutionStatus::Failed {
                            error: error_msg.clone().into(),
                        },
                        output: crate::execution::ExecutionOutput {
                            data: bytes::Bytes::new(),
                            stdout: None,
                            stderr: Some(error_msg),
                            exit_code: Some(1),
                            format: Some("text/plain".to_string()),
                            result: HashMap::new(),
                            metadata: HashMap::new(),
                        },
                        metrics: crate::RuntimeMetrics::default(),
                        duration,
                        runtime_used: crate::execution::RuntimeType::Native,
                        warnings: Vec::new(),
                    })
                }
            }
        } else {
            // No matching primal provider found
            let error_msg = format!(
                "No primal provider registered for type '{}'. Available providers: {}",
                primal_type,
                if providers.is_empty() {
                    "none".to_string()
                } else {
                    providers
                        .iter()
                        .map(|p| p.primal_type().as_str().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            );
            warn!("{}", error_msg);
            Ok(ExecutionResponse {
                execution_id,
                status: crate::execution::ExecutionStatus::Failed {
                    error: error_msg.clone().into(),
                },
                output: crate::execution::ExecutionOutput {
                    data: bytes::Bytes::new(),
                    stdout: None,
                    stderr: Some(error_msg),
                    exit_code: Some(127),
                    format: Some("text/plain".to_string()),
                    result: HashMap::new(),
                    metadata: HashMap::new(),
                },
                metrics: crate::RuntimeMetrics::default(),
                duration: start_time.elapsed(),
                runtime_used: crate::execution::RuntimeType::Native,
                warnings: vec![
                    "Register a primal provider via primal_registry.register_primal()".to_string(),
                ],
            })
        }
    }
}
