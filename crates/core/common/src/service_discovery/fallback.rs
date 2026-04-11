// SPDX-License-Identifier: AGPL-3.0-or-later
//! Localhost and socket-based capability fallbacks
//!
//! Second-line fallback when the primary [`super::ServiceDiscovery`] pass returned no services.
//! Uses ecoPrimals runtime sockets, biomeOS capability socket paths, optional
//! `TOADSTOOL_LOCAL_PORT` for native compute, and the deprecated `TOADSTOOL_URL`-style TCP
//! fallback when [`crate::discovery_defaults::LocalhostFallbacks`] allows it.

use std::path::PathBuf;
use std::time::SystemTime;

use tracing::warn;

use crate::constants::PRIMAL_NAME;
use crate::primal_identity::{
    Capability, ComputeCapability, CoordinationCapability, ServiceEndpoint, StorageCapability,
};

use super::types::DiscoveredService;

/// Map a [`Capability`] to the biomeOS socket-path category slug, if one exists.
fn biomeos_category(capability: &Capability) -> Option<&'static str> {
    match capability {
        Capability::Crypto(_) => Some("crypto"),
        Capability::Storage(_) => Some("storage"),
        Capability::Coordination(_) => Some("coordination"),
        Capability::Compute(_) => Some("compute"),
        _ => None,
    }
}

/// Probe `$XDG_RUNTIME_DIR/ecoPrimals/{capability}.sock` (with TMPDIR/temp fallbacks when
/// `XDG_RUNTIME_DIR` is unset) and build [`DiscoveredService`] entries for existing paths.
pub(crate) fn services_from_eco_primals_runtime_sockets() -> Vec<DiscoveredService> {
    const SOCKET_SPECS: &[(&str, Capability)] = &[
        (
            PRIMAL_NAME,
            Capability::Compute(ComputeCapability::NativeExecution),
        ),
        (
            "compute",
            Capability::Compute(ComputeCapability::NativeExecution),
        ),
        (
            "coordination",
            Capability::Coordination(CoordinationCapability::ServiceDiscovery),
        ),
        (
            "storage",
            Capability::Storage(StorageCapability::ObjectStorage),
        ),
    ];

    let runtime_base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("TMPDIR").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    let eco_dir = runtime_base.join("ecoPrimals");

    let mut out = Vec::new();
    for &(slug, ref cap) in SOCKET_SPECS {
        let sock_path = eco_dir.join(format!("{slug}.sock"));
        if !sock_path.exists() {
            continue;
        }
        let url = format!("unix://{}", sock_path.display());
        let endpoint = match ServiceEndpoint::from_url_string(&url) {
            Ok(ep) => ep,
            Err(e) => {
                warn!(path = %sock_path.display(), error = %e, "invalid unix socket URL for ecoPrimals fallback");
                continue;
            }
        };
        out.push(
            DiscoveredService::discovered_now(
                format!("fallback-socket-{slug}"),
                slug,
                "dev",
                vec![cap.clone()],
                vec![endpoint],
            )
            .with_metadata("source", "fallback-unix-socket"),
        );
    }
    out
}

/// Fallback when primary discovery finds no services for this capability.
///
/// Uses ecoPrimals runtime sockets, biomeOS capability socket paths from
/// [`crate::primal_sockets::get_socket_path_for_capability`], optional
/// `TOADSTOOL_LOCAL_PORT` for native compute, and the deprecated TCP fallback.
#[must_use]
pub fn localhost_capability_fallback(capability: &Capability) -> Vec<DiscoveredService> {
    let mut out: Vec<DiscoveredService> = services_from_eco_primals_runtime_sockets()
        .into_iter()
        .filter(|s| s.has_capability(capability))
        .collect();

    let now = SystemTime::now();

    if let Some(cat) = biomeos_category(capability) {
        let path = crate::primal_sockets::get_socket_path_for_capability(cat);
        if path.exists() {
            let path_str = path.display().to_string();
            let already = out.iter().any(|s| {
                s.endpoints
                    .iter()
                    .any(|e| e.protocol == "unix" && e.address == path_str)
            });
            if !already {
                let url = format!("unix://{path_str}");
                if let Ok(endpoint) = ServiceEndpoint::from_url_string(&url) {
                    let ep = endpoint.with_metadata("path", path_str);
                    let slug = path.file_name().and_then(|s| s.to_str()).unwrap_or("sock");
                    let mut svc = DiscoveredService::discovered_now(
                        format!("localhost-fallback-{slug}"),
                        format!("fallback-{cat}"),
                        "dev",
                        vec![capability.clone()],
                        vec![ep],
                    )
                    .with_metadata("source", "localhost-capability-fallback");
                    svc.discovered_at = now;
                    svc.last_seen = now;
                    out.push(svc);
                }
            }
        }
    }

    if matches!(
        capability,
        Capability::Compute(ComputeCapability::NativeExecution)
    ) {
        let port: u16 = std::env::var("TOADSTOOL_LOCAL_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if port > 0 {
            let has_local_port = out.iter().any(|s| {
                s.metadata
                    .get("source")
                    .is_some_and(|v| v == "TOADSTOOL_LOCAL_PORT")
            });
            if !has_local_port {
                let mut svc = DiscoveredService::discovered_now(
                    "localhost-compute-local-port",
                    "localhost-compute",
                    "dev",
                    vec![Capability::Compute(ComputeCapability::NativeExecution)],
                    vec![ServiceEndpoint::http(
                        crate::constants::network::DEFAULT_HOSTNAME,
                        port,
                    )],
                )
                .with_metadata("source", "TOADSTOOL_LOCAL_PORT");
                svc.discovered_at = now;
                svc.last_seen = now;
                out.push(svc);
            }
        }

        let fallbacks = crate::discovery_defaults::LocalhostFallbacks::default();
        if fallbacks.should_use_fallback() {
            if let Some(url) = fallbacks.get_fallback_url(PRIMAL_NAME) {
                if let Ok(ep) = ServiceEndpoint::from_url_string(&url) {
                    let mut svc = DiscoveredService::discovered_now(
                        format!("fallback-{PRIMAL_NAME}"),
                        PRIMAL_NAME,
                        "dev",
                        vec![Capability::Compute(ComputeCapability::NativeExecution)],
                        vec![ep],
                    )
                    .with_metadata("source", "fallback-tcp")
                    .with_metadata("deprecation", "tcp_url_fallback");
                    svc.discovered_at = now;
                    svc.last_seen = now;
                    out.push(svc);
                }
            }
        }
    }

    out
}
