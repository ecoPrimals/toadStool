// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! [`CapabilityDiscovery`] implementation for [`DiscoveryEngine`].

use super::super::capabilities::{
    CapabilityDiscovery, DiscoveredService, DiscoveryError, DiscoveryPreferences, DiscoverySource,
    EndpointSource, ServiceHealth, ServiceMetadata,
};
use super::DiscoveryEngine;
use std::sync::Arc;

impl CapabilityDiscovery for DiscoveryEngine {
    fn discover(
        &self,
        capability: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<DiscoveredService, DiscoveryError>> + Send + '_,
        >,
    > {
        let capability_str = capability.to_string();

        Box::pin(async move {
            // Check cache first
            if let Some(cached) = self.get_from_cache(&capability_str).await {
                tracing::debug!(capability = capability_str, "Using cached discovery");
                return Ok(cached);
            }

            // Discover endpoint
            let endpoint = self.discover_endpoint(&capability_str).await?;

            // Create discovered service
            let service = DiscoveredService {
                capability: capability_str.clone(),
                endpoint,
                protocols: vec!["http".to_string()], // Default, should be detected
                metadata: ServiceMetadata {
                    version: None,
                    health: ServiceHealth::Unknown,
                    last_seen: std::time::SystemTime::now(),
                    priority: 50,
                    extra: std::collections::HashMap::new(),
                },
                source: DiscoverySource::UniversalAdapter,
            };

            // Cache the result
            self.store_in_cache(service.clone()).await;

            Ok(service)
        })
    }

    fn discover_with_preferences(
        &self,
        capability: &str,
        preferences: DiscoveryPreferences,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<DiscoveredService, DiscoveryError>> + Send + '_,
        >,
    > {
        let capability_str = capability.to_string();

        Box::pin(async move {
            let timeout = preferences.timeout.unwrap_or(self.config.default_timeout);

            // Discover service with timeout
            let discovered =
                match tokio::time::timeout(timeout, self.discover(&capability_str)).await {
                    Ok(result) => result?,
                    Err(_) => return Err(DiscoveryError::Timeout(timeout)),
                };

            // Filter by required protocols if specified
            if !preferences.required_protocols.is_empty() {
                let has_all_protocols = preferences.required_protocols.iter().all(|required| {
                    discovered
                        .protocols
                        .iter()
                        .any(|protocol| protocol == required)
                });

                if !has_all_protocols {
                    return Err(DiscoveryError::ProtocolNotSupported(
                        preferences.required_protocols.join(", "),
                    ));
                }
            }

            // Filter by minimum health level
            if discovered.metadata.health < preferences.min_health {
                return Err(DiscoveryError::NoHealthyServices(capability_str.clone()));
            }

            // Prefer local if requested
            if preferences.prefer_local {
                // Check if endpoint is localhost/127.0.0.1
                let is_local = discovered
                    .endpoint
                    .contains(crate::constants::DEFAULT_HOSTNAME)
                    || discovered
                        .endpoint
                        .contains(crate::constants::LOCALHOST_IPV4)
                    || discovered
                        .endpoint
                        .contains(crate::constants::LOCALHOST_IPV6);

                if !is_local {
                    // Try to find a local alternative from cache
                    let cache = self.cache.read().await;
                    if let Some(cached) = cache.get(&capability_str) {
                        let is_cached_local =
                            cached.endpoint.contains(crate::constants::DEFAULT_HOSTNAME)
                                || cached.endpoint.contains(crate::constants::LOCALHOST_IPV4)
                                || cached.endpoint.contains(crate::constants::LOCALHOST_IPV6);
                        if is_cached_local {
                            return Ok(cached.clone());
                        }
                    }
                }
            }

            Ok(discovered)
        })
    }

    fn discover_all(
        &self,
        capability: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<DiscoveredService>, DiscoveryError>>
                + Send
                + '_,
        >,
    > {
        let capability_str = capability.to_string();

        Box::pin(async move {
            let mut discovered_services = Vec::new();
            let sources: Vec<Arc<dyn EndpointSource>> =
                self.sources.read().await.iter().map(Arc::clone).collect();

            // Query all sources sequentially (avoid lifetime issues)
            for source in &sources {
                match source.resolve(&capability_str).await {
                    Ok(Some(endpoint)) => {
                        // Create discovered service from this endpoint
                        let service = DiscoveredService {
                            capability: capability_str.clone(),
                            endpoint,
                            protocols: vec!["http".to_string()], // Default to HTTP
                            metadata: ServiceMetadata {
                                version: None,
                                health: ServiceHealth::Unknown,
                                last_seen: std::time::SystemTime::now(),
                                priority: 50,
                                extra: std::collections::HashMap::new(),
                            },
                            source: DiscoverySource::from(source.source_name()),
                        };

                        discovered_services.push(service);
                    }
                    Ok(None) => {
                        tracing::trace!(
                            "Source {} returned no results for {}",
                            source.source_name(),
                            capability_str
                        );
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Source {} query failed for {}: {}",
                            source.source_name(),
                            capability_str,
                            e
                        );
                    }
                }
            }

            if discovered_services.is_empty() {
                return Err(DiscoveryError::CapabilityNotFound(capability_str.clone()));
            }

            // Deduplicate by endpoint
            discovered_services.sort_by(|a, b| a.endpoint.cmp(&b.endpoint));
            discovered_services.dedup_by(|a, b| a.endpoint == b.endpoint);

            Ok(discovered_services)
        })
    }
}
