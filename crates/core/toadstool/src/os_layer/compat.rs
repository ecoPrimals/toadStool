use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

// Note: The manager module now re-exports this trait, so no circular dependency
use crate::{ExecutionRequest, ExecutionResponse, ToadStoolResult};

/// Compatibility layer trait for different operating systems
///
/// This is the canonical definition of the CompatibilityLayer trait.
/// All OS-specific compatibility implementations should use this trait.
///
/// Migrated from async_trait to native async for zero-cost abstraction.
pub trait CompatibilityLayer: Send + Sync {
    /// Get the name of this compatibility layer
    fn name(&self) -> &str;

    /// Get supported features
    fn features(&self) -> Vec<String>;

    /// Check if this layer can handle the given request
    fn can_handle(&self, request: &ExecutionRequest) -> bool;

    /// Execute a request with OS layer compatibility
    fn execute_with_compatibility(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>>;

    /// Initialize the compatibility layer
    fn initialize(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Shutdown the compatibility layer
    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;
}

/// Linux compatibility layer
#[derive(Debug)]
pub struct LinuxCompatibilityLayer {
    config: LinuxCompatConfig,
}

/// Windows compatibility layer
#[derive(Debug)]
pub struct WindowsCompatibilityLayer {
    config: WindowsCompatConfig,
}

/// macOS compatibility layer
#[derive(Debug)]
pub struct MacOSCompatibilityLayer {
    config: MacOSCompatConfig,
}

/// Legacy systems compatibility layer
#[derive(Debug)]
pub struct LegacyCompatibilityLayer {
    config: LegacyCompatConfig,
}

/// Configuration for Linux compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxCompatConfig {
    /// Enable namespace isolation
    pub namespace_isolation: bool,
    /// Enable cgroup resource control
    pub cgroup_control: bool,
    /// Enable seccomp filtering
    pub seccomp_filtering: bool,
    /// Enable capabilities management
    pub capabilities_management: bool,
}

impl Default for LinuxCompatConfig {
    fn default() -> Self {
        Self {
            namespace_isolation: true,
            cgroup_control: true,
            seccomp_filtering: true,
            capabilities_management: true,
        }
    }
}

/// Configuration for Windows compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsCompatConfig {
    /// Enable job object control
    pub job_object_control: bool,
    /// Enable token restriction
    pub token_restriction: bool,
    /// Enable `AppContainer` isolation
    pub app_container_isolation: bool,
    /// Enable integrity levels
    pub integrity_levels: bool,
}

impl Default for WindowsCompatConfig {
    fn default() -> Self {
        Self {
            job_object_control: true,
            token_restriction: true,
            app_container_isolation: true,
            integrity_levels: true,
        }
    }
}

/// Configuration for macOS compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacOSCompatConfig {
    /// Enable sandbox profiles
    pub sandbox_profiles: bool,
    /// Enable System Integrity Protection
    pub sip_integration: bool,
    /// Enable Transparency, Consent & Control
    pub tcc_integration: bool,
    /// Enable code signing verification
    pub code_signing: bool,
}

impl Default for MacOSCompatConfig {
    fn default() -> Self {
        Self {
            sandbox_profiles: true,
            sip_integration: true,
            tcc_integration: true,
            code_signing: true,
        }
    }
}

/// Configuration for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyCompatConfig {
    /// Target legacy system
    pub target_system: String,
    /// Emulation mode
    pub emulation_mode: String,
    /// Resource limits
    pub resource_limits: std::collections::HashMap<String, u64>,
    /// Compatibility mappings
    pub compatibility_mappings: std::collections::HashMap<String, String>,
}

impl Default for LegacyCompatConfig {
    fn default() -> Self {
        Self {
            target_system: "generic".to_string(),
            emulation_mode: "basic".to_string(),
            resource_limits: std::collections::HashMap::new(),
            compatibility_mappings: std::collections::HashMap::new(),
        }
    }
}

// Stub implementations
impl Default for LinuxCompatibilityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxCompatibilityLayer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: LinuxCompatConfig::default(),
        }
    }

    #[must_use]
    pub fn get_config(&self) -> &LinuxCompatConfig {
        &self.config
    }
}

impl Default for WindowsCompatibilityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsCompatibilityLayer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: WindowsCompatConfig::default(),
        }
    }

    #[must_use]
    pub fn get_config(&self) -> &WindowsCompatConfig {
        &self.config
    }
}

impl Default for MacOSCompatibilityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MacOSCompatibilityLayer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: MacOSCompatConfig::default(),
        }
    }

    #[must_use]
    pub fn get_config(&self) -> &MacOSCompatConfig {
        &self.config
    }
}

