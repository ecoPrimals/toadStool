use async_trait::async_trait;
use serde::{Deserialize, Serialize};


use crate::error::PrimalResult;

/// Primal type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimalType {
    ToadStool,
    Songbird,
    BearDog,
    NestGate,
    Squirrel,
}

impl PrimalType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PrimalType::ToadStool => "toadstool",
            PrimalType::Songbird => "songbird",
            PrimalType::BearDog => "beardog",
            PrimalType::NestGate => "nestgate",
            PrimalType::Squirrel => "squirrel",
        }
    }
}

/// Primal integration trait
#[async_trait]
pub trait PrimalIntegration {
    /// Initialize the primal from biome manifest configuration
    async fn initialize_from_manifest(
        &self,
        config: &crate::manifest::config::PrimalConfig,
    ) -> PrimalResult<()>;

    /// Register this primal with Songbird service mesh
    async fn register_with_songbird(&self) -> PrimalResult<()>;

    /// Validate dependencies against the biome manifest
    async fn validate_dependencies(
        &self,
        manifest: &crate::manifest::BiomeManifest,
    ) -> PrimalResult<()>;

    /// Start primal services
    async fn start_services(&self) -> PrimalResult<()>;

    /// Stop primal services gracefully
    async fn shutdown(&self) -> PrimalResult<()>;

    /// Get current health status
    async fn health_check(&self) -> PrimalResult<crate::types::health::PrimalHealthStatus>;

    /// Get primal type identifier
    fn primal_type(&self) -> PrimalType;

    /// Get primal capabilities
    fn capabilities(&self) -> Vec<String>;
}

/// Primal capabilities structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalCapabilities {
    pub core: Vec<String>,
    pub extended: Vec<String>,
    pub integrations: Vec<String>,
}
