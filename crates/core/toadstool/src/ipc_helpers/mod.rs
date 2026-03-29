// SPDX-License-Identifier: AGPL-3.0-only
//! IPC helpers for primal-to-primal communication
//!
//! ## Deep Debt Principles
//!
//! - **Services, Not Libraries**: Communicate via services, not code embedding
//! - **Runtime Discovery**: Discover primals at runtime via Songbird
//! - **Self-Knowledge**: Only know ourselves, discover others
//! - **Standard Protocol**: JSON-RPC 2.0 over Unix sockets
//!
//! ## Architecture
//!
//! ```text
//! ToadStool ──[discover]──> Songbird ──[resolve]──> Other Primal
//!     │                         │
//!     └─────[register]──────────┘
//! ```

mod connection;
mod framing;

use crate::semantic_methods::SemanticMethodRegistry;
use std::sync::OnceLock;
use tracing::debug;

// Re-export connection APIs
pub use connection::{
    connect_to_primal, find_by_capability, get_default_songbird_socket, register_with_songbird,
    resolve_primal,
};

/// Global semantic method registry (initialized once)
static SEMANTIC_REGISTRY: OnceLock<SemanticMethodRegistry> = OnceLock::new();

fn get_registry() -> &'static SemanticMethodRegistry {
    SEMANTIC_REGISTRY.get_or_init(SemanticMethodRegistry::new)
}

/// Resolve method name from semantic to implementation
pub fn resolve_method_name(method: &str) -> String {
    let registry = get_registry();
    if registry.is_semantic(method) {
        registry.resolve(method).map_or_else(
            || {
                debug!("Unknown semantic method '{}', passing through", method);
                String::from(method)
            },
            |impl_name| {
                debug!("Resolved semantic method '{}' → '{}'", method, impl_name);
                String::from(impl_name)
            },
        )
    } else {
        String::from(method)
    }
}

/// Check if a method name is semantic (contains '.')
pub fn is_semantic_method(method: &str) -> bool {
    get_registry().is_semantic(method)
}

/// Get semantic name for implementation method (if registered)
pub fn get_semantic_name(implementation: &str) -> Option<String> {
    get_registry()
        .get_semantic(implementation)
        .map(String::from)
}

/// Get all registered semantic method names
pub fn list_semantic_methods() -> Vec<&'static str> {
    get_registry().semantic_names()
}

#[cfg(test)]
mod tests;
