// SPDX-License-Identifier: AGPL-3.0-only
//! Service and endpoint selection for message routing

use crate::config::RoutingStrategy;
use crate::types::{
    HealthStatus, ProtocolError, ProtocolResult, ServiceEndpoint, ServiceInfo, TransportType,
};

/// Select service from candidates based on routing strategy and health
pub fn select_service<'a>(
    services: &'a [ServiceInfo],
    strategy: &RoutingStrategy,
) -> ProtocolResult<&'a ServiceInfo> {
    if services.is_empty() {
        return Err(ProtocolError::Routing("No services available".to_string()));
    }

    match *strategy {
        RoutingStrategy::RoundRobin | RoutingStrategy::Random => services
            .iter()
            .find(|s| s.health_status == HealthStatus::Healthy)
            .or_else(|| services.first())
            .ok_or_else(|| ProtocolError::Routing("No healthy services available".to_string())),
        _ => services
            .first()
            .ok_or_else(|| ProtocolError::Routing("No services available".to_string())),
    }
}

/// Select endpoint from service matching transport and health
pub fn select_endpoint<'a>(
    service: &'a ServiceInfo,
    supported_transports: &[TransportType],
) -> ProtocolResult<&'a ServiceEndpoint> {
    service
        .endpoints
        .iter()
        .find(|e| {
            e.health_status == HealthStatus::Healthy && supported_transports.contains(&e.transport)
        })
        .or_else(|| {
            service
                .endpoints
                .iter()
                .find(|e| supported_transports.contains(&e.transport))
        })
        .ok_or_else(|| ProtocolError::Routing("No suitable endpoints available".to_string()))
}
