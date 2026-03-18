// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service Registry - Dynamic service discovery
//!
//! Eliminates hardcoded primal/service names, enabling:
//! - Dynamic service discovery
//! - Environment-specific service configuration
//! - Multi-instance deployments
//! - Flexible ecosystem integration

mod registry;
mod types;

#[cfg(test)]
mod tests;

// Re-exports for backward compatibility
pub use registry::ServiceRegistry;
pub use types::{ServiceEndpoint, ServiceError, ServiceResult, ServiceType};
