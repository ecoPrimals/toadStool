// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security context and policies for `ToadStool` workloads

mod context;
mod policy;
mod provider;
mod types;

// Re-export all public types for backward compatibility
pub use context::SecurityContext;
pub use policy::{AuditEvent, AuditSettings, SecurityPolicy, SecuritySettings};
pub use provider::SecurityProvider;
pub use types::{Capability, FilesystemSecurity, IsolationLevel, NetworkSecurity, UserContext};

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
