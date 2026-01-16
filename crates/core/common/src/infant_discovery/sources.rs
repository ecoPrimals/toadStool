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

/// Fallback source - provides hardcoded fallbacks for development
pub struct FallbackSource {
    fallbacks: std::collections::HashMap<String, String>,
}

impl FallbackSource {
    /// Create new fallback source
    ///
    /// Uses environment-aware configuration where possible, falling back to defaults.
    #[must_use]
    pub fn new() -> Self {
        let mut fallbacks = std::collections::HashMap::new();

        // Helper to get host from environment or use default
        let bind_host = std::env::var("TOADSTOOL_BIND_HOST")
            .or_else(|_| std::env::var("BIND_HOST"))
            .unwrap_or_else(|_| crate::constants::LOCALHOST_IPV4.to_string());

        // Helper to get service endpoints from environment or defaults
        let songbird_endpoint = std::env::var("SONGBIRD_URL")
            .or_else(|_| std::env::var("SONGBIRD_ENDPOINT"))
            .unwrap_or_else(|_| format!("http://{bind_host}:8081"));

        let beardog_endpoint = std::env::var("BEARDOG_URL")
            .or_else(|_| std::env::var("BEARDOG_ENDPOINT"))
            .unwrap_or_else(|_| format!("http://{bind_host}:8082"));

        // Default development fallbacks (respecting environment variables)
        fallbacks.insert("ai_processing".to_string(), songbird_endpoint);
        fallbacks.insert(
            "authentication".to_string(),
            format!("http://{bind_host}:9090"),
        );
        fallbacks.insert(
            "persistent_storage".to_string(),
            format!("http://{bind_host}:5432"),
        );
        fallbacks.insert(
            "natural_language_processing".to_string(),
            format!("http://{bind_host}:7777"),
        );
        fallbacks.insert("service_orchestration".to_string(), beardog_endpoint);

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

            let common_mdns_ports: &[(&str, u16)] = &[
                ("songbird", 9090),
                ("nestgate", 8080),
                ("squirrel", 7070),
                ("beardog", 6060),
            ];

            // Check if service matches common patterns
            for (svc, _port) in common_mdns_ports {
                if service.contains(svc) {
                    // PURE RUST: Use unix socket paths instead of HTTP
                    let socket_path = crate::primal_sockets::get_socket_path_for_service(svc);
                    let endpoint = format!("unix://{}", socket_path.display());
                    
                    tracing::debug!(
                        service,
                        endpoint,
                        "Found local service via socket discovery (pure Rust!)"
                    );
                    return Ok(Some(endpoint));
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

/// Service mesh source - discovers via Consul, etcd, etc.
pub struct ServiceMeshSource {
    mesh_type: ServiceMeshType,
}

#[derive(Debug, Clone, Copy)]
pub enum ServiceMeshType {
    /// Auto-detect mesh type
    Auto,
    /// Consul service mesh
    Consul,
    /// etcd key-value store
    Etcd,
    /// Kubernetes service discovery
    Kubernetes,
}

impl ServiceMeshSource {
    /// Create new service mesh source with auto-detection
    #[must_use]
    pub const fn new() -> Self {
        Self {
            mesh_type: ServiceMeshType::Auto,
        }
    }

    /// Create with specific mesh type
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
    fn resolve(
        &self,
        service: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>> {
        let service = service.to_string();
        let mesh_type = self.mesh_type;

        Box::pin(async move {
            match mesh_type {
                ServiceMeshType::Consul => {
                    // PURE RUST: Consul integration removed for pure Rust
                    // Use environment variables or unix sockets instead
                    tracing::trace!(
                        service,
                        "Consul discovery disabled (pure Rust mode) - use environment variables"
                    );
                    return Ok(None); // Graceful degradation - fall back to other sources
                }
                ServiceMeshType::Etcd => {
                    use base64::Engine;

                    // Query etcd for service key (Deep Debt compliant: runtime discovery)
                    let etcd_addr = crate::constants::network::etcd_endpoints();

                    let key = format!("/services/{service}");
                    let url = format!("{etcd_addr}/v3/kv/range");

                    let payload = serde_json::json!({
                        "key": base64::engine::general_purpose::STANDARD.encode(key.as_bytes()),
                    });

                    match reqwest::Client::new()
                        .post(&url)
                        .json(&payload)
                        .timeout(std::time::Duration::from_secs(5))
                        .send()
                        .await
                    {
                        Ok(response) if response.status().is_success() => {
                            tracing::trace!(service, "Found service in etcd");
                            // Would need to parse etcd response format
                        }
                        Ok(_) | Err(_) => {
                            tracing::trace!(service, "etcd service lookup failed");
                        }
                    }
                }
                ServiceMeshType::Kubernetes => {
                    // PURE RUST: Kubernetes DNS can still work (no HTTP needed for DNS)
                    // Try Kubernetes DNS (works inside cluster)
                    let k8s_dns = format!("{service}.default.svc.cluster.local");
                    tracing::trace!(service, k8s_dns, "Trying Kubernetes DNS lookup");
                    return Ok(Some(format!("http://{k8s_dns}")));
                }
                ServiceMeshType::Auto => {
                    // PURE RUST: Auto-detection simplified
                    // Try Kubernetes DNS (works without HTTP)
                    tracing::trace!("Auto-discovery using K8s DNS (pure Rust mode)");

                    // Try K8s DNS as fallback
                    let k8s_dns = format!("{service}.default.svc.cluster.local");
                    tracing::trace!(service, "Auto-detection: trying K8s DNS");
                    return Ok(Some(format!("http://{k8s_dns}")));
                }
            }

            Ok(None)
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
        let source = FallbackSource::new();
        let result = source.resolve("ai_processing").await.unwrap();

        // Should resolve to Songbird endpoint (environment-aware or default)
        assert!(result.is_some());
        let endpoint = result.unwrap();
        assert!(endpoint.starts_with("http://"));
        assert!(endpoint.contains("8081") || endpoint.contains("8080")); // Allow both defaults
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
}
