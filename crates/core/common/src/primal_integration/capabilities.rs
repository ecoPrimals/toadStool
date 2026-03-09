// SPDX-License-Identifier: AGPL-3.0-only
//! Standard capability identifiers for discovery.
//!
//! Use these constants with [`super::discover_service_by_capability`] or
//! [`super::discover_service_socket_by_capability`] instead of hardcoding primal names.
//! Primals only have self-knowledge and discover other primals at runtime.

pub const SECURITY: &str = "security";
pub const STORAGE: &str = "storage";
pub const ORCHESTRATION: &str = "orchestration";
pub const AI: &str = "ai";
pub const COMPUTE: &str = "compute";
