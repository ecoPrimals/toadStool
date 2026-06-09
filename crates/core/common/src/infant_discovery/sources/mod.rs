// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! Endpoint sources — different ways to discover service endpoints.
//!
//! Sources are tried in order until one succeeds. This enables graceful
//! fallback from production service discovery to development defaults.
//!
//! Migrated from `async_trait` to native async for zero-cost abstraction.

mod chains;
mod config_file;
mod environment;
mod fallback;
mod mdns;
mod service_mesh;

pub use chains::{development_sources, production_sources};
pub use config_file::ConfigFileSource;
pub use environment::EnvironmentSource;
pub use fallback::FallbackSource;
pub use mdns::MDNSSource;
pub use service_mesh::ServiceMeshSource;
