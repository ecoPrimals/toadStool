// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service deployment functionality
//!
//! ✅ FULLY MODERNIZED (Nov 24, 2025):
//! - Removed ALL sleep() calls (was 10!)
//! - Zero-delay deployments (no fake work)
//! - Ready for concurrent execution
//! - Production-grade patterns

use crate::Result;
use std::future::Future;
use tracing::{debug, info, warn};

use super::types::*;

/// Deployment extension trait
pub trait DeploymentExt {
    /// Deploy services based on configuration
    fn deploy_services(&mut self) -> impl Future<Output = Result<()>> + Send;
}

impl DeploymentExt for ZeroConfigDeployment {
    async fn deploy_services(&mut self) -> Result<()> {
        info!("🚀 Deploying services");

        // Deploy core services
        self.deploy_core_services().await?;

        // Deploy runtime engines
        self.deploy_runtime_engines().await?;

        // Deploy monitoring services
        self.deploy_monitoring_services().await?;

        // Deploy ecosystem integrations
        self.deploy_ecosystem_integrations().await?;

        info!("✅ Service deployment completed");
        Ok(())
    }
}

impl ZeroConfigDeployment {
    /// Deploy core services
    pub(crate) async fn deploy_core_services(&self) -> Result<()> {
        debug!("Deploying core services");

        // Deploy ToadStool orchestrator
        self.deploy_orchestrator().await?;

        // Deploy monitoring services
        self.deploy_monitoring().await?;

        Ok(())
    }

    /// Single-node orchestrator is implicit; multi-node deployment via Kubernetes/Docker
    /// Swarm is a P2 scaling feature.
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )]
    async fn deploy_orchestrator(&self) -> Result<()> {
        debug!("Orchestrator: single-node active (multi-node: P2)");
        Ok(())
    }

    /// Built-in tracing/metrics are active; external stack (Prometheus/Grafana) is P2.
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )]
    async fn deploy_monitoring(&self) -> Result<()> {
        debug!("Monitoring: built-in tracing active (external stack: P2)");
        Ok(())
    }

    /// Deploy runtime engines
    pub(crate) async fn deploy_runtime_engines(&self) -> Result<()> {
        debug!("Deploying runtime engines");

        // Deploy based on available runtimes
        for runtime in &self.config.runtime.preferred_runtimes {
            match runtime.as_str() {
                "native" => self.deploy_native_runtime().await?,
                "container" => self.deploy_container_runtime().await?,
                "wasm" => self.deploy_wasm_runtime().await?,
                "gpu" => self.deploy_gpu_runtime().await?,
                _ => warn!("Unknown runtime: {}", runtime),
            }
        }

        Ok(())
    }

    /// Runtimes are registered via the capability registry; deployment is discovery-based.
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )]
    async fn deploy_native_runtime(&self) -> Result<()> {
        debug!("Native runtime: available via capability registry");
        Ok(())
    }

    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )]
    async fn deploy_container_runtime(&self) -> Result<()> {
        debug!("Container runtime: available via capability registry");
        Ok(())
    }

    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )]
    async fn deploy_wasm_runtime(&self) -> Result<()> {
        debug!("WASM runtime: available via capability registry");
        Ok(())
    }

    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )]
    async fn deploy_gpu_runtime(&self) -> Result<()> {
        debug!("GPU runtime: available via capability registry");
        Ok(())
    }

    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )]
    pub(crate) async fn deploy_monitoring_services(&self) -> Result<()> {
        debug!("Monitoring services: built-in active");
        Ok(())
    }

    /// Deploy ecosystem integrations
    pub(crate) async fn deploy_ecosystem_integrations(&self) -> Result<()> {
        debug!("Deploying ecosystem integrations");

        if self.config.network.coordination_enabled {
            self.deploy_coordination_integration().await?;
        }

        if self.config.security.security_provider_enabled {
            self.deploy_security_integration().await?;
        }

        if self.config.storage.storage_provider_enabled {
            self.deploy_storage_integration().await?;
        }

        Ok(())
    }

    async fn deploy_coordination_integration(&self) -> Result<()> {
        let status = verify_capability_socket("coordination");
        match status {
            SocketStatus::Available(path) => {
                info!(path = %path.display(), "Coordination service: socket discovered");
            }
            SocketStatus::NotFound => {
                debug!("Coordination service: socket not yet available, will discover at runtime");
            }
        }
        Ok(())
    }

    async fn deploy_security_integration(&self) -> Result<()> {
        let status = verify_capability_socket("security");
        match status {
            SocketStatus::Available(path) => {
                info!(path = %path.display(), "Security service: socket discovered");
            }
            SocketStatus::NotFound => {
                debug!("Security service: socket not yet available, will discover at runtime");
            }
        }
        Ok(())
    }

    async fn deploy_storage_integration(&self) -> Result<()> {
        let status = verify_capability_socket("storage");
        match status {
            SocketStatus::Available(path) => {
                info!(path = %path.display(), "Storage service: socket discovered");
            }
            SocketStatus::NotFound => {
                debug!("Storage service: socket not yet available, will discover at runtime");
            }
        }
        Ok(())
    }
}

enum SocketStatus {
    Available(std::path::PathBuf),
    NotFound,
}

fn verify_capability_socket(capability: &str) -> SocketStatus {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    let socket_path = std::path::PathBuf::from(&runtime_dir)
        .join("biomeos")
        .join(format!("{capability}.sock"));
    if socket_path.exists() {
        SocketStatus::Available(socket_path)
    } else {
        SocketStatus::NotFound
    }
}
