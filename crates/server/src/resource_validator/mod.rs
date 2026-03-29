// SPDX-License-Identifier: AGPL-3.0-only
//! Resource validation for collaborative intelligence
//!
//! This module validates whether the system has sufficient resources to execute
//! a given execution graph. It compares estimated requirements against actual
//! system capabilities discovered at runtime.
//!
//! ## Deep Debt Principles
//!
//! - **Runtime Discovery**: Queries real system state, no hardcoded capabilities
//! - **Capability-Based**: Validates based on advertised capabilities
//! - **Self-Knowledge**: System reports its own capabilities
//! - **No Hardcoding**: All thresholds and limits from configuration or system query
//! - **Safe Rust**: All validation logic in safe Rust

pub(crate) mod analysis;
mod error;
pub(crate) mod system_query;
mod types;

#[cfg(test)]
mod tests;

use tracing::{info, warn};

use crate::graph_types::ExecutionGraph;
use crate::resource_estimator::ResourceEstimator;

pub use error::ValidationError;
pub use types::{AvailabilityResult, ResourceGap, SystemCapabilities};

/// Resource validator
///
/// Validates execution graphs against system capabilities.
pub struct ResourceValidator {
    estimator: ResourceEstimator,
}

impl Default for ResourceValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceValidator {
    /// Create a new resource validator
    pub fn new() -> Self {
        Self {
            estimator: ResourceEstimator::new(),
        }
    }

    /// Validate whether the system can execute the graph
    ///
    /// This performs:
    /// 1. Resource estimation
    /// 2. System capability query
    /// 3. Comparison and gap analysis
    /// 4. Warning generation
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError`] if estimation fails or system capability query fails.
    pub async fn validate_availability(
        &self,
        graph: &ExecutionGraph,
    ) -> Result<AvailabilityResult, ValidationError> {
        info!("Validating resource availability for graph: {}", graph.id);

        // Estimate requirements
        let estimate = self
            .estimator
            .estimate(graph)
            .map_err(ValidationError::EstimationFailed)?;

        // Query system capabilities
        let capabilities = system_query::query_system_capabilities().await?;

        // Compare and identify gaps
        let gaps = analysis::identify_gaps(&estimate, &capabilities);

        // Generate warnings
        let warnings = analysis::generate_warnings(&estimate, &capabilities);

        // Determine if execution is possible
        let available = gaps.is_empty();

        if available {
            info!("✅ System has sufficient resources for graph {}", graph.id);
        } else {
            warn!(
                "❌ System lacks resources for graph {}. Gaps: {:?}",
                graph.id, gaps
            );
        }

        Ok(AvailabilityResult {
            graph_id: graph.id.clone(),
            available,
            gaps,
            warnings,
            system_capabilities: capabilities,
            estimated_requirements: estimate,
        })
    }
}
