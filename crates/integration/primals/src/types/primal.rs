// SPDX-License-Identifier: AGPL-3.0-only
use async_trait::async_trait;
use serde::{Deserialize, Serialize};


use crate::error::PrimalResult;

/// Primal capability descriptor (replaces hardcoded enum)
///
/// Instead of enumerating primal names, we describe primals by their capabilities.
/// This follows the infant discovery principle: "Each primal knows only itself."
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrimalDescriptor {
    /// Primal identifier (self-reported, not hardcoded)
    pub id: String,
    /// Capabilities this primal provides
    pub capabilities: Vec<String>,
    /// Optional type hint (for compatibility)
    pub type_hint: Option<String>,
}

impl PrimalDescriptor {
    /// Create a new primal descriptor
    pub fn new(id: impl Into<String>, capabilities: Vec<String>) -> Self {
        Self {
            id: id.into(),
            capabilities,
            type_hint: None,
        }
    }
    
    /// Check if this primal has a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
    
    /// Get a display name for this primal
    pub fn display_name(&self) -> &str {
        &self.id
    }
}

// Re-export the canonical PrimalIntegration trait from the parent module
// This trait is defined in `crate::PrimalIntegration` (lib.rs) with the complete interface.
// Keeping this re-export for backward compatibility with any code that imports from here.
pub use crate::PrimalIntegration;

// Note: The legacy trait definition that was here has been removed to eliminate duplication.
// Please use `crate::PrimalIntegration` or import from the crate root instead.

/// Primal capabilities structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalCapabilities {
    pub core: Vec<String>,
    pub extended: Vec<String>,
    pub integrations: Vec<String>,
}
