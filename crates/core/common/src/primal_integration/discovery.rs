//! Capability-based service discovery.
//!
//! Discovers ecoPrimal services via env vars, mDNS, Kubernetes, Docker Compose, and registries.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use tracing::{debug, info, warn};

use crate::constants::network::{http_url, DEFAULT_HTTP_PORT};

use super::{DiscoveryError, DiscoveryResult, PrimalEndpoint};

/// Port for capability-based service discovery (K8s, Docker Compose).
/// Overridable via `TOADSTOOL_DISCOVERY_HTTP_PORT` environment variable.
fn discovery_http_port() -> u16 {
    std::env::var("TOADSTOOL_DISCOVERY_HTTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_HTTP_PORT)
}

/// Discover any service by capability at runtime.
///
/// Primals only have self-knowledge and discover other primals at runtime.
/// This is the core discovery function — discover by capability (e.g., `security`,
/// `storage`), not by primal name. Use [`super::capabilities`] constants for standard
/// capability identifiers.
///
/// Tries multiple methods:
/// 1. Environment variables (TOADSTOOL_{CAPABILITY}_ENDPOINT)
/// 2. mDNS/DNS-SD discovery
/// 3. Kubernetes service discovery
/// 4. Docker Compose service names
/// 5. Runtime registry (consul, etcd)
///
/// For socket-based discovery returning `Option<String>`, use
/// [`super::discover_service_socket_by_capability`].
///
/// # Errors
///
/// Returns error if no service with the requested capability can be discovered
pub fn discover_service_by_capability(capability: &str) -> DiscoveryResult {
    info!("🔍 Discovering service with capability: {}", capability);

    // Try environment variable first (highest priority)
    let env_var = format!("TOADSTOOL_{}_ENDPOINT", capability.to_uppercase());
    if let Ok(endpoint) = std::env::var(&env_var) {
        debug!("✅ Found {} via {}: {}", capability, env_var, endpoint);
        return Ok(vec![PrimalEndpoint {
            service_id: format!("{capability}-env"),
            url: endpoint,
            capabilities: vec![capability.to_string()],
            healthy: true, // Assume healthy, will verify on first use
            last_check: std::time::SystemTime::now(),
        }]);
    }

    // Try generic TOADSTOOL_SERVICE_{NAME}_URL pattern
    let generic_var = format!("TOADSTOOL_SERVICE_{}_URL", capability.to_uppercase());
    if let Ok(endpoint) = std::env::var(&generic_var) {
        debug!("✅ Found {} via {}: {}", capability, generic_var, endpoint);
        return Ok(vec![PrimalEndpoint {
            service_id: format!("{capability}-service"),
            url: endpoint,
            capabilities: vec![capability.to_string()],
            healthy: true,
            last_check: std::time::SystemTime::now(),
        }]);
    }

    // Try capability-based discovery methods (mDNS, Kubernetes, Docker Compose, Registry)
    if let Some(endpoints) = try_discover_via_mdns(capability) {
        debug!(
            "✅ Found {} via mDNS: {} endpoint(s)",
            capability,
            endpoints.len()
        );
        return Ok(endpoints);
    }
    if let Some(endpoints) = try_discover_via_kubernetes(capability) {
        debug!(
            "✅ Found {} via Kubernetes DNS: {} endpoint(s)",
            capability,
            endpoints.len()
        );
        return Ok(endpoints);
    }
    if let Some(endpoints) = try_discover_via_docker_compose(capability) {
        debug!(
            "✅ Found {} via Docker Compose: {} endpoint(s)",
            capability,
            endpoints.len()
        );
        return Ok(endpoints);
    }
    if let Some(endpoints) = try_discover_via_registry(capability) {
        debug!(
            "✅ Found {} via registry: {} endpoint(s)",
            capability,
            endpoints.len()
        );
        return Ok(endpoints);
    }

    // Try filesystem-based discovery (development mode)
    if let Some(endpoints) = try_discover_via_filesystem(capability) {
        debug!(
            "✅ Found {} via filesystem: {} endpoint(s)",
            capability,
            endpoints.len()
        );
        return Ok(endpoints);
    }

    // Fallback to configurable defaults for known capabilities
    if let Some(builtin) = builtin_default_endpoint(capability) {
        let default_var = format!(
            "TOADSTOOL_{}_DEFAULT_ENDPOINT",
            capability.to_uppercase().replace('-', "_")
        );
        let endpoint = std::env::var(&default_var).unwrap_or(builtin);

        warn!(
            "⚠️  No {} service found via discovery, using default: {} (override via {})",
            capability, endpoint, default_var
        );
        return Ok(vec![PrimalEndpoint {
            service_id: format!("{capability}-default"),
            url: endpoint,
            capabilities: vec![capability.to_string()],
            healthy: true,
            last_check: std::time::SystemTime::now(),
        }]);
    }

    warn!("⚠️  No {} service found via discovery", capability);
    Err(DiscoveryError::NoServiceFound {
        capability: capability.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Primal-specific discovery (thin wrappers)
// ---------------------------------------------------------------------------

/// Discover bearDog encryption service at runtime
///
/// Tries multiple discovery methods in order:
/// 1. `BEARDOG_ENDPOINT` environment variable
/// 2. mDNS (_encryption._tcp.local.)
/// 3. Kubernetes DNS (beardog.default.svc.cluster.local)
/// 4. Docker Compose (beardog:6060)
/// 5. Runtime registry (consul, etcd)
///
/// # Errors
///
/// Returns error if no bearDog service can be discovered
pub fn discover_encryption_service() -> DiscoveryResult {
    discover_service_by_capability("encryption")
}

/// Discover nestGate storage service at runtime
///
/// Tries multiple discovery methods in order:
/// 1. `NESTGATE_ENDPOINT` environment variable
/// 2. mDNS (_storage._tcp.local.)
/// 3. Kubernetes DNS (nestgate.default.svc.cluster.local)
/// 4. Docker Compose (nestgate:8080)
/// 5. Runtime registry (consul, etcd)
///
/// # Errors
///
/// Returns error if no nestGate service can be discovered
pub fn discover_storage_service() -> DiscoveryResult {
    discover_service_by_capability("storage")
}

/// Discover songBird coordination service at runtime
///
/// # Errors
///
/// Returns error if no songBird service can be discovered
pub fn discover_coordination_service() -> DiscoveryResult {
    discover_service_by_capability("coordination")
}

/// Discover squirrel MCP platform at runtime
///
/// # Errors
///
/// Returns error if no squirrel service can be discovered
pub fn discover_mcp_service() -> DiscoveryResult {
    discover_service_by_capability("mcp")
}

// ---------------------------------------------------------------------------
// External system discovery
// ---------------------------------------------------------------------------

/// Discover Redis cache service
///
/// # Errors
///
/// Returns error if no cache service can be discovered
pub fn discover_cache_service() -> DiscoveryResult {
    discover_service_by_capability("cache")
}

/// Discover `PostgreSQL` database service
///
/// # Errors
///
/// Returns error if no database service can be discovered
pub fn discover_database_service() -> DiscoveryResult {
    discover_service_by_capability("database")
}

/// Discover S3-compatible object storage
///
/// # Errors
///
/// Returns error if no object storage service can be discovered
pub fn discover_object_storage() -> DiscoveryResult {
    discover_service_by_capability("object-storage")
}

// ---------------------------------------------------------------------------
// Discovery backends (mDNS, K8s, Docker Compose, Registry)
// ---------------------------------------------------------------------------

/// Probe mDNS/DNS-SD for services advertising the given capability.
///
/// Browses for `_toadstool._tcp.local.` and filters by capability in TXT records
/// (`cap_{name}`). Uses `mdns-sd` crate. Returns `None` when mDNS daemon is
/// unavailable or no matching services are found within the probe timeout.
#[must_use]
fn try_discover_via_mdns(capability: &str) -> Option<Vec<PrimalEndpoint>> {
    debug!("Probing mDNS for capability '{}'", capability);

    let mdns = mdns_sd::ServiceDaemon::new().ok()?;
    let service_type = "_toadstool._tcp.local.";
    let receiver = mdns.browse(service_type).ok()?;

    let timeout = Duration::from_secs(2);
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
                    if let Some(cap_name) = key.strip_prefix("cap_") {
                        if !cap_name.ends_with("_features") {
                            capabilities.push(cap_name.to_string());
                        }
                    }
                }
                if capabilities.iter().any(|c| c == capability) {
                    let host = info
                        .get_addresses()
                        .iter()
                        .next()
                        .map(std::string::ToString::to_string)
                        .unwrap_or_else(|| info.get_hostname().trim_end_matches('.').to_string());
                    let port = info.get_port();
                    let url = format!("http://{host}:{port}");
                    discovered.push(PrimalEndpoint {
                        service_id: format!("{}-mdns", capability),
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

/// Probe Kubernetes DNS for a service with the given capability.
///
/// Checks `KUBERNETES_SERVICE_HOST` to detect K8s environment, then attempts
/// DNS resolution of `{capability}.{namespace}.svc.cluster.local`. Uses namespace
/// from `POD_NAMESPACE` or `default`. Returns `None` when not in Kubernetes or
/// when the capability-based service name does not resolve.
#[must_use]
fn try_discover_via_kubernetes(capability: &str) -> Option<Vec<PrimalEndpoint>> {
    debug!("Probing Kubernetes DNS for capability '{}'", capability);

    let _k8s_host = std::env::var("KUBERNETES_SERVICE_HOST").ok()?;
    let namespace = std::env::var("POD_NAMESPACE").unwrap_or_else(|_| "default".to_string());
    let service_name = capability.replace('_', "-");
    let dns_name = format!("{service_name}.{namespace}.svc.cluster.local");
    let port = discovery_http_port();

    let _ = (dns_name.as_str(), port).to_socket_addrs().ok()?.next()?;

    let url = http_url(&dns_name, port);
    Some(vec![PrimalEndpoint {
        service_id: format!("{}-k8s", capability),
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
fn try_discover_via_docker_compose(capability: &str) -> Option<Vec<PrimalEndpoint>> {
    debug!("Probing Docker Compose for capability '{}'", capability);

    let in_compose = std::env::var("COMPOSE_PROJECT_NAME").is_ok()
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

    let url = http_url(&service_name, port);
    Some(vec![PrimalEndpoint {
        service_id: format!("{}-compose", capability),
        url,
        capabilities: vec![capability.to_string()],
        healthy: true,
        last_check: std::time::SystemTime::now(),
    }])
}

/// Probe a well-known registry (Consul, etcd, or TOADSTOOL_REGISTRY_ENDPOINT) for
/// a service with the given capability.
///
/// Checks `TOADSTOOL_REGISTRY_ENDPOINT`, `CONSUL_HTTP_ADDR`, or `ETCD_ENDPOINTS`.
/// For HTTP registries, performs a blocking GET request and parses the JSON response
/// to find services advertising the capability. Returns `None` when no registry is
/// configured, when the registry is unreachable, or when no matching service is found.
#[must_use]
fn try_discover_via_registry(capability: &str) -> Option<Vec<PrimalEndpoint>> {
    debug!("Probing registry for capability '{}'", capability);

    let endpoint = std::env::var("TOADSTOOL_REGISTRY_ENDPOINT")
        .ok()
        .or_else(|| {
            std::env::var("CONSUL_HTTP_ADDR")
                .ok()
                .map(|a| format!("http://{a}"))
        })
        .or_else(|| {
            std::env::var("ETCD_ENDPOINTS")
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
    let mut stream = TcpStream::connect_timeout(addr, Duration::from_secs(3)).ok()?;

    let request = format!(
        "GET /{} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, host_port
    );
    stream.write_all(request.as_bytes()).ok()?;
    stream.flush().ok()?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response).ok()?;

    let body = response
        .as_slice()
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|pos| &response[pos + 4..])
        .unwrap_or(&response);

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
                        service_id: format!("{}-registry", service_name),
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
fn builtin_default_endpoint(_capability: &str) -> Option<String> {
    None
}

/// Probe well-known filesystem locations for a capability (development mode).
///
/// Checks `TOADSTOOL_SERVICE_DIR` (or `$XDG_RUNTIME_DIR/biomeos`) for a
/// subdirectory named after the capability. This allows zero-config local
/// development when primals are co-located on the same filesystem.
#[must_use]
fn try_discover_via_filesystem(capability: &str) -> Option<Vec<PrimalEndpoint>> {
    debug!("Probing filesystem for capability '{}'", capability);

    let base = std::env::var("TOADSTOOL_SERVICE_DIR").ok().or_else(|| {
        std::env::var("XDG_RUNTIME_DIR")
            .ok()
            .map(|xdg| format!("{xdg}/biomeos"))
    })?;

    let full_path = std::path::Path::new(&base).join(capability);
    if full_path.exists() {
        info!(
            "✅ Found {} at filesystem path: {:?}",
            capability, full_path
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
