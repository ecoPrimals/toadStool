// SPDX-License-Identifier: AGPL-3.0-only
//! Service-level validation rules for BYOB deployments.
//!
//! Covers minimum service count, concurrent-service limits per team, runnable service
//! definition (image or command), and uniqueness of host port bindings.

use std::collections::HashSet;

use crate::byob::byob_types::{ByobDeploymentRequest, ServiceSpec};
use crate::{ToadStoolError, ToadStoolResult};

/// Validate the services section: non-empty list, count vs quota, and each service spec.
pub(super) fn validate_services(request: &ByobDeploymentRequest) -> ToadStoolResult<()> {
    if request.services.is_empty() {
        return Err(ToadStoolError::validation(
            "Deployment must have at least one service",
        ));
    }

    if request.services.len() > request.resource_quotas.max_concurrent_services as usize {
        return Err(ToadStoolError::resource(format!(
            "Service count {} exceeds team quota {}",
            request.services.len(),
            request.resource_quotas.max_concurrent_services
        )));
    }

    for (name, spec) in &request.services {
        validate_service(name, spec)?;
    }

    Ok(())
}

fn validate_service(name: &str, spec: &ServiceSpec) -> ToadStoolResult<()> {
    if spec.image.is_none() && spec.command.is_none() {
        return Err(ToadStoolError::validation(format!(
            "Service '{name}' must have either image or command specified"
        )));
    }

    let mut seen_host_ports = HashSet::new();
    for port_mapping in &spec.ports {
        if let Some(host_port) = port_mapping.host_port {
            if !seen_host_ports.insert(host_port) {
                return Err(ToadStoolError::validation(format!(
                    "Service '{name}' has duplicate host port: {host_port}"
                )));
            }
        }
    }

    Ok(())
}
