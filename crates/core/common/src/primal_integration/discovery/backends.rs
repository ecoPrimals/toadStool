// SPDX-License-Identifier: AGPL-3.0-or-later
//! Discovery backends: mDNS, Kubernetes, Docker Compose, registry, filesystem.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use tracing::debug;

use crate::constants::network::HTTP_PROTOCOL;
use crate::interned_strings::socket_env;

use super::super::PrimalEndpoint;
use super::discovery_http_port;

/// Probe mDNS/DNS-SD for services advertising the given capability.
///
/// Browses for `_toadstool._tcp.local.` and filters by capability in TXT records
/// (`cap_{name}`). Uses `mdns-sd` crate. Returns `None` when mDNS daemon is
/// unavailable or no matching services are found within the probe timeout.
///
/// When compiled without the `mdns` feature, always returns `None`.
#[must_use]
#[cfg(feature = "mdns")]
pub fn try_discover_via_mdns(capability: &str) -> Option<Vec<PrimalEndpoint>> {
    const MDNS_PROBE_TIMEOUT_SECS: u64 = 2;
    const MDNS_PROBE_TIMEOUT_TEST_MS: u64 = 50;

    debug!("Probing mDNS for capability '{}'", capability);

    let mdns = mdns_sd::ServiceDaemon::new().ok()?;
    let service_type = "_toadstool._tcp.local.";
    let receiver = mdns.browse(service_type).ok()?;
    let timeout = if cfg!(test) {
        Duration::from_millis(MDNS_PROBE_TIMEOUT_TEST_MS)
    } else {
        Duration::from_secs(MDNS_PROBE_TIMEOUT_SECS)
    };
    let deadline = std::time::Instant::now() + timeout;
    let mut discovered = Vec::new();

    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        match receiver.recv_timeout(remaining) {
            Ok(mdns_sd::ServiceEvent::ServiceResolved(info)) => {
                let mut capabilities = Vec::new();
                for prop in info.get_properties().iter() {
                    let key = prop.key();
                    if let Some(cap_name) = key.strip_prefix("cap_")
                        && !cap_name.ends_with("_features")
                    {
                        capabilities.push(cap_name.to_string());
                    }
                }
                if capabilities.iter().any(|c| c == capability) {
                    let host = info.get_addresses().iter().next().map_or_else(
                        || info.get_hostname().trim_end_matches('.').to_string(),
                        ToString::to_string,
                    );
                    let port = info.get_port();
                    let url = format!("http://{host}:{port}");
                    discovered.push(PrimalEndpoint {
                        service_id: format!("{capability}-mdns"),
                        url,
                        capabilities: vec![capability.to_string()],
                        healthy: true,
                        last_check: std::time::SystemTime::now(),
                    });
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    let _ = mdns.stop_browse(service_type);
    let _ = mdns.shutdown();

    if discovered.is_empty() {
        None
    } else {
        Some(discovered)
    }
}

/// Stub when `mdns` feature is disabled.
#[must_use]
#[cfg(not(feature = "mdns"))]
pub fn try_discover_via_mdns(_capability: &str) -> Option<Vec<PrimalEndpoint>> {
    None
}

/// Probe Kubernetes DNS for a service with the given capability.
///
/// Checks `KUBERNETES_SERVICE_HOST` to detect K8s environment, then attempts
/// DNS resolution of `{capability}.{namespace}.svc.cluster.local`. Uses namespace
/// from `POD_NAMESPACE` or `default`. Returns `None` when not in Kubernetes or
/// when the capability-based service name does not resolve.
#[must_use]
pub fn try_discover_via_kubernetes(capability: &str) -> Option<Vec<PrimalEndpoint>> {
    debug!("Probing Kubernetes DNS for capability '{}'", capability);

    let _k8s_host = std::env::var(socket_env::KUBERNETES_SERVICE_HOST).ok()?;
    let namespace =
        std::env::var(socket_env::POD_NAMESPACE).unwrap_or_else(|_| "default".to_string());
    let service_name = capability.replace('_', "-");
    let dns_name = format!("{service_name}.{namespace}.svc.cluster.local");
    let port = discovery_http_port();

    let _ = (dns_name.as_str(), port).to_socket_addrs().ok()?.next()?;

    let url = format!("{HTTP_PROTOCOL}{dns_name}:{port}");
    Some(vec![PrimalEndpoint {
        service_id: format!("{capability}-k8s"),
        url,
        capabilities: vec![capability.to_string()],
        healthy: true,
        last_check: std::time::SystemTime::now(),
    }])
}

/// Probe Docker Compose DNS for a service with the given capability.
///
/// Checks for `COMPOSE_PROJECT_NAME` or presence of `docker-compose.yml` / `compose.yaml`
/// in the current directory, then attempts DNS resolution of `{capability}:{port}`
/// (Docker's embedded DNS resolves Compose service names). Port is configurable via
/// `TOADSTOOL_DISCOVERY_HTTP_PORT` (default: 8080). Returns `None` when
/// not in a Compose environment or when the service name does not resolve.
#[must_use]
pub fn try_discover_via_docker_compose(capability: &str) -> Option<Vec<PrimalEndpoint>> {
    debug!("Probing Docker Compose for capability '{}'", capability);

    let in_compose = std::env::var(socket_env::COMPOSE_PROJECT_NAME).is_ok()
        || std::path::Path::new("docker-compose.yml").exists()
        || std::path::Path::new("compose.yaml").exists()
        || std::path::Path::new("compose.yml").exists();

    if !in_compose {
        return None;
    }

    let service_name = capability.replace('_', "-");
    let port = discovery_http_port();

    let _ = (service_name.as_str(), port)
        .to_socket_addrs()
        .ok()?
        .next()?;

    let url = format!("{HTTP_PROTOCOL}{service_name}:{port}");
    Some(vec![PrimalEndpoint {
        service_id: format!("{capability}-compose"),
        url,
        capabilities: vec![capability.to_string()],
        healthy: true,
        last_check: std::time::SystemTime::now(),
    }])
}

/// Probe a well-known registry (Consul, etcd, or `TOADSTOOL_REGISTRY_ENDPOINT`) for
/// a service with the given capability.
///
/// Checks `TOADSTOOL_REGISTRY_ENDPOINT`, `CONSUL_HTTP_ADDR`, or `ETCD_ENDPOINTS`.
/// For HTTP registries, performs a blocking GET request and parses the JSON response
/// to find services advertising the capability. Returns `None` when no registry is
/// configured, when the registry is unreachable, or when no matching service is found.
#[must_use]
pub fn try_discover_via_registry(capability: &str) -> Option<Vec<PrimalEndpoint>> {
    const TCP_CONNECT_TIMEOUT_SECS: u64 = 3;
    const TCP_CONNECT_TIMEOUT_TEST_MS: u64 = 100;

    #[derive(serde::Deserialize)]
    struct RegistryServices {
        #[serde(default)]
        services: Vec<RegistryService>,
    }
    #[derive(serde::Deserialize)]
    struct RegistryService {
        #[serde(default)]
        capabilities: Vec<String>,
        #[serde(default)]
        endpoints: Vec<String>,
        #[serde(default)]
        name: String,
    }

    debug!("Probing registry for capability '{}'", capability);

    let endpoint = std::env::var(socket_env::TOADSTOOL_REGISTRY_ENDPOINT)
        .ok()
        .or_else(|| {
            std::env::var(socket_env::CONSUL_HTTP_ADDR)
                .ok()
                .map(|a| format!("http://{a}"))
        })
        .or_else(|| {
            std::env::var(socket_env::ETCD_ENDPOINTS)
                .ok()
                .and_then(|s| s.split(',').next().map(|e| format!("http://{}", e.trim())))
        })?;

    if !endpoint.starts_with("http://") {
        return None;
    }

    let url = endpoint.trim_start_matches("http://").trim_end_matches('/');
    let (host_port, path) = url.split_once('/').unwrap_or((url, ""));
    let path = if path.is_empty() || !path.contains("services") {
        "services"
    } else {
        path
    };

    let (host, port) = host_port
        .rsplit_once(':')
        .and_then(|(h, p)| p.parse::<u16>().ok().map(|port| (h, port)))
        .unwrap_or((host_port, 80));

    let addrs: Vec<_> = (host, port).to_socket_addrs().ok()?.collect();
    let addr = addrs.first()?;
    let connect_timeout = if cfg!(test) {
        Duration::from_millis(TCP_CONNECT_TIMEOUT_TEST_MS)
    } else {
        Duration::from_secs(TCP_CONNECT_TIMEOUT_SECS)
    };
    let mut stream = TcpStream::connect_timeout(addr, connect_timeout).ok()?;

    let request = format!("GET /{path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    stream.flush().ok()?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response).ok()?;

    let body = response
        .as_slice()
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map_or(&response[..], |pos| &response[pos + 4..]);

    let config: RegistryServices = serde_json::from_slice(body).ok()?;
    let now = std::time::SystemTime::now();

    let endpoints: Vec<PrimalEndpoint> = config
        .services
        .into_iter()
        .filter(|s| {
            s.capabilities
                .iter()
                .any(|c| c.replace('_', "-") == capability.replace('_', "-"))
        })
        .flat_map(|s| {
            let service_name = s.name.clone();
            s.endpoints.into_iter().filter_map(move |url| {
                if url.starts_with("http://") || url.starts_with("https://") {
                    Some(PrimalEndpoint {
                        service_id: format!("{service_name}-registry"),
                        url,
                        capabilities: vec![capability.to_string()],
                        healthy: true,
                        last_check: now,
                    })
                } else {
                    None
                }
            })
        })
        .collect();

    if endpoints.is_empty() {
        None
    } else {
        Some(endpoints)
    }
}

/// Built-in default endpoints for known capabilities.
/// Returns `None` for all capabilities — discovered via capability resolution at runtime.
/// Caller sets `TOADSTOOL_{CAPABILITY}_ENDPOINT` or discovers via mDNS/registry.
pub const fn builtin_default_endpoint(_capability: &str) -> Option<String> {
    None
}

/// Probe well-known filesystem locations for a capability (development mode).
///
/// Checks `TOADSTOOL_SERVICE_DIR` (or `$XDG_RUNTIME_DIR/biomeos`) for a
/// subdirectory named after the capability. This allows zero-config local
/// development when primals are co-located on the same filesystem.
#[must_use]
pub fn try_discover_via_filesystem(capability: &str) -> Option<Vec<PrimalEndpoint>> {
    debug!("Probing filesystem for capability '{}'", capability);

    let base = std::env::var(socket_env::TOADSTOOL_SERVICE_DIR)
        .ok()
        .or_else(|| {
            std::env::var(socket_env::XDG_RUNTIME_DIR)
                .ok()
                .map(|xdg| format!("{xdg}/biomeos"))
        })?;

    let full_path = std::path::Path::new(&base).join(capability);
    if full_path.exists() {
        tracing::info!(
            "✅ Found {} at filesystem path: {:?}",
            capability,
            full_path
        );
        Some(vec![PrimalEndpoint {
            service_id: format!("{capability}-fs"),
            url: format!("file://{}", full_path.display()),
            capabilities: vec![capability.to_string()],
            healthy: true,
            last_check: std::time::SystemTime::now(),
        }])
    } else {
        None
    }
}
