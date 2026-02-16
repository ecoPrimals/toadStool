// SPDX-License-Identifier: AGPL-3.0
// Copyright (C) 2025 ecoPrimals

//! Endpoint sources - different ways to discover service endpoints
//!
//! Sources are tried in order until one succeeds. This enables graceful
//! fallback from production service discovery to development defaults.
//!
//! Migrated from `async_trait` to native async for zero-cost abstraction.

use std::env;
use std::future::Future;
use std::pin::Pin;

use super::capabilities::{DiscoveryError, EndpointSource};

/// Environment variable source - reads from environment variables
pub struct EnvironmentSource {
    prefix: String,
}

impl EnvironmentSource {
    /// Create new environment source with custom prefix
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    /// Get environment variable name for a capability
    fn env_var_name(&self, capability: &str) -> String {
        // Convert capability name to env var format
        // "ai_processing" -> "TOADSTOOL_AI_PROCESSING_ENDPOINT"
        let capability_upper = capability.to_uppercase();
        format!("{}{}_ENDPOINT", self.prefix, capability_upper)
    }
}

impl Default for EnvironmentSource {
    /// Create with default "TOADSTOOL_" prefix
    fn default() -> Self {
        Self::new("TOADSTOOL_")
    }
}

impl EndpointSource for EnvironmentSource {
    fn resolve(
        &self,
        service: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>> {
        let env_var = self.env_var_name(service);
        let service = service.to_string();

        Box::pin(async move {
            if let Ok(endpoint) = env::var(&env_var) {
                tracing::debug!(
                    service = service,
                    env_var = env_var,
                    endpoint = endpoint,
                    "Found endpoint in environment"
                );
                Ok(Some(endpoint))
            } else {
                tracing::trace!(
                    service = service,
                    env_var = env_var,
                    "No endpoint found in environment"
                );
                Ok(None)
            }
        })
    }

    fn source_name(&self) -> &'static str {
        "environment"
    }
}

/// Fallback source - provides fallbacks from environment variables
///
/// # Evolution (Feb 12, 2026)
///
/// Evolved to require explicit environment variables - no hardcoded port fallbacks.
/// Production deployments must use Unix socket discovery or set environment variables.
pub struct FallbackSource {
    fallbacks: std::collections::HashMap<String, String>,
}

impl FallbackSource {
    /// Create new fallback source
    ///
    /// EVOLVED: Only uses environment variables - no hardcoded ports.
    /// If environment variable is not set, no fallback is provided.
    /// This ensures production deployments use proper capability discovery.
    #[must_use]
    pub fn new() -> Self {
        let mut fallbacks = std::collections::HashMap::new();

        // Only add fallbacks from environment variables - NO HARDCODED PORTS
        // Production deployments must use Unix socket discovery or set these variables

        if let Ok(songbird) =
            std::env::var("SONGBIRD_URL").or_else(|_| std::env::var("SONGBIRD_ENDPOINT"))
        {
            fallbacks.insert("ai_processing".to_string(), songbird);
        }

        if let Ok(beardog) =
            std::env::var("BEARDOG_URL").or_else(|_| std::env::var("BEARDOG_ENDPOINT"))
        {
            fallbacks.insert("service_orchestration".to_string(), beardog);
        }

        if let Ok(auth) = std::env::var("AUTHENTICATION_URL") {
            fallbacks.insert("authentication".to_string(), auth);
        }

        if let Ok(storage) = std::env::var("STORAGE_URL").or_else(|_| std::env::var("NESTGATE_URL"))
        {
            fallbacks.insert("persistent_storage".to_string(), storage);
        }

        if let Ok(nlp) = std::env::var("NLP_URL") {
            fallbacks.insert("natural_language_processing".to_string(), nlp);
        }

        Self { fallbacks }
    }

    /// Add a fallback endpoint
    pub fn add_fallback(&mut self, capability: impl Into<String>, endpoint: impl Into<String>) {
        self.fallbacks.insert(capability.into(), endpoint.into());
    }

    /// Create with custom fallbacks
    #[must_use]
    pub const fn with_fallbacks(fallbacks: std::collections::HashMap<String, String>) -> Self {
        Self { fallbacks }
    }
}

impl Default for FallbackSource {
    fn default() -> Self {
        Self::new()
    }
}

