//! Job types and priority management

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::resources::ResourceRequirements;

use super::types::PrimalContext;

/// Job priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum JobPriority {
    /// Emergency - highest priority (level 0)
    Emergency = 0,
    /// Critical - very high priority (level 1)
    Critical = 1,
    /// High priority (level 2)
    High = 2,
    /// Normal priority (level 3)
    Normal = 3,
    /// Low priority (level 4)
    Low = 4,
    /// Background - lowest priority (level 5)
    Background = 5,
}

/// Universal job types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UniversalJobType {
    /// Native process execution
    Native {
        executable: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    /// WebAssembly execution
    Wasm {
        module: Vec<u8>,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    /// Primal delegation
    Primal {
        primal_type: String,
        endpoint: String,
        payload: serde_json::Value,
    },
    /// `BiomeOS` orchestration
    BiomeOS {
        biome_manifest: serde_json::Value,
        team_id: String,
    },
}

/// Universal job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalJob {
    /// Job ID
    pub id: Uuid,
    /// Job type
    pub job_type: UniversalJobType,
    /// Job priority
    pub priority: JobPriority,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Execution timeout
    pub timeout: Option<Duration>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Context
    pub context: PrimalContext,
}
