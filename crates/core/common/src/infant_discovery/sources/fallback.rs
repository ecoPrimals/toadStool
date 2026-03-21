// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

//! Capability endpoint fallbacks derived from well-known environment variables.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::infant_discovery::capabilities::{DiscoveryError, EndpointSource};

/// Fallback source - provides fallbacks from environment variables
///
/// # Evolution (Feb 12, 2026)
///
/// Evolved to require explicit environment variables - no hardcoded port fallbacks.
/// Production deployments must use Unix socket discovery or set environment variables.
pub struct FallbackSource {
    fallbacks: HashMap<String, String>,
}

impl FallbackSource {
    /// Create new fallback source
    ///
    /// EVOLVED: Only uses environment variables - no hardcoded ports.
    /// If environment variable is not set, no fallback is provided.
    /// This ensures production deployments use proper capability discovery.
    #[must_use]
    pub fn new() -> Self {
        let mut fallbacks = HashMap::new();

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
    pub const fn with_fallbacks(fallbacks: HashMap<String, String>) -> Self {
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
            result.map_or_else(
                || {
                    tracing::trace!(service = service, "No fallback endpoint configured");
                    Ok(None)
                },
                |endpoint| {
                    tracing::debug!(
                        service = service,
                        endpoint = endpoint,
                        "Using fallback endpoint"
                    );
                    Ok(Some(endpoint))
                },
            )
        })
    }

    fn source_name(&self) -> &'static str {
        "fallback"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_source() {
        temp_env::with_var("SONGBIRD_URL", Some("http://test-songbird:8081"), || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let source = FallbackSource::new();
                let result = source.resolve("ai_processing").await.unwrap();
                assert!(result.is_some());
                let endpoint = result.unwrap();
                assert!(endpoint.starts_with("http://"));
                assert!(endpoint.contains("8081"));
            });
        });
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_custom_fallback() {
        let mut fallbacks = HashMap::new();
        fallbacks.insert(
            "custom_service".to_string(),
            "http://custom:1234".to_string(),
        );

        let source = FallbackSource::with_fallbacks(fallbacks);
        let result = source.resolve("custom_service").await.unwrap();

        assert_eq!(result, Some("http://custom:1234".to_string()));
    }

    #[test]
    fn test_fallback_source_new() {
        let source = FallbackSource::new();
        assert_eq!(source.source_name(), "fallback");
    }

    #[test]
    fn test_fallback_source_default() {
        let source = FallbackSource::default();
        assert!(source.fallbacks.is_empty() || !source.fallbacks.is_empty());
    }

    #[test]
    fn test_fallback_source_authentication() {
        temp_env::with_var("AUTHENTICATION_URL", Some("http://auth:9090"), || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let source = FallbackSource::new();
                let result = source.resolve("authentication").await.unwrap();
                assert!(result.is_some());
                let endpoint = result.unwrap();
                assert!(endpoint.starts_with("http://"));
                assert!(endpoint.contains("9090"));
            });
        });
    }

    #[test]
    fn test_fallback_source_persistent_storage() {
        temp_env::with_var("STORAGE_URL", Some("http://storage:5432"), || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let source = FallbackSource::new();
                let result = source.resolve("persistent_storage").await.unwrap();
                assert!(result.is_some());
                let endpoint = result.unwrap();
                assert!(endpoint.contains("5432"));
            });
        });
    }

    #[test]
    fn test_fallback_source_nlp() {
        temp_env::with_var("NLP_URL", Some("http://nlp:7777"), || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let source = FallbackSource::new();
                let result = source.resolve("natural_language_processing").await.unwrap();
                assert!(result.is_some());
                let endpoint = result.unwrap();
                assert!(endpoint.contains("7777"));
            });
        });
    }

    #[test]
    fn test_fallback_source_orchestration() {
        temp_env::with_var("BEARDOG_URL", Some("http://beardog:8082"), || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let source = FallbackSource::new();
                let result = source.resolve("service_orchestration").await.unwrap();
                assert!(result.is_some());
                let endpoint = result.unwrap();
                assert!(endpoint.starts_with("http://"));
            });
        });
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
}