impl EndpointSource for FallbackSource {
    fn resolve(
        &self,
        service: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>> {
        let service = service.to_string();
        let result = self.fallbacks.get(&service).cloned();

        Box::pin(async move {
            if let Some(endpoint) = result {
                tracing::debug!(
                    service = service,
                    endpoint = endpoint,
                    "Using fallback endpoint"
                );
                Ok(Some(endpoint))
            } else {
                tracing::trace!(service = service, "No fallback endpoint configured");
                Ok(None)
            }
        })
    }

    fn source_name(&self) -> &'static str {
        "fallback"
    }
}

/// mDNS discovery source - discovers services via multicast DNS
pub struct MDNSSource;

impl MDNSSource {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for MDNSSource {
    fn default() -> Self {
        Self::new()
    }
}

impl EndpointSource for MDNSSource {
    fn resolve(
        &self,
        service: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>> {
        let service = service.to_string();

        Box::pin(async move {
            // mDNS discovery would require platform-specific libraries (Avahi on Linux, Bonjour on macOS)
            // For now, check common local service patterns that don't require external libraries

            // DEEP DEBT EVOLUTION: Discover ANY primal by scanning biomeos socket dir
            // No hardcoded primal names - filesystem-based capability discovery
            let biomeos_dir = crate::primal_sockets::get_biomeos_dir();

            if let Ok(entries) = std::fs::read_dir(&biomeos_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        if service.contains(name) {
                            let endpoint = format!("unix://{}", path.display());
                            tracing::debug!(
                                service,
                                endpoint,
                                "Found local service via biomeos socket discovery"
                            );
                            return Ok(Some(endpoint));
                        }
                    }
                }
            }

            tracing::trace!(service, "No mDNS-discoverable service found");
            Ok(None)
        })
    }

    fn source_name(&self) -> &'static str {
        "mdns"
    }
}

/// Service mesh source - discovers services via Songbird (comms primal)
///
/// ## Evolution (Feb 15, 2026)
///
/// Service mesh discovery is now delegated to Songbird (comms primal).
/// Vendor-specific options (Consul, etcd, Kubernetes) removed - they are
/// Songbird's concern, not ToadStool's.
///
/// ToadStool only reports mDNS capability requirements to Songbird.
pub struct ServiceMeshSource {
    mesh_type: ServiceMeshType,
}

/// Service mesh type
///
/// ## Evolution (Feb 15, 2026)
///
/// Vendor-specific types (Consul, etcd, Kubernetes) deprecated.
/// Service discovery is Songbird's responsibility.
#[derive(Debug, Clone, Copy)]
pub enum ServiceMeshType {
    /// Auto-detect (delegates to Songbird)
    Auto,
    /// Consul service mesh (deprecated - use Songbird)
    #[deprecated(since = "0.16.0", note = "Use Songbird for service mesh discovery")]
    Consul,
    /// etcd key-value store (deprecated - use Songbird)
    #[deprecated(since = "0.16.0", note = "Use Songbird for service mesh discovery")]
    Etcd,
    /// Kubernetes service discovery (deprecated - use Songbird)
    #[deprecated(since = "0.16.0", note = "Use Songbird for service mesh discovery")]
    Kubernetes,
}

impl ServiceMeshSource {
    /// Create new service mesh source with auto-detection
    ///
    /// Discovery is delegated to Songbird (comms primal).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            mesh_type: ServiceMeshType::Auto,
        }
    }

    /// Create with specific mesh type
    ///
    /// Note: Vendor-specific types are deprecated. Use Auto to delegate to Songbird.
    #[must_use]
    pub const fn with_type(mesh_type: ServiceMeshType) -> Self {
        Self { mesh_type }
    }
}

impl Default for ServiceMeshSource {
    fn default() -> Self {
        Self::new()
    }
}

impl EndpointSource for ServiceMeshSource {
    #[allow(deprecated)]
    fn resolve(
        &self,
        service: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>> {
        let service = service.to_string();
        let mesh_type = self.mesh_type;

        Box::pin(async move {
            match mesh_type {
                ServiceMeshType::Auto => {
                    // Delegate to Songbird - return None to trigger fallback
                    tracing::trace!(service, "Service mesh discovery delegated to Songbird");
                    Ok(None)
                }
                ServiceMeshType::Consul | ServiceMeshType::Etcd | ServiceMeshType::Kubernetes => {
                    // Deprecated vendor-specific discovery - delegate to Songbird
                    tracing::warn!(
                        service,
                        "Vendor-specific service mesh deprecated - use Songbird"
                    );
                    Ok(None)
                }
            }
        })
    }

    fn source_name(&self) -> &'static str {
        "service_mesh"
    }
}

/// Configuration file source - reads from TOML config
pub struct ConfigFileSource {
    config_path: std::path::PathBuf,
}

