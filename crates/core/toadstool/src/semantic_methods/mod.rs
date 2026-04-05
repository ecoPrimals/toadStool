// SPDX-License-Identifier: AGPL-3.0-or-later
//! Semantic Method Name Registry
//!
//! Maps semantic method names to implementation functions following
//! `wateringHole/SEMANTIC_METHOD_NAMING_STANDARD.md`
//!
//! ## Semantic Namespace Structure
//!
//! Format: `{domain}.{operation}[.{variant}]`
//!
//! - **Domain**: Capability area (compute, resource, storage, network, security)
//! - **Operation**: What the method does (execute, get, store, configure, etc.)
//! - **Variant** (optional): Specific algorithm or mode
//!
//! ## Evolution Strategy
//!
//! **Phase 1** (Current): Backward-compatible aliases
//! - Both old and new names work
//! - New code uses semantic names
//! - Zero breaking changes
//!
//! **Phase 2** (Future): Deprecation warnings
//! - Log warnings for old names
//! - Encourage migration
//!
//! **Phase 3** (Future): Remove old names
//! - Clean semantic-only API
//!
//! ## Example
//!
//! ```rust
//! use toadstool::semantic_methods::SemanticMethodRegistry;
//!
//! let registry = SemanticMethodRegistry::new();
//!
//! // Resolve semantic name to implementation
//! assert_eq!(
//!     registry.resolve("compute.execute"),
//!     Some("execute_workload")
//! );
//!
//! // Check if method is semantic
//! assert!(registry.is_semantic("compute.execute"));
//! assert!(!registry.is_semantic("execute_workload"));
//! ```

mod mappings_core;
mod mappings_extended;

use std::collections::HashMap;

/// Semantic method registry
///
/// Maps semantic method names (e.g., `compute.execute`) to implementation
/// method names (e.g., `execute_workload`) for backward compatibility.
#[derive(Debug, Clone)]
pub struct SemanticMethodRegistry {
    /// Method aliases: `semantic_name` → `implementation_name`
    aliases: HashMap<String, String>,

    /// Reverse mapping: `implementation_name` → `semantic_name`
    reverse: HashMap<String, String>,
}

impl SemanticMethodRegistry {
    /// Create new registry with default mappings
    ///
    /// Initializes all standard ToadStool method mappings following
    /// the wateringHole semantic naming standard.
    pub fn new() -> Self {
        let mut aliases = HashMap::new();
        let mut reverse = HashMap::new();

        // Helper to add bidirectional mapping
        let mut add_mapping = |semantic: &str, implementation: &str| {
            aliases.insert(semantic.to_string(), implementation.to_string());
            reverse.insert(implementation.to_string(), semantic.to_string());
        };

        mappings_core::register(&mut add_mapping);
        mappings_extended::register(&mut add_mapping);

        Self { aliases, reverse }
    }

    /// Resolve semantic name to implementation name
    ///
    /// Returns the implementation method name if the semantic name is registered,
    /// otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use toadstool::semantic_methods::SemanticMethodRegistry;
    ///
    /// let registry = SemanticMethodRegistry::new();
    /// assert_eq!(registry.resolve("compute.execute"), Some("execute_workload"));
    /// assert_eq!(registry.resolve("unknown.method"), None);
    /// ```
    pub fn resolve(&self, semantic_name: &str) -> Option<&str> {
        self.aliases.get(semantic_name).map(|s| s.as_str())
    }

    /// Get semantic name for implementation method
    ///
    /// Returns the semantic method name if the implementation name is registered,
    /// otherwise returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use toadstool::semantic_methods::SemanticMethodRegistry;
    ///
    /// let registry = SemanticMethodRegistry::new();
    /// assert_eq!(registry.get_semantic("execute_workload"), Some("compute.execute"));
    /// ```
    pub fn get_semantic(&self, implementation_name: &str) -> Option<&str> {
        self.reverse.get(implementation_name).map(|s| s.as_str())
    }

    /// Check if method name is semantic (contains '.')
    ///
    /// # Examples
    ///
    /// ```
    /// use toadstool::semantic_methods::SemanticMethodRegistry;
    ///
    /// let registry = SemanticMethodRegistry::new();
    /// assert!(registry.is_semantic("compute.execute"));
    /// assert!(registry.is_semantic("resource.cpu.get_usage"));
    /// assert!(!registry.is_semantic("execute_workload"));
    /// ```
    pub fn is_semantic(&self, method_name: &str) -> bool {
        method_name.contains('.')
    }

    /// Check if semantic name is registered
    pub fn is_registered(&self, semantic_name: &str) -> bool {
        self.aliases.contains_key(semantic_name)
    }

    /// Get all registered semantic names
    pub fn semantic_names(&self) -> Vec<&str> {
        self.aliases.keys().map(|s| s.as_str()).collect()
    }

    /// Get all registered implementation names
    pub fn implementation_names(&self) -> Vec<&str> {
        self.reverse.keys().map(|s| s.as_str()).collect()
    }

    /// Get count of registered mappings
    pub fn count(&self) -> usize {
        self.aliases.len()
    }
}

impl Default for SemanticMethodRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
