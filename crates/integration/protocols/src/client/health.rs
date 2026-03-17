// SPDX-License-Identifier: AGPL-3.0-only
//! Health monitoring for protocol services

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};
use tracing::debug;

use crate::config::HealthConfig;
use crate::types::{HealthStatus, ProtocolEvent, ServiceInfo};

/// Spawn background health monitoring task for registered services
pub fn spawn_health_monitor(
    services: Arc<RwLock<HashMap<String, ServiceInfo>>>,
    health_config: HealthConfig,
    event_bus: broadcast::Sender<ProtocolEvent>,
) {
    let interval = health_config.base.interval;

    tokio::spawn(async move {
        let mut interval_timer = tokio::time::interval(interval);

        loop {
            interval_timer.tick().await;

            let mut services_snapshot = services.write().await;
            let mut health_updates = Vec::new();

            for (service_id, service_info) in services_snapshot.iter() {
                for endpoint in &service_info.endpoints {
                    let addr = format!("{}:{}", endpoint.address, endpoint.port);
                    let is_healthy = match tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        tokio::net::TcpStream::connect(&addr),
                    )
                    .await
                    {
                        Ok(Ok(_stream)) => {
                            debug!("✅ Service {} endpoint {} is healthy", service_id, addr);
                            true
                        }
                        Ok(Err(e)) => {
                            debug!(
                                "⚠️ Service {} endpoint {} connection failed: {}",
                                service_id, addr, e
                            );
                            false
                        }
                        Err(_) => {
                            debug!("⚠️ Service {} endpoint {} timed out", service_id, addr);
                            false
                        }
                    };

                    health_updates.push((service_id.clone(), endpoint.id.clone(), is_healthy));
                }
            }

            for (service_id, _endpoint_id, is_healthy) in health_updates {
                if let Some(service_info) = services_snapshot.get_mut(&service_id) {
                    let new_status = if is_healthy {
                        HealthStatus::Healthy
                    } else {
                        HealthStatus::Unhealthy
                    };

                    if service_info.health_status != new_status {
                        service_info.health_status = new_status.clone();
                        if event_bus
                            .send(ProtocolEvent::ServiceHealthChanged {
                                service_id: service_id.clone(),
                                status: new_status,
                            })
                            .is_err()
                        {
                            tracing::debug!("No event receivers for ServiceHealthChanged");
                        }
                    }
                }
            }

            let len = services_snapshot.len();
            drop(services_snapshot);
            debug!("Health check cycle completed for {} services", len);
        }
    });
}