impl Default for LegacyCompatibilityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl LegacyCompatibilityLayer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: LegacyCompatConfig::default(),
        }
    }

    #[must_use]
    pub fn get_config(&self) -> &LegacyCompatConfig {
        &self.config
    }
}

// Stub trait implementations (migrated to native async)
impl CompatibilityLayer for LinuxCompatibilityLayer {
    fn name(&self) -> &'static str {
        "linux"
    }

    fn features(&self) -> Vec<String> {
        vec!["namespaces".to_string(), "cgroups".to_string()]
    }

    fn can_handle(&self, _request: &ExecutionRequest) -> bool {
        true
    }

    fn execute_with_compatibility(
        &self,
        _request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            // Simplified stub implementation
            Ok(ExecutionResponse::default())
        })
    }

    fn initialize(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }
}

impl CompatibilityLayer for WindowsCompatibilityLayer {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn features(&self) -> Vec<String> {
        vec!["job_objects".to_string(), "tokens".to_string()]
    }

    fn can_handle(&self, _request: &ExecutionRequest) -> bool {
        true
    }

    fn execute_with_compatibility(
        &self,
        _request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            // Simplified stub implementation
            Ok(ExecutionResponse::default())
        })
    }

    fn initialize(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }
}

impl CompatibilityLayer for MacOSCompatibilityLayer {
    fn name(&self) -> &'static str {
        "macos"
    }

    fn features(&self) -> Vec<String> {
        vec!["sandbox_profiles".to_string(), "sip".to_string()]
    }

    fn can_handle(&self, _request: &ExecutionRequest) -> bool {
        true
    }

    fn execute_with_compatibility(
        &self,
        _request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            // Simplified stub implementation
            Ok(ExecutionResponse::default())
        })
    }

    fn initialize(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }
}

impl CompatibilityLayer for LegacyCompatibilityLayer {
    fn name(&self) -> &'static str {
        "legacy"
    }

    fn features(&self) -> Vec<String> {
        vec!["emulation".to_string(), "compatibility".to_string()]
    }

    fn can_handle(&self, _request: &ExecutionRequest) -> bool {
        true
    }

    fn execute_with_compatibility(
        &self,
        _request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            // Simplified stub implementation
            Ok(ExecutionResponse::default())
        })
    }

    fn initialize(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }
}

