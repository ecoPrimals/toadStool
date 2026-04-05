// SPDX-License-Identifier: AGPL-3.0-or-later

mod capabilities;
mod health;
mod lifecycle;
mod status;

use std::collections::HashMap;

use toadstool_common::primal_identity::{Capability, ComputeCapability, ServiceEndpoint};
use toadstool_common::service_discovery::DiscoveredService;

pub(super) fn create_test_service(name: &str, healthy: bool) -> DiscoveredService {
    create_test_service_with_id(uuid::Uuid::new_v4().to_string().as_str(), name, healthy)
}

pub(super) fn create_test_service_with_id(
    id: &str,
    name: &str,
    healthy: bool,
) -> DiscoveredService {
    DiscoveredService {
        id: id.to_string(),
        name: name.to_string(),
        version: "1.0.0".to_string(),
        capabilities: vec![Capability::Compute(ComputeCapability::NativeExecution)],
        endpoints: vec![ServiceEndpoint::http("localhost", 8080)],
        metadata: HashMap::new(),
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
        healthy,
    }
}

pub(super) fn create_test_service_with_capabilities(
    id: &str,
    name: &str,
    healthy: bool,
    capabilities: Vec<Capability>,
) -> DiscoveredService {
    DiscoveredService {
        id: id.to_string(),
        name: name.to_string(),
        version: "1.0.0".to_string(),
        capabilities,
        endpoints: vec![ServiceEndpoint::http("localhost", 8080)],
        metadata: HashMap::new(),
        discovered_at: std::time::SystemTime::now(),
        last_seen: std::time::SystemTime::now(),
        healthy,
    }
}
