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

//! # `ToadStool` Universal Compute Platform
//!
//! `ToadStool` is a revolutionary universal compute platform that embodies the philosophy:
//! **"If it computes, we can run it"**
//!
//! ## Core Principles
//!
//! - **Universal Execution**: Run anything, anywhere, on any substrate
//! - **Pure Ecosystem**: No external dependencies, pure Rust ecosystem
//! - **Recursive Hosting**: `ToadStools` can host other `ToadStools` infinitely
//! - **OS-Layer Compatibility**: Act as an OS when local environments aren't compatible
//! - **Ecosystem Integration**: Seamless integration with biomeOS and all primals
//!
//! ## Architecture
//!
//! ```text
//! biomeOS (Universal OS)
//!     ↓
//! ToadStool (Universal Compute)
//!     ├── Native Runtime (Process execution)
//!     ├── WASM Runtime (WebAssembly execution)
//!     ├── Universal Scheduler (Any substrate)
//!     └── Ecosystem Integration (All primals)
//! ```

pub mod biomeos_integration;
pub mod byob;
pub mod deployment_layer;
pub mod discovery;
pub mod ecosystem;
pub mod encryption;
pub mod error;
pub mod execution;
pub mod fractal_integration;
pub mod layer_adaptation;
pub mod os_layer;
pub mod performance_hardening;
pub mod production_hardening;
pub mod resources;
pub mod runtime;
pub mod runtime_discovery;
pub mod security;
pub mod security_hardening;
pub mod self_identity;
pub mod universal;
pub mod workload;
// biomeOS integration is now handled as a primal through the ecosystem module
// No longer need hard integration - biomeOS interacts as a primal like Songbird

// Re-export core types
// BiomeOS integration is now handled as a primal - only export specific types if needed
// pub use biomeos_integration::*; // Commented out to avoid ambiguous glob with toadstool_common::auth
pub use ecosystem::{
    DiscoveryMethodConfig, EcosystemConfig, EcosystemCoordinator,
    EcosystemMessage as EcosystemCoreMessage, EcosystemMessageType as EcosystemCoreMessageType,
    ServiceChannel, ServiceClient, ServiceInstance, ServiceStatus,
};
pub use encryption::{
    CryptoCapability, CryptoProvider, CryptoProviderRegistry, EncryptedInput, EncryptedOutput,
    EncryptedPayload, EncryptionConfig, EncryptionContext, EncryptionContextBuilder, EncryptionKey,
    EncryptionMetadata, SecurityLevel as EncryptionSecurityLevel,
};
pub use error::*;
pub use execution::{
    CallbackConfig, CallbackEvent, ExecutionInput, ExecutionOutput, ExecutionRequest,
    ExecutionResponse, ExecutionStatus, LoggingConfig, RuntimeCapabilities,
    RuntimeConfig as ExecutionRuntimeConfig, RuntimeEngine, RuntimeType,
};
pub use os_layer::{
    LegacyCompatibilityLayer, LinuxCompatibilityLayer, MacOSCompatibilityLayer,
    ManagerCompatibilityLayer as CompatibilityLayer, OSLayerConfig, OSLayerManager,
    PlatformInfo as OSPlatformInfo, WindowsCompatibilityLayer,
};
pub use resources::ResourceRequirements as UniversalResourceRequirements;
pub use resources::{
    CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements, ResourceLimits,
    ResourceMonitor, ResourceRequirements, RuntimeMetrics, StorageRequirements,
    SystemResourceMonitor, SystemResources,
};
pub use runtime::*;
pub use security::{
    AuditEvent, AuditSettings, Capability, FilesystemSecurity, IsolationLevel, NetworkSecurity,
    SecurityContext, SecurityPolicy as ToadStoolSecurityPolicy, SecuritySettings,
};
pub use universal::{
    init_with_runtime_engines, JobPriority, PlatformStatus, UniversalComputePlatform, UniversalJob,
    UniversalJobType, UniversalPlatformConfig, UniversalScheduler, UniversalSystemResources,
};
pub use workload::{
    ExecutableSource, GpuArgument, GpuProgramSource, PortMapping as WorkloadPortMapping,
    PortProtocol, PythonSource, RegistryAuth, VolumeMount, VolumeMountType, WasiConfig,
    WasmModuleSource, WorkloadSpec, WorkloadType,
};
// biomeOS integration types are now available through the ecosystem primal interface
// No longer need hard integration re-exports - biomeOS interacts as a primal

// Re-export common utilities
pub use toadstool_common::*;
pub use toadstool_config as config;

/// `ToadStool` version information
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

/// Initialize `ToadStool` Universal Compute Platform
///
/// # Errors
///
/// Returns an error if:
/// - Tracing subscriber initialization fails
/// - System resources cannot be accessed
pub fn init() -> anyhow::Result<()> {
    // Initialize tracing for universal compute
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {e}"))?;

    tracing::info!(
        "🍄 ToadStool Universal Compute Platform v{} initialized",
        VERSION
    );
    tracing::info!("🌟 Capabilities: {:?}", UNIVERSAL_CAPABILITIES);
    tracing::info!("🚀 Ready for universal compute execution");

    Ok(())
}

/// Initialize `ToadStool` with ecosystem integration
///
/// # Errors
///
/// Returns an error if:
/// - Platform initialization fails
/// - Ecosystem discovery fails
/// - Primal integration fails
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

/// Initialize `ToadStool` with biomeOS as a primal (same as ecosystem init)
pub async fn init_with_biomeos() -> anyhow::Result<UniversalComputePlatform> {
    init()?;

    tracing::info!("🌱 Initializing with biomeOS as primal...");

    // biomeOS is now handled as a primal through the ecosystem
    let platform = init_with_ecosystem().await?;

    tracing::info!("✅ ToadStool ready with biomeOS primal integration");

    Ok(platform)
}
