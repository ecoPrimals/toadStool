// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for ecosystem discovery helpers and [`super::EcosystemDiscoverer`].

use crate::ecosystem_types::{ServiceInfo, ServiceStatus};

mod discoverer_behaviour;
mod env_and_merge;

pub(super) fn sample_service_info(name: &str, endpoint: &str) -> ServiceInfo {
    ServiceInfo {
        name: name.to_string(),
        endpoint: endpoint.to_string(),
        service_type: "Compute".to_string(),
        version: "1".to_string(),
        capabilities: vec![],
        status: ServiceStatus::Healthy,
        discovered_via: "test".to_string(),
        response_time_ms: 0,
    }
}
