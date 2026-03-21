// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

//! Environment-variable based endpoint resolution.

use std::env;
use std::future::Future;
use std::pin::Pin;

use crate::infant_discovery::capabilities::{DiscoveryError, EndpointSource};

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
            env::var(&env_var).map_or_else(
                |_| {
                    tracing::trace!(
                        service = service,
                        env_var = env_var,
                        "No endpoint found in environment"
                    );
                    Ok(None)
                },
                |endpoint| {
                    tracing::debug!(
                        service = service,
                        env_var = env_var,
                        endpoint = endpoint,
                        "Found endpoint in environment"
                    );
                    Ok(Some(endpoint))
                },
            )
        })
    }

    fn source_name(&self) -> &'static str {
        "environment"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_source() {
        temp_env::with_var(
            "TOADSTOOL_TEST_CAPABILITY_ENDPOINT",
            Some("http://test:9999"),
            || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let source = EnvironmentSource::default();
                    let result = source.resolve("test_capability").await.unwrap();
                    assert_eq!(result, Some("http://test:9999".to_string()));
                });
            },
        );
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

    #[test]
    fn test_environment_source_no_env() {
        temp_env::with_var_unset("TOADSTOOL_NONEXISTENT_ENDPOINT", || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let source = EnvironmentSource::default();
                let result = source.resolve("nonexistent").await.unwrap();
                assert_eq!(result, None);
            });
        });
    }

    #[test]
    fn test_environment_source_custom_prefix() {
        temp_env::with_var("MYAPP_SERVICE_ENDPOINT", Some("http://custom:7777"), || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let source = EnvironmentSource::new("MYAPP_");
                let result = source.resolve("service").await.unwrap();
                assert_eq!(result, Some("http://custom:7777".to_string()));
            });
        });
    }
}
