// SPDX-License-Identifier: AGPL-3.0-only
/// Authentication and credential management for distributed services.
pub mod auth;
/// Service registry for discovery and health tracking.
pub mod registry;

pub use auth::*;
pub use registry::*;
