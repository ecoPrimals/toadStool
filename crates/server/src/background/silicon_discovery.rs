// SPDX-License-Identifier: AGPL-3.0-or-later
//! Silicon capability discovery — queries the shader compiler for backend capabilities.
//!
//! Runs as a background task after the visualization client discovers a shader compilation
//! provider (coralReef). Queries `shader.compile.capabilities` and stores the result in
//! shared state for the silicon handler's routing decisions.
//!
//! This implements the Node Atomic AAR requirement: toadStool queries coralReef's compilation
//! surface to inform silicon registry routing (which backends can target which units).

use crate::visualization_client::SharedVisualizationClient;
use std::sync::Arc;
use std::time::Duration;
use std::sync::RwLock;
use tracing::{debug, info};

use toadstool_integration_primals::shader_compiler::{
    ShaderCapabilitiesQuery, ShaderCompilerCapabilities, ShaderCompilerStatus,
    SHADER_CAPABILITIES_METHOD,
};

const INITIAL_DELAY: Duration = Duration::from_secs(5);
const RETRY_INTERVAL: Duration = Duration::from_secs(30);
const REFRESH_INTERVAL: Duration = Duration::from_secs(300);

/// Shared silicon registry state — populated by background discovery, read by handlers.
#[derive(Debug)]
pub struct SiliconRegistry {
    pub capabilities: Option<ShaderCompilerCapabilities>,
    pub status: ShaderCompilerStatus,
}

impl Default for SiliconRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SiliconRegistry {
    fn new() -> Self {
        Self {
            capabilities: None,
            status: ShaderCompilerStatus::Unknown,
        }
    }
}

/// Thread-safe shared reference to the silicon registry.
pub type SharedSiliconRegistry = Arc<RwLock<SiliconRegistry>>;

/// Create a new shared silicon registry.
pub fn create_silicon_registry() -> SharedSiliconRegistry {
    Arc::new(RwLock::new(SiliconRegistry::new()))
}

/// Run the silicon capability discovery background loop.
///
/// Waits for the shader compiler to become available, then queries
/// `shader.compile.capabilities` to populate the silicon registry with
/// compiler backend information for routing decisions.
pub async fn run(
    shader_client: SharedVisualizationClient,
    registry: SharedSiliconRegistry,
) {
    info!("silicon discovery background task starting");

    tokio::time::sleep(INITIAL_DELAY).await;

    loop {
        if shader_client.is_available().await {
            match query_compiler_capabilities(&shader_client).await {
                Ok(caps) => {
                    let backend_count = caps.backends.len();
                    info!(
                        backends = ?caps.backends,
                        precision_modes = caps.precision_modes.len(),
                        gemm_tiling = caps.gemm_tiling,
                        "silicon registry: compiler capabilities discovered"
                    );

                    {
                        let mut reg = registry.write().unwrap_or_else(|e| e.into_inner());
                        reg.status = ShaderCompilerStatus::Available(
                            caps.compiler_version
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string()),
                        );
                        reg.capabilities = Some(caps);
                    }

                    debug!(
                        backend_count,
                        "silicon registry updated — sleeping until refresh"
                    );
                    tokio::time::sleep(REFRESH_INTERVAL).await;
                }
                Err(e) => {
                    debug!(error = %e, "shader.compile.capabilities query failed — will retry");
                    {
                        let mut reg = registry.write().unwrap_or_else(|e| e.into_inner());
                        reg.status = ShaderCompilerStatus::Error(e.clone());
                    }

                    tokio::time::sleep(RETRY_INTERVAL).await;
                }
            }
        } else {
            {
                let mut reg = registry.write().unwrap_or_else(|e| e.into_inner());
                if reg.status != ShaderCompilerStatus::Unavailable {
                    debug!("shader compiler not yet available — silicon registry pending");
                    reg.status = ShaderCompilerStatus::Unavailable;
                }
            }

            tokio::time::sleep(RETRY_INTERVAL).await;
        }
    }
}

/// Query the shader compiler for its capabilities via IPC.
async fn query_compiler_capabilities(
    shader_client: &SharedVisualizationClient,
) -> Result<ShaderCompilerCapabilities, String> {
    let guard = shader_client
        .client_ref()
        .await
        .ok_or_else(|| "shader compiler client unavailable".to_string())?;

    let client = guard
        .get()
        .ok_or_else(|| "shader compiler client guard empty".to_string())?;

    let query = ShaderCapabilitiesQuery::default();
    let params = serde_json::to_value(&query)
        .map_err(|e| format!("failed to serialize query: {e}"))?;

    let response = client
        .call(SHADER_CAPABILITIES_METHOD, params)
        .await
        .map_err(|e| format!("IPC call failed: {e}"))?;

    serde_json::from_value::<ShaderCompilerCapabilities>(response)
        .map_err(|e| format!("failed to deserialize response: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_default_state() {
        let reg = SiliconRegistry::new();
        assert!(reg.capabilities.is_none());
        assert_eq!(reg.status, ShaderCompilerStatus::Unknown);
    }

    #[test]
    fn test_create_shared_registry() {
        let registry = create_silicon_registry();
        assert!(Arc::strong_count(&registry) == 1);
    }

    #[tokio::test]
    async fn test_registry_write_read() {
        let registry = create_silicon_registry();
        {
            let mut reg = registry.write().unwrap_or_else(|e| e.into_inner());
            reg.capabilities = Some(ShaderCompilerCapabilities {
                backends: vec!["ptx".to_string(), "spirv".to_string()],
                precision_modes: vec![],
                integer_subgroup: true,
                gemm_tiling: true,
                max_workgroup_size: Some(1024),
                compiler_version: Some("coralReef-v1.2".to_string()),
            });
            reg.status =
                ShaderCompilerStatus::Available("coralReef-v1.2".to_string());
        }

        let reg = registry.read().unwrap_or_else(|e| e.into_inner());
        let caps = reg.capabilities.as_ref().unwrap();
        assert_eq!(caps.backends, vec!["ptx", "spirv"]);
        assert!(caps.gemm_tiling);
        assert_eq!(
            reg.status,
            ShaderCompilerStatus::Available("coralReef-v1.2".to_string())
        );
    }
}
