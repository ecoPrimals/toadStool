// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! # ToadStool Core Library
//!
//! Universal compute platform for execution environments, security sandboxing, and resource management.
//!
//! ToadStool provides a unified interface for running any workload, in any language, on any platform
//! with consistent security, monitoring, and resource management.

pub mod error;
pub mod execution;
pub mod resources;
pub mod runtime;
pub mod security;
pub mod workload;

// Re-export core types
pub use error::*;
pub use execution::*;
pub use resources::*;
pub use runtime::*;
pub use security::*;
pub use workload::*;

// Re-export common utilities
pub use toadstool_common::*;
pub use toadstool_config as config;

/// ToadStool version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize ToadStool with tracing
pub fn init() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;

    tracing::info!("ToadStool v{} initialized", VERSION);
    Ok(())
}