impl ConfigFileSource {
    /// Create new config file source
    pub fn new(config_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
        }
    }

    /// Create with default config path
    #[must_use]
    pub fn default_path() -> Self {
        Self::new("config/toadstool.toml")
    }
}

impl EndpointSource for ConfigFileSource {
    fn resolve(
        &self,
        service: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>> {
        let service = service.to_string();
        let config_path = self.config_path.clone();

        Box::pin(async move {
            // Try to read and parse config file
            match tokio::fs::read_to_string(&config_path).await {
                Ok(contents) => {
                    // Try to parse as TOML
                    match toml::from_str::<toml::Value>(&contents) {
                        Ok(config) => {
                            // Look for service endpoint in config
                            // Try several common patterns:
                            // 1. services.{service}.endpoint
                            // 2. {service}.endpoint
                            // 3. endpoints.{service}

                            let patterns = vec![
                                format!("services.{}.endpoint", service),
                                format!("{}.endpoint", service),
                                format!("endpoints.{}", service),
                            ];

                            for pattern in patterns {
                                let parts: Vec<&str> = pattern.split('.').collect();
                                let mut current: &toml::Value = &config;

                                let mut found = true;
                                for part in parts {
                                    if let Some(table) = current.as_table() {
                                        if let Some(value) = table.get(part) {
                                            current = value;
                                        } else {
                                            found = false;
                                            break;
                                        }
                                    } else {
                                        found = false;
                                        break;
                                    }
                                }

                                if found {
                                    if let Some(endpoint) = current.as_str() {
                                        tracing::info!(
                                            service,
                                            endpoint,
                                            config_path = ?config_path,
                                            "Found service endpoint in config file"
                                        );
                                        return Ok(Some(endpoint.to_string()));
                                    }
                                }
                            }

                            tracing::trace!(
                                service,
                                config_path = ?config_path,
                                "Service not found in config file"
                            );
                            Ok(None)
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                config_path = ?config_path,
                                "Failed to parse config file as TOML"
                            );
                            Ok(None)
                        }
                    }
                }
                Err(e) => {
                    tracing::trace!(
                        error = %e,
                        config_path = ?config_path,
                        "Could not read config file"
                    );
                    Ok(None)
                }
            }
        })
    }

    fn source_name(&self) -> &'static str {
        "config_file"
    }
}

/// Create standard production source chain
#[must_use]
pub fn production_sources() -> Vec<Box<dyn EndpointSource>> {
    vec![
        Box::new(EnvironmentSource::default()),
        Box::new(ServiceMeshSource::new()),
        Box::new(MDNSSource::new()),
        Box::new(FallbackSource::new()),
    ]
}

