use crate::error::PrimalResult;
use crate::manifest::BiomeManifest;
use crate::types::primal::PrimalIntegration;
use std::collections::HashMap;

/// Orchestrator for managing primals
pub struct PrimalOrchestrator {
    primals: HashMap<String, Box<dyn PrimalIntegration>>,
}

impl Default for PrimalOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl PrimalOrchestrator {
    pub fn new() -> Self {
        Self {
            primals: HashMap::new(),
        }
    }

    pub async fn deploy_biome(&self, manifest: BiomeManifest) -> PrimalResult<String> {
        // Stub implementation
        Ok(format!("Deployed biome: {}", manifest.name))
    }

    pub async fn register_primal(
        &mut self,
        primal_id: String,
        primal: Box<dyn PrimalIntegration>,
    ) -> PrimalResult<()> {
        self.primals.insert(primal_id, primal);
        Ok(())
    }

    pub async fn get_primal(&self, primal_id: &str) -> Option<&dyn PrimalIntegration> {
        self.primals.get(primal_id).map(|p| p.as_ref())
    }
}
