// SPDX-License-Identifier: AGPL-3.0-only
//! Service mesh configuration extension
//!
//! Provides service mesh-specific configuration and validation.

use toadstool::error::ToadStoolResult;
use tracing::{debug, info};

/// Service mesh extension trait
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
pub(crate) trait ServiceMeshExt {
    /// Apply service mesh configuration
    async fn apply_service_mesh_config(&self) -> ToadStoolResult<()>;

    /// Validate service mesh configuration
    fn validate_service_mesh_config(&self) -> ToadStoolResult<()>;
}

impl ServiceMeshExt for super::SongbirdNetworkConfigurator {
    async fn apply_service_mesh_config(&self) -> ToadStoolResult<()> {
        info!("🕸️ Applying service mesh configuration");

        let config = &self.config.service_mesh;
        debug!("Mesh type: {}", config.mesh_type);

        // Configuration details...
        debug!("Service mesh configuration applied");

        Ok(())
    }

    fn validate_service_mesh_config(&self) -> ToadStoolResult<()> {
        let config = &self.config.service_mesh;

        if config.enabled {
            // Validate mesh type
            match config.mesh_type.as_str() {
                "istio" | "linkerd" | "consul" | "native" => {}
                _ => {
                    return Err(toadstool::error::ToadStoolError::configuration(format!(
                        "Invalid mesh type: {}",
                        config.mesh_type
                    )));
                }
            }

            // Validate sidecar configuration
            if config.sidecar.enabled && config.sidecar.proxy.listen_port == 0 {
                return Err(toadstool::error::ToadStoolError::configuration(
                    "Sidecar listen port cannot be 0".to_string(),
                ));
            }
        }

        Ok(())
    }
}
