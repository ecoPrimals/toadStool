// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! TOML configuration file based endpoint resolution.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use crate::infant_discovery::capabilities::{DiscoveryError, EndpointSource};

/// Configuration file source - reads from TOML config
pub struct ConfigFileSource {
    config_path: PathBuf,
}

impl ConfigFileSource {
    /// Create new config file source
    pub fn new(config_path: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
        }
    }

    /// Create with default config path (`config/toadstool.toml`).
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
            match tokio::fs::read_to_string(&config_path).await {
                Ok(contents) => match toml::from_str::<toml::Value>(&contents) {
                    Ok(config) => {
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
                },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_file_source_new() {
        let source = ConfigFileSource::new("/path/to/config.toml");
        assert_eq!(source.config_path.to_str(), Some("/path/to/config.toml"));
        assert_eq!(source.source_name(), "config_file");
    }

    #[test]
    fn test_config_file_source_default_path() {
        let source = ConfigFileSource::default_path();
        assert!(
            source
                .config_path
                .to_str()
                .unwrap()
                .contains("toadstool.toml")
        );
    }

    #[tokio::test]
    async fn test_config_file_source_missing_file() {
        let source = ConfigFileSource::new("/nonexistent/config.toml");
        let result = source.resolve("any_service").await.unwrap();

        assert_eq!(result, None);
    }
}
