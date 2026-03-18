// SPDX-License-Identifier: AGPL-3.0-or-later
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

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::return_self_not_must_use,
    clippy::redundant_closure_for_method_calls,
    clippy::map_unwrap_or,
    clippy::struct_excessive_bools,
    clippy::unnecessary_wraps,
    clippy::semicolon_if_nothing_returned,
    clippy::needless_pass_by_value,
    clippy::wildcard_imports,
    clippy::similar_names,
    clippy::if_not_else,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::unnested_or_patterns,
    clippy::items_after_statements,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::unused_self,
    clippy::no_effect_underscore_binding,
    clippy::used_underscore_binding,
    clippy::unreadable_literal,
    clippy::default_trait_access,
    clippy::single_match_else,
    clippy::manual_let_else,
    clippy::format_push_string,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::unused_async
)]

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
pub mod cloud_provider_trait;
pub mod composition_constraints;
pub mod composition_engine;
pub mod cross_spring_provenance;
pub mod deployment_layer;
pub mod discovery;
pub mod ecosystem;
pub mod encryption;
pub mod error;
pub mod execution;
pub mod fractal_integration;
/// Universal IPC (evolved from ipc_helpers).
pub mod ipc;
/// Legacy IPC helpers - prefer `toadstool::ipc` for new code.
pub mod ipc_helpers;
pub mod launcher; // Phase 3: Deployment coordination
pub mod layer_adaptation;
pub mod multi_workload_compositor;
pub mod os_layer;
pub mod performance_hardening;
pub mod plugin_system;
pub mod production_hardening;
pub mod resources;
pub mod runtime;
pub mod runtime_discovery;
pub mod security;
pub mod security_hardening;
pub mod self_identity;
pub mod semantic_methods;
pub mod universal;
pub mod workload;
pub mod workload_migration;
// biomeOS integration is now handled as a primal through the ecosystem module
// Re-export core types
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
pub use error::{
    ConfigError, ConfigResult, ExecutionError, ExecutionResult, IntegrationError,
    IntegrationResult, NetworkError, NetworkResult, ResourceError, ResourceResult, SecurityError,
    SecurityResult, SystemError, SystemResult,
};
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
    ResourceMonitor, ResourceRequirements, ResourceUsage, RuntimeMetrics, StorageRequirements,
    SystemResourceMonitor, SystemResources,
};
pub use runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy};
pub use security::{
    AuditEvent, AuditSettings, Capability, FilesystemSecurity, IsolationLevel, NetworkSecurity,
    SecurityContext, SecurityPolicy as ToadStoolSecurityPolicy, SecuritySettings,
};
pub use universal::{
    JobPriority, PlatformStatus, UniversalComputePlatform, UniversalJob, UniversalJobType,
    UniversalPlatformConfig, UniversalScheduler, UniversalSystemResources,
    init_with_runtime_engines,
};
pub use workload::{
    ExecutableSource, GpuArgument, GpuProgramSource, PortMapping as WorkloadPortMapping,
    PortProtocol, PythonSource, RegistryAuth, VolumeMount, VolumeMountType, WasiConfig,
    WasmModuleSource, WorkloadSpec, WorkloadType,
};
// biomeOS integration types are now available through the ecosystem primal interface
// No longer need hard integration re-exports - biomeOS interacts as a primal

// Re-export common utilities (targeted, not wildcard — keeps downstream recompilation minimal)
pub use toadstool_common as common;
pub use toadstool_common::{
    ToadStoolError, ToadStoolErrorExt, ToadStoolErrorWithCode, ToadStoolResult,
    ToadStoolResultWithCode,
    auth::{AuthCredentials, AuthType, ServiceAuthConfig},
    error_codes::{self, ErrorCategory, ErrorCode, codes},
};
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
pub fn init() -> ToadStoolResult<()> {
    // Initialize tracing for universal compute
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .map_err(|e| ToadStoolError::runtime(format!("Failed to initialize tracing: {e}")))?;

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
pub async fn init_with_ecosystem() -> ToadStoolResult<UniversalComputePlatform> {
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
pub async fn init_with_biomeos() -> ToadStoolResult<UniversalComputePlatform> {
    init()?;

    tracing::info!("🌱 Initializing with biomeOS as primal...");

    // biomeOS is now handled as a primal through the ecosystem
    let platform = init_with_ecosystem().await?;

    tracing::info!("✅ ToadStool ready with biomeOS primal integration");

    Ok(platform)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constant() {
        assert!(!VERSION.is_empty());
        assert!(
            VERSION
                .chars()
                .next()
                .expect("VERSION should have at least one char")
                .is_ascii_digit()
        );
    }

    #[test]
    fn test_init_succeeds_or_tracing_already_initialized() {
        // init() sets up tracing; may fail if another test already initialized it
        let result = init();
        if let Err(e) = result {
            assert!(
                e.to_string().to_lowercase().contains("tracing"),
                "Unexpected init error: {e}"
            );
        }
    }

    #[test]
    fn test_universal_capabilities() {
        assert!(!UNIVERSAL_CAPABILITIES.is_empty());
        assert!(UNIVERSAL_CAPABILITIES.contains(&"native_execution"));
        assert!(UNIVERSAL_CAPABILITIES.contains(&"wasm_execution"));
        assert!(UNIVERSAL_CAPABILITIES.contains(&"pure_ecosystem"));
    }
}