// Note: The old ManagerCompatibilityLayer trait implementations have been removed.
// All OS-specific layers now implement the full CompatibilityLayer trait (5 methods)
// defined above, which includes can_handle() and execute_with_compatibility() along
// with name(), features(), initialize(), and shutdown().

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_config_default() {
        let config = LinuxCompatConfig::default();

        assert!(config.namespace_isolation);
        assert!(config.cgroup_control);
        assert!(config.seccomp_filtering);
        assert!(config.capabilities_management);
    }

    #[test]
    fn test_windows_config_default() {
        let config = WindowsCompatConfig::default();

        assert!(config.job_object_control);
        assert!(config.token_restriction);
        assert!(config.app_container_isolation);
        assert!(config.integrity_levels);
    }

    #[test]
    fn test_macos_config_default() {
        let config = MacOSCompatConfig::default();

        assert!(config.sandbox_profiles);
        assert!(config.sip_integration);
        assert!(config.tcc_integration);
        assert!(config.code_signing);
    }

    #[test]
    fn test_legacy_config_default() {
        let config = LegacyCompatConfig::default();

        assert_eq!(config.target_system, "generic");
        assert_eq!(config.emulation_mode, "basic");
        assert!(config.resource_limits.is_empty());
        assert!(config.compatibility_mappings.is_empty());
    }

    #[test]
    fn test_linux_layer_creation() {
        let layer = LinuxCompatibilityLayer::new();
        let config = layer.get_config();

        assert!(config.namespace_isolation);
    }

    #[test]
    fn test_windows_layer_creation() {
        let layer = WindowsCompatibilityLayer::new();
        let config = layer.get_config();

        assert!(config.job_object_control);
    }

    #[test]
    fn test_macos_layer_creation() {
        let layer = MacOSCompatibilityLayer::new();
        let config = layer.get_config();

        assert!(config.sandbox_profiles);
    }

    #[test]
    fn test_legacy_layer_creation() {
        let layer = LegacyCompatibilityLayer::new();
        let config = layer.get_config();

        assert_eq!(config.target_system, "generic");
    }

    #[test]
    fn test_linux_layer_default() {
        let layer = LinuxCompatibilityLayer::default();
        assert_eq!(layer.name(), "linux");
    }

    #[test]
    fn test_windows_layer_default() {
        let layer = WindowsCompatibilityLayer::default();
        assert_eq!(layer.name(), "windows");
    }

    #[test]
    fn test_macos_layer_default() {
        let layer = MacOSCompatibilityLayer::default();
        assert_eq!(layer.name(), "macos");
    }

    #[test]
    fn test_legacy_layer_default() {
        let layer = LegacyCompatibilityLayer::default();
        assert_eq!(layer.name(), "legacy");
    }

    #[test]
    fn test_linux_layer_features() {
        let layer = LinuxCompatibilityLayer::new();
        let features = layer.features();

        assert!(features.contains(&"namespaces".to_string()));
        assert!(features.contains(&"cgroups".to_string()));
    }

    #[test]
    fn test_windows_layer_features() {
        let layer = WindowsCompatibilityLayer::new();
        let features = layer.features();

        assert!(features.contains(&"job_objects".to_string()));
        assert!(features.contains(&"tokens".to_string()));
    }

    #[test]
    fn test_macos_layer_features() {
        let layer = MacOSCompatibilityLayer::new();
        let features = layer.features();

        assert!(features.contains(&"sandbox_profiles".to_string()));
        assert!(features.contains(&"sip".to_string()));
    }

    #[test]
    fn test_legacy_layer_features() {
        let layer = LegacyCompatibilityLayer::new();
        let features = layer.features();

        assert!(features.contains(&"emulation".to_string()));
        assert!(features.contains(&"compatibility".to_string()));
    }

    #[test]
    fn test_linux_can_handle() {
        let layer = LinuxCompatibilityLayer::new();
        let request = ExecutionRequest::default();

        assert!(CompatibilityLayer::can_handle(&layer, &request));
    }

    #[test]
    fn test_windows_can_handle() {
        let layer = WindowsCompatibilityLayer::new();
        let request = ExecutionRequest::default();

        assert!(CompatibilityLayer::can_handle(&layer, &request));
    }

    #[test]
    fn test_macos_can_handle() {
        let layer = MacOSCompatibilityLayer::new();
        let request = ExecutionRequest::default();

        assert!(CompatibilityLayer::can_handle(&layer, &request));
    }

    #[test]
    fn test_legacy_can_handle() {
        let layer = LegacyCompatibilityLayer::new();
        let request = ExecutionRequest::default();

        assert!(CompatibilityLayer::can_handle(&layer, &request));
    }

    #[test]
    fn test_config_serialization_linux() {
        let config = LinuxCompatConfig::default();
        let json = serde_json::to_string(&config).expect("Failed to serialize");
        let deserialized: LinuxCompatConfig =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.namespace_isolation, config.namespace_isolation);
    }

    #[test]
    fn test_config_serialization_windows() {
        let config = WindowsCompatConfig::default();
        let json = serde_json::to_string(&config).expect("Failed to serialize");
        let deserialized: WindowsCompatConfig =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.job_object_control, config.job_object_control);
    }

    #[test]
    fn test_config_serialization_macos() {
        let config = MacOSCompatConfig::default();
        let json = serde_json::to_string(&config).expect("Failed to serialize");
        let deserialized: MacOSCompatConfig =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.sandbox_profiles, config.sandbox_profiles);
    }

    #[test]
    fn test_config_serialization_legacy() {
        let config = LegacyCompatConfig::default();
        let json = serde_json::to_string(&config).expect("Failed to serialize");
        let deserialized: LegacyCompatConfig =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.target_system, config.target_system);
    }

    #[tokio::test]
    async fn test_linux_initialize() {
        let mut layer = LinuxCompatibilityLayer::new();
        let result = layer.initialize().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_windows_initialize() {
        let mut layer = WindowsCompatibilityLayer::new();
        let result = layer.initialize().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_macos_initialize() {
        let mut layer = MacOSCompatibilityLayer::new();
        let result = layer.initialize().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_legacy_initialize() {
        let mut layer = LegacyCompatibilityLayer::new();
        let result = layer.initialize().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_linux_shutdown() {
        let mut layer = LinuxCompatibilityLayer::new();
        let result = layer.shutdown().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_windows_shutdown() {
        let mut layer = WindowsCompatibilityLayer::new();
        let result = layer.shutdown().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_macos_shutdown() {
        let mut layer = MacOSCompatibilityLayer::new();
        let result = layer.shutdown().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_legacy_shutdown() {
        let mut layer = LegacyCompatibilityLayer::new();
        let result = layer.shutdown().await;

        assert!(result.is_ok());
    }
}
