// SPDX-License-Identifier: AGPL-3.0-or-later
//! Standard capability identifiers for discovery.
//!
//! Use these constants with [`super::discover_service_by_capability`] or
//! [`super::discover_service_socket_by_capability`] instead of hardcoding primal names.
//! Primals only have self-knowledge and discover other primals at runtime.

/// Security/crypto capability identifier
pub const SECURITY: &str = "security";
/// Storage capability identifier
pub const STORAGE: &str = "storage";
/// Orchestration/coordination capability identifier
pub const ORCHESTRATION: &str = "orchestration";
/// AI/intelligence capability identifier
pub const AI: &str = "ai";
/// Compute capability identifier
pub const COMPUTE: &str = "compute";
