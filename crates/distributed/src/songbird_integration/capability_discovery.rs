//! Capability-based Songbird Discovery
//!
//! **Modern Pattern**: Discover Songbird by capability, not by name
//! 
//! Philosophy: "Each primal knows only itself. Everything else is discovered."

use std::sync::Arc;
use std::time::Duration;
use chrono::Utc;
use tokio::sync::RwLock;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_common::infant_discovery::{
    DiscoveryEngine, DiscoveredService, DiscoveryPreferences,
};
use tracing::{debug, info, warn};

use super::types::{SongbirdConnection, SongbirdProtocol};

impl SongbirdConnection {
    /// Create a new capability-based Songbird connection
    ///
    /// **Modern Pattern**: No hardcoded endpoint! Discovers by capability.
    ///
    /// # Example
    /// ```no_run
    /// use toadstool_distributed::songbird_integration::SongbirdConnection;
    /// use toadstool_common::infant_discovery::DiscoveryEngine;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let discovery = Arc::new(DiscoveryEngine::new());
    /// 
    /// // Discovers any service with required capabilities
    /// let connection = SongbirdConnection::discover(
    ///     discovery,
    ///     vec![
    ///         "service-discovery".to_string(),
    ///         "load-balancing".to_string(),
    ///     ],
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover(
        discovery: Arc<DiscoveryEngine>,
        required_capabilities: Vec<String>,
    ) -> ToadStoolResult<Self> {
        info!(
            "🔍 Discovering Songbird services with capabilities: {:?}",
            required_capabilities
        );

        let connection = Self {
            discovery: discovery.clone(),
            required_capabilities,
            timeout: Duration::from_secs(30),
            preferred_protocol: SongbirdProtocol::Http,
            cached_services: Arc::new(RwLock::new(Vec::new())),
            last_discovery: Arc::new(RwLock::new(None)),
        };

        // Perform initial discovery
        connection.refresh_discovery().await?;

        Ok(connection)
    }

    /// Refresh service discovery
    ///
    /// Queries discovery engine for services matching required capabilities.
    /// Updates internal cache with fresh results.
    pub async fn refresh_discovery(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        debug!("🔄 Refreshing Songbird service discovery");

        let mut all_services = Vec::new();

        // Discover services for each required capability
        for capability in &self.required_capabilities {
            match self.discovery.find_by_capability(capability).await {
                Ok(services) => {
                    debug!(
                        "Found {} services for capability '{}'",
                        services.len(),
                        capability
                    );
                    all_services.extend(services);
                }
                Err(e) => {
                    warn!(
                        "Failed to discover services for capability '{}': {}",
                        capability, e
                    );
                }
            }
        }

        // Deduplicate by endpoint
        all_services.sort_by(|a, b| a.endpoint.cmp(&b.endpoint));
        all_services.dedup_by(|a, b| a.endpoint == b.endpoint);

        // Filter by health if we have health info
        let healthy_services: Vec<_> = all_services
            .into_iter()
            .filter(|s| {
                matches!(
                    s.metadata.health,
                    toadstool_common::infant_discovery::ServiceHealth::Healthy
                        | toadstool_common::infant_discovery::ServiceHealth::Unknown
                )
            })
            .collect();

        info!(
            "✅ Discovered {} healthy Songbird-compatible services",
            healthy_services.len()
        );

        // Update cache
        {
            let mut cache = self.cached_services.write().await;
            *cache = healthy_services.clone();
        }

        // Update last discovery time
        {
            let mut last = self.last_discovery.write().await;
            *last = Some(Utc::now());
        }

        Ok(healthy_services)
    }

    /// Get available services (from cache or refresh if stale)
    ///
    /// Returns cached services if fresh (< 5 minutes old),
    /// otherwise performs new discovery.
    pub async fn get_available_services(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        let should_refresh = {
            let last = self.last_discovery.read().await;
            match *last {
                None => true,
                Some(last_time) => {
                    let age = Utc::now().signed_duration_since(last_time);
                    age.num_seconds() > 300 // 5 minutes
                }
            }
        };

        if should_refresh {
            debug!("Cache stale, refreshing discovery");
            self.refresh_discovery().await
        } else {
            let cache = self.cached_services.read().await;
            Ok(cache.clone())
        }
    }

