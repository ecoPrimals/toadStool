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

//! # ToadStool Universal Compute Platform
//!
//! ToadStool is a revolutionary universal compute platform that embodies the philosophy:
//! **"If it computes, we can run it"**
//!
//! ## Core Principles
//!
//! - **Universal Execution**: Run anything, anywhere, on any substrate
//! - **Pure Ecosystem**: No external dependencies, pure Rust ecosystem
//! - **Recursive Hosting**: ToadStools can host other ToadStools infinitely
//! - **OS-Layer Compatibility**: Act as an OS when local environments aren't compatible
//! - **Ecosystem Integration**: Seamless integration with biomeOS and all primals
//!
//! ## Architecture
//!
//! ```
//! biomeOS (Universal OS)
//!     ↓
//! ToadStool (Universal Compute)
//!     ├── Native Runtime (Process execution)
//!     ├── WASM Runtime (WebAssembly execution)
//!     ├── Universal Scheduler (Any substrate)
//!     └── Ecosystem Integration (All primals)
//! ```

pub mod error;
pub mod execution;
pub mod resources;
pub mod runtime;
pub mod security;
pub mod workload;
pub mod byob;
pub mod universal;
pub mod ecosystem;
pub mod os_layer;

// Re-export core types
pub use error::*;
pub use execution::*;
pub use resources::{
    ResourceRequirements, CpuRequirements, MemoryRequirements, StorageRequirements, 
    NetworkRequirements, GpuRequirements, RuntimeMetrics, ResourceMonitor, SystemResources,
    ResourceLimits
};
pub use runtime::*;
pub use security::*;
pub use workload::*;
pub use universal::{
    UniversalComputePlatform, UniversalPlatformConfig, UniversalJob, UniversalJobType,
    UniversalScheduler, JobPriority, PlatformStatus, init_with_runtime_engines,
    ResourceRequirements as UniversalResourceRequirements, SystemResources as UniversalSystemResources
};
pub use ecosystem::*;
pub use os_layer::{
    OSLayerManager, OSLayerConfig, CompatibilityLayer, LinuxCompatibilityLayer,
    WindowsCompatibilityLayer, MacOSCompatibilityLayer, LegacyCompatibilityLayer,
    BiomeOrchestrator, BiomeOSConfig, BiomeDeployment, BiomeDeploymentStatus,
    PlatformInfo as OSPlatformInfo
};

// Re-export common utilities
pub use toadstool_common::*;
pub use toadstool_config as config;

/// ToadStool version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Universal compute platform capabilities
pub const UNIVERSAL_CAPABILITIES: &[&str] = &[
    "native_execution",
    "wasm_execution", 
    "universal_scheduling",
    "recursive_hosting",
    "os_layer_compatibility",
    "ecosystem_integration",
    "biome_orchestration",
    "pure_ecosystem",
    "substrate_agnostic",
    "infinite_nesting",
];

/// Initialize ToadStool Universal Compute Platform
pub fn init() -> anyhow::Result<()> {
    // Initialize tracing for universal compute
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;

    tracing::info!("🍄 ToadStool Universal Compute Platform v{} initialized", VERSION);
    tracing::info!("🌟 Capabilities: {:?}", UNIVERSAL_CAPABILITIES);
    tracing::info!("🚀 Ready for universal compute execution");
    
    Ok(())
}

/// Initialize ToadStool with ecosystem integration
pub async fn init_with_ecosystem() -> anyhow::Result<UniversalComputePlatform> {
    init()?;
    
    tracing::info!("🌐 Initializing ecosystem integration...");
    
    // Create universal compute platform
    let platform = UniversalComputePlatform::new().await?;
    
    // Discover and integrate with ecosystem primals
    platform.discover_ecosystem().await?;
    
    tracing::info!("✅ ToadStool Universal Compute Platform ready with ecosystem integration");
    
    Ok(platform)
}

/// Initialize ToadStool with biomeOS integration
pub async fn init_with_biomeos() -> anyhow::Result<UniversalComputePlatform> {
    init()?;
    
    tracing::info!("🌱 Initializing biomeOS integration...");
    
    // Create universal compute platform with biomeOS
    let platform = UniversalComputePlatform::new_with_biomeos().await?;
    
    tracing::info!("✅ ToadStool ready as universal OS compute layer");
    
    Ok(platform)
}