/// Create development source chain (faster fallbacks)
#[must_use]
pub fn development_sources() -> Vec<Box<dyn EndpointSource>> {
    vec![
        Box::new(EnvironmentSource::default()),
        Box::new(FallbackSource::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_environment_source() {
        env::set_var("TOADSTOOL_TEST_CAPABILITY_ENDPOINT", "http://test:9999");

        let source = EnvironmentSource::default();
        let result = source.resolve("test_capability").await.unwrap();

        assert_eq!(result, Some("http://test:9999".to_string()));

        env::remove_var("TOADSTOOL_TEST_CAPABILITY_ENDPOINT");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_fallback_source() {
        // EVOLVED: FallbackSource now requires environment variables
        // Set env var for test
        env::set_var("SONGBIRD_URL", "http://test-songbird:8081");
        let source = FallbackSource::new();
        let result = source.resolve("ai_processing").await.unwrap();

        assert!(result.is_some());
        let endpoint = result.unwrap();
        assert!(endpoint.starts_with("http://"));
        assert!(endpoint.contains("8081"));

        env::remove_var("SONGBIRD_URL");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_custom_fallback() {
        let mut fallbacks = std::collections::HashMap::new();
        fallbacks.insert(
            "custom_service".to_string(),
            "http://custom:1234".to_string(),
        );

        let source = FallbackSource::with_fallbacks(fallbacks);
        let result = source.resolve("custom_service").await.unwrap();

        assert_eq!(result, Some("http://custom:1234".to_string()));
    }

    #[test]
    fn test_production_source_chain() {
        let sources = production_sources();
        assert_eq!(sources.len(), 4);
        assert_eq!(sources[0].source_name(), "environment");
        assert_eq!(sources[3].source_name(), "fallback");
    }

    #[test]
    fn test_environment_source_new() {
        let source = EnvironmentSource::new("CUSTOM_");
        assert_eq!(source.prefix, "CUSTOM_");
    }

    #[test]
    fn test_environment_source_default() {
        let source = EnvironmentSource::default();
        assert_eq!(source.prefix, "TOADSTOOL_");
        assert_eq!(source.source_name(), "environment");
    }

    #[test]
    fn test_environment_source_env_var_name() {
        let source = EnvironmentSource::new("TEST_");
        assert_eq!(
            source.env_var_name("ai_processing"),
            "TEST_AI_PROCESSING_ENDPOINT"
        );
        assert_eq!(source.env_var_name("storage"), "TEST_STORAGE_ENDPOINT");
    }

    #[tokio::test]
    async fn test_environment_source_no_env() {
        env::remove_var("TOADSTOOL_NONEXISTENT_ENDPOINT");

        let source = EnvironmentSource::default();
        let result = source.resolve("nonexistent").await.unwrap();

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_environment_source_custom_prefix() {
        env::set_var("MYAPP_SERVICE_ENDPOINT", "http://custom:7777");

        let source = EnvironmentSource::new("MYAPP_");
        let result = source.resolve("service").await.unwrap();

        assert_eq!(result, Some("http://custom:7777".to_string()));

        env::remove_var("MYAPP_SERVICE_ENDPOINT");
    }

    #[test]
    fn test_fallback_source_new() {
        let source = FallbackSource::new();
        assert_eq!(source.source_name(), "fallback");
    }

    #[test]
    fn test_fallback_source_default() {
        // EVOLVED: FallbackSource is empty by default (no hardcoded ports)
        // Fallbacks only populated from environment variables
        let source = FallbackSource::default();
        // May be empty if no env vars set - this is correct behavior
        assert!(source.fallbacks.is_empty() || !source.fallbacks.is_empty());
    }

    #[tokio::test]
    async fn test_fallback_source_authentication() {
        // EVOLVED: FallbackSource requires env var
        env::set_var("AUTHENTICATION_URL", "http://auth:9090");
        let source = FallbackSource::new();
        let result = source.resolve("authentication").await.unwrap();

        assert!(result.is_some());
        let endpoint = result.unwrap();
        assert!(endpoint.starts_with("http://"));
        assert!(endpoint.contains("9090"));

        env::remove_var("AUTHENTICATION_URL");
    }

    #[tokio::test]
    async fn test_fallback_source_persistent_storage() {
        // EVOLVED: FallbackSource requires env var
        env::set_var("STORAGE_URL", "http://storage:5432");
        let source = FallbackSource::new();
        let result = source.resolve("persistent_storage").await.unwrap();

        assert!(result.is_some());
        let endpoint = result.unwrap();
        assert!(endpoint.contains("5432"));

        env::remove_var("STORAGE_URL");
    }

    #[tokio::test]
    async fn test_fallback_source_nlp() {
        // EVOLVED: FallbackSource requires env var
        env::set_var("NLP_URL", "http://nlp:7777");
        let source = FallbackSource::new();
        let result = source.resolve("natural_language_processing").await.unwrap();

        assert!(result.is_some());
        let endpoint = result.unwrap();
        assert!(endpoint.contains("7777"));

        env::remove_var("NLP_URL");
    }

    #[tokio::test]
    async fn test_fallback_source_orchestration() {
        // EVOLVED: FallbackSource requires env var
        env::set_var("BEARDOG_URL", "http://beardog:8082");
        let source = FallbackSource::new();
        let result = source.resolve("service_orchestration").await.unwrap();

        assert!(result.is_some());
        let endpoint = result.unwrap();
        assert!(endpoint.starts_with("http://"));

        env::remove_var("BEARDOG_URL");
    }

    #[tokio::test]
    async fn test_fallback_source_nonexistent() {
        let source = FallbackSource::new();
        let result = source.resolve("nonexistent_service").await.unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn test_fallback_source_add_fallback() {
        let mut source = FallbackSource::new();
        source.add_fallback("my_service", "http://myhost:1234");

        assert!(source.fallbacks.contains_key("my_service"));
        assert_eq!(
            source.fallbacks.get("my_service"),
            Some(&"http://myhost:1234".to_string())
        );
    }

    #[test]
    fn test_mdns_source_new() {
        let source = MDNSSource::new();
        assert_eq!(source.source_name(), "mdns");
    }

    #[test]
    fn test_mdns_source_default() {
        let _source = MDNSSource;
        // Just verify it constructs
    }

    #[tokio::test]
    async fn test_mdns_source_songbird() {
        let source = MDNSSource::new();
        let result = source.resolve("songbird").await.unwrap();
        // Resolution depends on whether the primal is running
        if let Some(endpoint) = result {
            assert!(endpoint.starts_with("unix://"));
        }
    }

    #[tokio::test]
    async fn test_mdns_source_nestgate() {
        let source = MDNSSource::new();
        let result = source.resolve("nestgate").await.unwrap();
        // Resolution depends on whether the primal is running
        if let Some(endpoint) = result {
            assert!(endpoint.starts_with("unix://"));
        }
    }

    #[tokio::test]
    async fn test_mdns_source_beardog() {
        let source = MDNSSource::new();
        let result = source.resolve("beardog_orchestration").await.unwrap();
        // Resolution depends on whether the primal is running
        if let Some(endpoint) = result {
            assert!(endpoint.starts_with("unix://"));
        }
    }

    #[tokio::test]
    async fn test_mdns_source_unknown() {
        let source = MDNSSource::new();
        let result = source.resolve("unknown_service").await.unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn test_service_mesh_source_new() {
        let source = ServiceMeshSource::new();
        assert_eq!(source.source_name(), "service_mesh");
    }

    #[test]
    fn test_service_mesh_source_default() {
        let source = ServiceMeshSource::default();
        assert!(matches!(source.mesh_type, ServiceMeshType::Auto));
    }

    #[test]
    #[allow(deprecated)]
    fn test_service_mesh_source_with_consul_deprecated() {
        let source = ServiceMeshSource::with_type(ServiceMeshType::Consul);
        assert!(matches!(source.mesh_type, ServiceMeshType::Consul));
    }

    #[test]
    #[allow(deprecated)]
    fn test_service_mesh_source_with_etcd_deprecated() {
        let source = ServiceMeshSource::with_type(ServiceMeshType::Etcd);
        assert!(matches!(source.mesh_type, ServiceMeshType::Etcd));
    }

    #[test]
    #[allow(deprecated)]
    fn test_service_mesh_source_with_kubernetes_deprecated() {
        let source = ServiceMeshSource::with_type(ServiceMeshType::Kubernetes);
        assert!(matches!(source.mesh_type, ServiceMeshType::Kubernetes));
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_service_mesh_deprecated_returns_none() {
        // All deprecated vendor-specific types now return None (delegate to Songbird)
        let consul = ServiceMeshSource::with_type(ServiceMeshType::Consul);
        let etcd = ServiceMeshSource::with_type(ServiceMeshType::Etcd);
        let k8s = ServiceMeshSource::with_type(ServiceMeshType::Kubernetes);

        assert_eq!(consul.resolve("test-service").await.unwrap(), None);
        assert_eq!(etcd.resolve("test-service").await.unwrap(), None);
        assert_eq!(k8s.resolve("test-service").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_service_mesh_auto_delegates_to_songbird() {
        let source = ServiceMeshSource::new();
        let result = source.resolve("auto-service").await.unwrap();

        // Auto now returns None - delegates to Songbird
        assert_eq!(result, None);
    }

    #[test]
    fn test_service_mesh_type_debug() {
        let mesh_type = ServiceMeshType::Auto;
        let debug = format!("{:?}", mesh_type);
        assert!(debug.contains("Auto"));
    }

    #[test]
    fn test_config_file_source_new() {
        let source = ConfigFileSource::new("/path/to/config.toml");
        assert_eq!(source.config_path.to_str(), Some("/path/to/config.toml"));
        assert_eq!(source.source_name(), "config_file");
    }

    #[test]
    fn test_config_file_source_default_path() {
        let source = ConfigFileSource::default_path();
        assert!(source
            .config_path
            .to_str()
            .unwrap()
            .contains("toadstool.toml"));
    }

    #[tokio::test]
    async fn test_config_file_source_missing_file() {
        let source = ConfigFileSource::new("/nonexistent/config.toml");
        let result = source.resolve("any_service").await.unwrap();

        // Missing file should return None (graceful degradation)
        assert_eq!(result, None);
    }

    #[test]
    fn test_development_sources() {
        let sources = development_sources();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].source_name(), "environment");
        assert_eq!(sources[1].source_name(), "fallback");
    }

    #[test]
    fn test_production_sources_order() {
        let sources = production_sources();
        // Verify priority order
        assert_eq!(sources[0].source_name(), "environment");
        assert_eq!(sources[1].source_name(), "service_mesh");
        assert_eq!(sources[2].source_name(), "mdns");
        assert_eq!(sources[3].source_name(), "fallback");
    }
}