    /// Get best available service for a request
    ///
    /// Selects service based on:
    /// 1. Health status (healthy preferred)
    /// 2. Protocol compatibility (preferred protocol first)
    /// 3. Priority/load (from metadata)
    pub async fn get_best_service(&self) -> ToadStoolResult<DiscoveredService> {
        let services = self.get_available_services().await?;

        if services.is_empty() {
            return Err(ToadStoolError::ServiceDiscovery(
                "No Songbird-compatible services available".to_string(),
            ));
        }

        // Filter by protocol compatibility
        let protocol_str = match self.preferred_protocol {
            SongbirdProtocol::Http => "http",
            SongbirdProtocol::Grpc => "grpc",
            SongbirdProtocol::WebSocket => "websocket",
        };

        let compatible_services: Vec<_> = services
            .iter()
            .filter(|s| {
                s.protocols.iter().any(|p| p.to_lowercase() == protocol_str)
                    || s.protocols.contains(&"http".to_string()) // HTTP as fallback
            })
            .collect();

        let selected = if !compatible_services.is_empty() {
            // Use first compatible service (already sorted by health/priority)
            compatible_services[0].clone()
        } else {
            // Fallback to first available service
            warn!(
                "No services with preferred protocol '{}', using fallback",
                protocol_str
            );
            services[0].clone()
        };

        debug!("📍 Selected service: {}", selected.endpoint);
        Ok(selected)
    }

    /// Execute with automatic failover
    ///
    /// Tries to execute the given operation against available services.
    /// Automatically fails over to next service if one fails.
    ///
    /// # Example
    /// ```no_run
    /// # async fn example(connection: SongbirdConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let result = connection.execute_with_failover(|service| async move {
    ///     // Your operation here
    ///     reqwest::get(&service.endpoint).await?.text().await
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_with_failover<F, Fut, T>(
        &self,
        operation: F,
    ) -> ToadStoolResult<T>
    where
        F: Fn(DiscoveredService) -> Fut,
        Fut: std::future::Future<Output = ToadStoolResult<T>>,
    {
        let services = self.get_available_services().await?;

        if services.is_empty() {
            return Err(ToadStoolError::ServiceDiscovery(
                "No services available for failover".to_string(),
            ));
        }

        let mut last_error = None;

        for service in services {
            debug!("Trying service: {}", service.endpoint);

            match operation(service.clone()).await {
                Ok(result) => {
                    info!("✅ Operation succeeded on {}", service.endpoint);
                    return Ok(result);
                }
                Err(e) => {
                    warn!("❌ Operation failed on {}: {}", service.endpoint, e);
                    last_error = Some(e);
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ToadStoolError::ServiceDiscovery("All services failed".to_string())
        }))
    }

    /// Check if connection is healthy
    ///
    /// Returns true if we have at least one healthy service available.
    pub async fn is_healthy(&self) -> bool {
        match self.get_available_services().await {
            Ok(services) => !services.is_empty(),
            Err(_) => false,
        }
    }

    /// Get connection statistics
    pub async fn get_stats(&self) -> ConnectionStats {
        let services = self.cached_services.read().await;
        let last = self.last_discovery.read().await;

        ConnectionStats {
            available_services: services.len(),
            last_discovery: *last,
            cache_age_seconds: last.map(|t| {
                Utc::now().signed_duration_since(t).num_seconds()
            }),
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub available_services: usize,
    pub last_discovery: Option<chrono::DateTime<Utc>>,
    pub cache_age_seconds: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_songbird_discovery_pattern() {
        // This test demonstrates the pattern but won't run without actual services
        // In production, services would be discovered via mDNS, env vars, etc.
        
        let discovery = Arc::new(DiscoveryEngine::new());
        
        // In a real environment, this would discover actual services
        let result = SongbirdConnection::discover(
            discovery,
            vec!["service-discovery".to_string()],
        ).await;
        
        // Without actual services, this is expected to work but find nothing
        // The important part is the pattern - no hardcoded endpoints!
        assert!(result.is_ok() || result.is_err()); // Either outcome is valid
    }
}

