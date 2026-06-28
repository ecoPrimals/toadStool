// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(
    missing_docs,
    reason = "each pub const is the env var name string; module docs describe usage"
)]
//! Environment variable names for runtime socket discovery (`SocketPathEnv::from_env`).
//!
//! Prefer capability-prefixed vars via [`super::CapabilityDomain::biomeos_socket_env`] /
//! [`super::CapabilityDomain::toadstool_socket_env`] (`BIOMEOS_*_SOCKET`, `TOADSTOOL_*_SOCKET`).
//! Legacy `LEGACY_*` names are identity-based fallbacks for older deployments.

// POSIX / XDG Base Directory Specification.
pub const XDG_RUNTIME_DIR: &str = "XDG_RUNTIME_DIR";
pub const XDG_DATA_HOME: &str = "XDG_DATA_HOME";
pub const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";
pub const XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";
pub const USER: &str = "USER";
/// Standard home directory path (shell / POSIX).
pub const HOME: &str = "HOME";
pub const HOSTNAME: &str = "HOSTNAME";
pub const TMPDIR: &str = "TMPDIR";
pub const TMP: &str = "TMP";
pub const TEMP: &str = "TEMP";

// Windows equivalents for cross-platform path resolution.
pub const USERPROFILE: &str = "USERPROFILE";
pub const USERNAME: &str = "USERNAME";
pub const APPDATA: &str = "APPDATA";

pub const TOADSTOOL_FAMILY_ID: &str = "TOADSTOOL_FAMILY_ID";
pub const TOADSTOOL_FAMILY: &str = "TOADSTOOL_FAMILY";
pub const BIOMEOS_FAMILY_ID: &str = "BIOMEOS_FAMILY_ID";
/// Node identity for multi-instance deployments (orchestration / logging).
pub const TOADSTOOL_NODE_ID: &str = "TOADSTOOL_NODE_ID";

/// Configurable JWT issuer for token validation (capability-based, replaces hardcoded identity).
pub const TOADSTOOL_AUTH_ISSUER: &str = "TOADSTOOL_AUTH_ISSUER";

/// Preferred: explicit crypto / security socket path (capability-based).
pub const BIOMEOS_CRYPTO_SOCKET: &str = "BIOMEOS_CRYPTO_SOCKET";
/// Preferred: explicit coordination socket path (capability-based).
pub const BIOMEOS_COORDINATION_SOCKET: &str = "BIOMEOS_COORDINATION_SOCKET";
/// Preferred: explicit storage socket path (capability-based).
pub const BIOMEOS_STORAGE_SOCKET: &str = "BIOMEOS_STORAGE_SOCKET";
/// Preferred: explicit routing / MCP-style socket path (capability-based).
pub const BIOMEOS_ROUTING_SOCKET: &str = "BIOMEOS_ROUTING_SOCKET";

pub const TOADSTOOL_SECURITY_SOCKET: &str = "TOADSTOOL_SECURITY_SOCKET";
pub const TOADSTOOL_COORDINATION_SOCKET: &str = "TOADSTOOL_COORDINATION_SOCKET";
pub const TOADSTOOL_STORAGE_SOCKET: &str = "TOADSTOOL_STORAGE_SOCKET";
pub const TOADSTOOL_INTELLIGENCE_SOCKET: &str = "TOADSTOOL_INTELLIGENCE_SOCKET";

/// **Deprecated** (identity-based). Prefer `BIOMEOS_CRYPTO_SOCKET` or capability discovery
/// (`interned_strings::capabilities::CRYPTO`).
#[deprecated(
    since = "0.4.0",
    note = "use BIOMEOS_CRYPTO_SOCKET or capability discovery"
)]
pub const LEGACY_BEARDOG_SOCKET_ENV: &str = "BEARDOG_SOCKET";
/// **Deprecated** (identity-based). Prefer `BIOMEOS_COORDINATION_SOCKET` or
/// `interned_strings::capabilities::COORDINATION`.
#[deprecated(
    since = "0.4.0",
    note = "use BIOMEOS_COORDINATION_SOCKET or capability discovery"
)]
pub const LEGACY_SONGBIRD_SOCKET_ENV: &str = "SONGBIRD_SOCKET";
/// **Deprecated** (identity-based). Prefer `BIOMEOS_STORAGE_SOCKET` or
/// `interned_strings::capabilities::STORAGE`.
#[deprecated(
    since = "0.4.0",
    note = "use BIOMEOS_STORAGE_SOCKET or capability discovery"
)]
pub const LEGACY_NESTGATE_SOCKET_ENV: &str = "NESTGATE_SOCKET";
/// **Deprecated** (identity-based). Prefer `BIOMEOS_ROUTING_SOCKET` or
/// `interned_strings::capabilities::ROUTING`.
#[deprecated(
    since = "0.4.0",
    note = "use BIOMEOS_ROUTING_SOCKET or capability discovery"
)]
pub const LEGACY_SQUIRREL_SOCKET_ENV: &str = "SQUIRREL_SOCKET";

pub const TOADSTOOL_SOCKET: &str = "TOADSTOOL_SOCKET";
pub const TOADSTOOL_TARPC_SOCKET: &str = "TOADSTOOL_TARPC_SOCKET";
pub const BIOMEOS_SOCKET_PATH: &str = "BIOMEOS_SOCKET_PATH";
pub const NUCLEUS_SOCKET: &str = "NUCLEUS_SOCKET";
pub const BIOMEOS_INSECURE: &str = "BIOMEOS_INSECURE";

// Connection hint env vars (URL/endpoint discovery chain).
// Capability-based vars are preferred; legacy primal-identity vars are fallbacks.

pub const TOADSTOOL_COORDINATION_ENDPOINT: &str = "TOADSTOOL_COORDINATION_ENDPOINT";
pub const COORDINATION_URL: &str = "COORDINATION_URL";
pub const COORDINATION_ENDPOINT: &str = "COORDINATION_ENDPOINT";
/// **Deprecated** (identity-based). Prefer `COORDINATION_URL`.
#[deprecated(since = "0.4.0", note = "use COORDINATION_URL")]
pub const LEGACY_SONGBIRD_URL: &str = "SONGBIRD_URL";
/// **Deprecated** (identity-based). Prefer `COORDINATION_ENDPOINT`.
#[deprecated(since = "0.4.0", note = "use COORDINATION_ENDPOINT")]
pub const LEGACY_SONGBIRD_ENDPOINT: &str = "SONGBIRD_ENDPOINT";

pub const TOADSTOOL_SECURITY_ENDPOINT: &str = "TOADSTOOL_SECURITY_ENDPOINT";
pub const SECURITY_URL: &str = "SECURITY_URL";
pub const SECURITY_ENDPOINT: &str = "SECURITY_ENDPOINT";
/// **Deprecated** (identity-based). Prefer `SECURITY_URL`.
#[deprecated(since = "0.4.0", note = "use SECURITY_URL")]
pub const LEGACY_BEARDOG_URL: &str = "BEARDOG_URL";
/// **Deprecated** (identity-based). Prefer `SECURITY_ENDPOINT`.
#[deprecated(since = "0.4.0", note = "use SECURITY_ENDPOINT")]
pub const LEGACY_BEARDOG_ENDPOINT: &str = "BEARDOG_ENDPOINT";

pub const TOADSTOOL_STORAGE_ENDPOINT: &str = "TOADSTOOL_STORAGE_ENDPOINT";
pub const STORAGE_URL: &str = "STORAGE_URL";
pub const STORAGE_ENDPOINT: &str = "STORAGE_ENDPOINT";
/// **Deprecated** (identity-based). Prefer `STORAGE_URL`.
#[deprecated(since = "0.4.0", note = "use STORAGE_URL")]
pub const LEGACY_NESTGATE_URL: &str = "NESTGATE_URL";
/// **Deprecated** (identity-based). Prefer `STORAGE_ENDPOINT`.
#[deprecated(since = "0.4.0", note = "use STORAGE_ENDPOINT")]
pub const LEGACY_NESTGATE_ENDPOINT: &str = "NESTGATE_ENDPOINT";

pub const TOADSTOOL_AI_ENDPOINT: &str = "TOADSTOOL_AI_ENDPOINT";
pub const TOADSTOOL_INTELLIGENCE_ENDPOINT: &str = "TOADSTOOL_INTELLIGENCE_ENDPOINT";

/// **Deprecated** (identity-based). Prefer `TOADSTOOL_COORDINATION_ENDPOINT`.
#[deprecated(since = "0.4.0", note = "use TOADSTOOL_COORDINATION_ENDPOINT")]
pub const TOADSTOOL_SONGBIRD_ENDPOINT: &str = "TOADSTOOL_SONGBIRD_ENDPOINT";
/// **Deprecated** (identity-based). Prefer `TOADSTOOL_SECURITY_ENDPOINT`.
#[deprecated(since = "0.4.0", note = "use TOADSTOOL_SECURITY_ENDPOINT")]
pub const TOADSTOOL_BEARDOG_ENDPOINT: &str = "TOADSTOOL_BEARDOG_ENDPOINT";
/// **Deprecated** (identity-based). Prefer `TOADSTOOL_STORAGE_ENDPOINT`.
#[deprecated(since = "0.4.0", note = "use TOADSTOOL_STORAGE_ENDPOINT")]
pub const TOADSTOOL_NESTGATE_ENDPOINT: &str = "TOADSTOOL_NESTGATE_ENDPOINT";
/// **Deprecated** (identity-based). Prefer `TOADSTOOL_AI_ENDPOINT`.
#[deprecated(since = "0.4.0", note = "use TOADSTOOL_AI_ENDPOINT")]
pub const TOADSTOOL_SQUIRREL_ENDPOINT: &str = "TOADSTOOL_SQUIRREL_ENDPOINT";
pub const AI_PROCESSING_ENDPOINT: &str = "AI_PROCESSING_ENDPOINT";
pub const TOADSTOOL_AI_PROCESSING_ENDPOINT: &str = "TOADSTOOL_AI_PROCESSING_ENDPOINT";
pub const INTELLIGENCE_URL: &str = "INTELLIGENCE_URL";
pub const INTELLIGENCE_ENDPOINT: &str = "INTELLIGENCE_ENDPOINT";
pub const AI_PROCESSING_URL: &str = "AI_PROCESSING_URL";
/// **Deprecated** (identity-based). Prefer `INTELLIGENCE_URL`.
#[deprecated(since = "0.4.0", note = "use INTELLIGENCE_URL")]
pub const LEGACY_SQUIRREL_URL: &str = "SQUIRREL_URL";
/// **Deprecated** (identity-based). Prefer `INTELLIGENCE_ENDPOINT`.
#[deprecated(since = "0.4.0", note = "use INTELLIGENCE_ENDPOINT")]
pub const LEGACY_SQUIRREL_ENDPOINT: &str = "SQUIRREL_ENDPOINT";

// HTTP URL overrides (parallel naming to `*_ENDPOINT`; used by discovery fallbacks and demos).
pub const TOADSTOOL_COORDINATION_URL: &str = "TOADSTOOL_COORDINATION_URL";
pub const TOADSTOOL_SECURITY_URL: &str = "TOADSTOOL_SECURITY_URL";
pub const TOADSTOOL_STORAGE_URL: &str = "TOADSTOOL_STORAGE_URL";

// Capability service ports (HTTP fallback / local bind hints).
pub const TOADSTOOL_COORDINATION_PORT: &str = "TOADSTOOL_COORDINATION_PORT";
pub const TOADSTOOL_SECURITY_PORT: &str = "TOADSTOOL_SECURITY_PORT";
pub const TOADSTOOL_STORAGE_PORT: &str = "TOADSTOOL_STORAGE_PORT";
/// Platform / intelligence capability port override.
pub const TOADSTOOL_INTELLIGENCE_PORT: &str = "TOADSTOOL_INTELLIGENCE_PORT";
/// Platform capability port override (alias for intelligence routing).
pub const TOADSTOOL_PLATFORM_PORT: &str = "TOADSTOOL_PLATFORM_PORT";
/// Generic coordination port (non-prefixed fallback).
pub const COORDINATION_PORT: &str = "COORDINATION_PORT";
/// Generic security port (non-prefixed fallback).
pub const SECURITY_PORT: &str = "SECURITY_PORT";
/// Generic storage port (non-prefixed fallback).
pub const STORAGE_PORT: &str = "STORAGE_PORT";
/// Generic intelligence port (non-prefixed fallback).
pub const INTELLIGENCE_PORT: &str = "INTELLIGENCE_PORT";
/// Generic platform port (non-prefixed fallback).
pub const PLATFORM_PORT: &str = "PLATFORM_PORT";
/// **Deprecated** (identity-based). Prefer `TOADSTOOL_COORDINATION_PORT`.
#[deprecated(since = "0.4.0", note = "use TOADSTOOL_COORDINATION_PORT")]
pub const TOADSTOOL_SONGBIRD_PORT: &str = "TOADSTOOL_SONGBIRD_PORT";
/// **Deprecated** (identity-based). Prefer `TOADSTOOL_COORDINATION_PORT` or `COORDINATION_PORT`.
#[deprecated(since = "0.4.0", note = "use TOADSTOOL_COORDINATION_PORT")]
pub const SONGBIRD_PORT: &str = "SONGBIRD_PORT";
/// **Deprecated** (identity-based). Prefer `TOADSTOOL_SECURITY_PORT` or `SECURITY_PORT`.
#[deprecated(since = "0.4.0", note = "use TOADSTOOL_SECURITY_PORT")]
pub const BEARDOG_PORT: &str = "BEARDOG_PORT";
/// **Deprecated** (identity-based). Prefer `TOADSTOOL_STORAGE_PORT` or `STORAGE_PORT`.
#[deprecated(since = "0.4.0", note = "use TOADSTOOL_STORAGE_PORT")]
pub const NESTGATE_PORT: &str = "NESTGATE_PORT";
/// **Deprecated** (identity-based). Prefer `TOADSTOOL_INTELLIGENCE_PORT` or `INTELLIGENCE_PORT`.
#[deprecated(since = "0.4.0", note = "use TOADSTOOL_INTELLIGENCE_PORT")]
pub const SQUIRREL_PORT: &str = "SQUIRREL_PORT";

/// NUCLEUS discovery socket (Songbird). Highest-precedence path for `coordination`
/// and `discovery` capability resolution — set by `composition_nucleus.sh`.
pub const DISCOVERY_SOCKET: &str = "DISCOVERY_SOCKET";
/// Socket directory for all primals in a NUCLEUS composition.
pub const BIOMEOS_SOCKET_DIR: &str = "BIOMEOS_SOCKET_DIR";

// Discovery bootstrap and bind hints.
pub const TOADSTOOL_MDNS_ENABLE: &str = "TOADSTOOL_MDNS_ENABLE";
pub const TOADSTOOL_MDNS_REQUIRE: &str = "TOADSTOOL_MDNS_REQUIRE";
pub const TOADSTOOL_DISCOVERY_FALLBACKS: &str = "TOADSTOOL_DISCOVERY_FALLBACKS";
pub const TOADSTOOL_ENV: &str = "TOADSTOOL_ENV";
pub const TOADSTOOL_BIND_HOST: &str = "TOADSTOOL_BIND_HOST";
pub const BIND_HOST: &str = "BIND_HOST";
pub const TOADSTOOL_BIND_ADDRESS: &str = "TOADSTOOL_BIND_ADDRESS";
/// IP address override for service listen sockets (default loopback).
pub const TOADSTOOL_LISTEN_ADDRESS: &str = "TOADSTOOL_LISTEN_ADDRESS";
/// Primary ToadStool service port override.
pub const TOADSTOOL_SERVICE_PORT: &str = "TOADSTOOL_SERVICE_PORT";
/// HTTP/API port override (parallel to [`TOADSTOOL_SERVICE_PORT`]).
pub const TOADSTOOL_API_PORT: &str = "TOADSTOOL_API_PORT";
/// Metrics / Prometheus scrape port override.
pub const TOADSTOOL_METRICS_PORT: &str = "TOADSTOOL_METRICS_PORT";
/// Health check HTTP port override.
pub const TOADSTOOL_HEALTH_PORT: &str = "TOADSTOOL_HEALTH_PORT";
/// Comma-separated explicit discovery endpoint URLs.
pub const TOADSTOOL_DISCOVERY_ENDPOINTS: &str = "TOADSTOOL_DISCOVERY_ENDPOINTS";
/// Enable mDNS for local network discovery (`NetworkConfig::from_env`).
pub const TOADSTOOL_ENABLE_MDNS: &str = "TOADSTOOL_ENABLE_MDNS";
/// Network bind mode: `localhost`, `all`, or `specific`.
pub const TOADSTOOL_BIND_MODE: &str = "TOADSTOOL_BIND_MODE";
pub const TOADSTOOL_TCP_BIND_ADDRESS: &str = "TOADSTOOL_TCP_BIND_ADDRESS";

/// Launcher-injected transport endpoint (JSON `TransportEndpoint`).
///
/// When set, the primal uses this transport instead of self-binding.
/// Follows the sourDough canonical transport standard.
/// Format: `{"transport":"uds","path":"/run/user/1000/biomeos/compute.sock"}`
pub const TRANSPORT_ENDPOINT: &str = "TRANSPORT_ENDPOINT";
/// Idle timeout (seconds) for Pure JSON-RPC TCP connections.
pub const TOADSTOOL_TCP_IDLE_TIMEOUT_SECS: &str = "TOADSTOOL_TCP_IDLE_TIMEOUT_SECS";
pub const TOADSTOOL_STANDALONE: &str = "TOADSTOOL_STANDALONE";
/// Override total BTSP handshake timeout (seconds). Default: 5.
pub const BTSP_HANDSHAKE_TIMEOUT_SECS: &str = "BTSP_HANDSHAKE_TIMEOUT_SECS";
/// Override per-RPC timeout for BearDog calls during BTSP (seconds). Default: 2.
pub const BTSP_RPC_TIMEOUT_SECS: &str = "BTSP_RPC_TIMEOUT_SECS";

/// Override Unix socket file mode (octal, e.g. "0660"). Default: 0600.
/// Set to "0660" for group-accessible sockets when biomeOS/primalSpring
/// runs as the same group but different user.
pub const TOADSTOOL_SOCKET_MODE: &str = "TOADSTOOL_SOCKET_MODE";

/// Runtime directory for the toadStool socket tree (`/run/toadstool/`).
///
/// Contains: `daemon.sock`, per-cylinder sockets (`cylinder-<bdf>.sock`),
/// and the fleet artifact (`toadstool-fleet.json`).
/// Override: `TOADSTOOL_RUN_DIR` env var. Default: `/run/toadstool`.
pub const TOADSTOOL_RUN_DIR: &str = "TOADSTOOL_RUN_DIR";

pub const TOADSTOOL_PORT: &str = "TOADSTOOL_PORT";
pub const TOADSTOOL_REQUEST_TIMEOUT: &str = "TOADSTOOL_REQUEST_TIMEOUT";
pub const TOADSTOOL_DNS_RESOLVERS: &str = "TOADSTOOL_DNS_RESOLVERS";
pub const TOADSTOOL_BASE_DOMAIN: &str = "TOADSTOOL_BASE_DOMAIN";
pub const TOADSTOOL_DEBUG: &str = "TOADSTOOL_DEBUG";
pub const TOADSTOOL_LOG_LEVEL: &str = "TOADSTOOL_LOG_LEVEL";
pub const TOADSTOOL_DATA_DIR: &str = "TOADSTOOL_DATA_DIR";
pub const TOADSTOOL_CACHE_DIR: &str = "TOADSTOOL_CACHE_DIR";

// Resource limits
pub const TOADSTOOL_MAX_CPU: &str = "TOADSTOOL_MAX_CPU";
pub const TOADSTOOL_MAX_MEMORY: &str = "TOADSTOOL_MAX_MEMORY";
pub const TOADSTOOL_MAX_CONCURRENT_EXECUTIONS: &str = "TOADSTOOL_MAX_CONCURRENT_EXECUTIONS";
pub const TOADSTOOL_EXECUTION_TIMEOUT: &str = "TOADSTOOL_EXECUTION_TIMEOUT";

// Logging
pub const TOADSTOOL_LOG_TO_FILE: &str = "TOADSTOOL_LOG_TO_FILE";
pub const TOADSTOOL_LOG_FILE: &str = "TOADSTOOL_LOG_FILE";
pub const TOADSTOOL_LOG_FORMAT: &str = "TOADSTOOL_LOG_FORMAT";
pub const TOADSTOOL_LOG_COLORS: &str = "TOADSTOOL_LOG_COLORS";
pub const TOADSTOOL_LOG_TIMESTAMPS: &str = "TOADSTOOL_LOG_TIMESTAMPS";
pub const TOADSTOOL_LOG_THREAD_IDS: &str = "TOADSTOOL_LOG_THREAD_IDS";
pub const TOADSTOOL_LOG_MODULE_PATHS: &str = "TOADSTOOL_LOG_MODULE_PATHS";
pub const TOADSTOOL_LOG_ROTATION: &str = "TOADSTOOL_LOG_ROTATION";
pub const TOADSTOOL_LOG_MAX_SIZE: &str = "TOADSTOOL_LOG_MAX_SIZE";
pub const TOADSTOOL_LOG_MAX_FILES: &str = "TOADSTOOL_LOG_MAX_FILES";

// Container/WASM/Python runtimes
pub const TOADSTOOL_CONTAINER_RUNTIME: &str = "TOADSTOOL_CONTAINER_RUNTIME";
pub const TOADSTOOL_CONTAINER_REGISTRY: &str = "TOADSTOOL_CONTAINER_REGISTRY";
pub const TOADSTOOL_CONTAINER_NETWORK_MODE: &str = "TOADSTOOL_CONTAINER_NETWORK_MODE";
pub const TOADSTOOL_WASM_ENGINE: &str = "TOADSTOOL_WASM_ENGINE";
pub const TOADSTOOL_WASM_MAX_MEMORY: &str = "TOADSTOOL_WASM_MAX_MEMORY";
pub const TOADSTOOL_WASM_ENABLE_WASI: &str = "TOADSTOOL_WASM_ENABLE_WASI";
pub const TOADSTOOL_PYTHON_EXECUTABLE: &str = "TOADSTOOL_PYTHON_EXECUTABLE";
pub const TOADSTOOL_PYTHON_VENV_PATH: &str = "TOADSTOOL_PYTHON_VENV_PATH";
pub const TOADSTOOL_PYTHON_INDEX_URL: &str = "TOADSTOOL_PYTHON_INDEX_URL";
pub const TOADSTOOL_PYTHON_MAX_MEMORY: &str = "TOADSTOOL_PYTHON_MAX_MEMORY";

// Security
pub const TOADSTOOL_JWT_SECRET: &str = "TOADSTOOL_JWT_SECRET";
pub const TOADSTOOL_SESSION_TIMEOUT: &str = "TOADSTOOL_SESSION_TIMEOUT";
pub const TOADSTOOL_MAX_LOGIN_ATTEMPTS: &str = "TOADSTOOL_MAX_LOGIN_ATTEMPTS";
pub const TOADSTOOL_LOCKOUT_DURATION: &str = "TOADSTOOL_LOCKOUT_DURATION";
pub const TOADSTOOL_ENCRYPTION_ENABLED: &str = "TOADSTOOL_ENCRYPTION_ENABLED";
pub const TOADSTOOL_ENCRYPTION_ALGORITHM: &str = "TOADSTOOL_ENCRYPTION_ALGORITHM";
pub const TOADSTOOL_ENCRYPTION_KEY_LENGTH: &str = "TOADSTOOL_ENCRYPTION_KEY_LENGTH";
pub const TOADSTOOL_AUDIT_ENABLED: &str = "TOADSTOOL_AUDIT_ENABLED";
pub const TOADSTOOL_AUDIT_LOG_FILE: &str = "TOADSTOOL_AUDIT_LOG_FILE";
pub const TOADSTOOL_AUDIT_LOG_LEVEL: &str = "TOADSTOOL_AUDIT_LOG_LEVEL";
pub const TOADSTOOL_SANDBOX_TYPE: &str = "TOADSTOOL_SANDBOX_TYPE";
pub const TOADSTOOL_SANDBOX_ALLOW_NETWORK: &str = "TOADSTOOL_SANDBOX_ALLOW_NETWORK";
pub const TOADSTOOL_SANDBOX_ALLOW_FILE_ACCESS: &str = "TOADSTOOL_SANDBOX_ALLOW_FILE_ACCESS";

// Feature flags
pub const TOADSTOOL_ENABLE_METRICS: &str = "TOADSTOOL_ENABLE_METRICS";
pub const TOADSTOOL_ENABLE_CACHE: &str = "TOADSTOOL_ENABLE_CACHE";
pub const TOADSTOOL_ENABLE_AUTH: &str = "TOADSTOOL_ENABLE_AUTH";
pub const TOADSTOOL_ENABLE_SANDBOX: &str = "TOADSTOOL_ENABLE_SANDBOX";
pub const TOADSTOOL_ENABLE_FEDERATION: &str = "TOADSTOOL_ENABLE_FEDERATION";
pub const TOADSTOOL_ENABLE_DISTRIBUTED: &str = "TOADSTOOL_ENABLE_DISTRIBUTED";
pub const TOADSTOOL_ENABLE_AUTO_CONFIG: &str = "TOADSTOOL_ENABLE_AUTO_CONFIG";
pub const TOADSTOOL_ENABLE_HOT_RELOAD: &str = "TOADSTOOL_ENABLE_HOT_RELOAD";
pub const TOADSTOOL_ENABLE_EXPERIMENTAL: &str = "TOADSTOOL_ENABLE_EXPERIMENTAL";
pub const TOADSTOOL_ENABLE_BETA: &str = "TOADSTOOL_ENABLE_BETA";
pub const TOADSTOOL_ENABLE_PROFILING: &str = "TOADSTOOL_ENABLE_PROFILING";
pub const TOADSTOOL_ENABLE_OPENAPI: &str = "TOADSTOOL_ENABLE_OPENAPI";
pub const TOADSTOOL_ENABLE_GRAPHQL: &str = "TOADSTOOL_ENABLE_GRAPHQL";

// App config
pub const TOADSTOOL_VERBOSE: &str = "TOADSTOOL_VERBOSE";
pub const TOADSTOOL_WORKER_THREADS: &str = "TOADSTOOL_WORKER_THREADS";

pub const ENABLE_PRIMAL_CAPABILITIES: &str = "ENABLE_PRIMAL_CAPABILITIES";
pub const PRIMAL_HEARTBEAT_INTERVAL: &str = "PRIMAL_HEARTBEAT_INTERVAL";

// BTSP family seed (handshake key material).
pub const FAMILY_SEED: &str = "FAMILY_SEED";

// Auth tokens (coordination plane registration).
pub const COORDINATION_AUTH_TOKEN: &str = "COORDINATION_AUTH_TOKEN";
/// **Deprecated** (identity-based). Prefer `COORDINATION_AUTH_TOKEN`.
#[deprecated(since = "0.4.0", note = "use COORDINATION_AUTH_TOKEN")]
pub const LEGACY_SONGBIRD_AUTH_TOKEN: &str = "SONGBIRD_AUTH_TOKEN";

// Optional capability endpoint env names (not socket-specific).
pub const AUTHENTICATION_URL: &str = "AUTHENTICATION_URL";
pub const NLP_URL: &str = "NLP_URL";
pub const TOADSTOOL_ENTROPY_SERVICE_URL: &str = "TOADSTOOL_ENTROPY_SERVICE_URL";

/// Universal adapter: explicit capability provider URL for discovery (`EnvironmentSource`).
pub const TOADSTOOL_SECURITY_PROVIDER: &str = "TOADSTOOL_SECURITY_PROVIDER";
/// Universal adapter: explicit capability provider URL for discovery (`EnvironmentSource`).
pub const TOADSTOOL_STORAGE_PROVIDER: &str = "TOADSTOOL_STORAGE_PROVIDER";
/// Universal adapter: explicit capability provider URL for discovery (`EnvironmentSource`).
pub const TOADSTOOL_COORDINATION_PROVIDER: &str = "TOADSTOOL_COORDINATION_PROVIDER";
/// Universal adapter: explicit capability provider URL for discovery (`EnvironmentSource`).
pub const TOADSTOOL_INTELLIGENCE_PROVIDER: &str = "TOADSTOOL_INTELLIGENCE_PROVIDER";

/// Comma-separated wgpu adapter selector (index, name substring, or `auto`).
pub const TOADSTOOL_GPU_ADAPTER: &str = "TOADSTOOL_GPU_ADAPTER";
/// Override directory for hw-learn recipe persistence.
pub const TOADSTOOL_HW_LEARN_STORE: &str = "TOADSTOOL_HW_LEARN_STORE";
/// Explicit shader compiler UDS path override.
pub const TOADSTOOL_SHADER_COMPILER_ADDR: &str = "TOADSTOOL_SHADER_COMPILER_ADDR";
/// Set `1` or `true` to restrict GPU backends to Vulkan (no display server probe).
pub const TOADSTOOL_HEADLESS: &str = "TOADSTOOL_HEADLESS";

/// Override path to the `rm_trigger` binary for catalyst sovereign handoff.
pub const TOADSTOOL_RM_TRIGGER_BIN: &str = "TOADSTOOL_RM_TRIGGER_BIN";

/// Override path for sovereign handoff forensics breadcrumb log.
pub const TOADSTOOL_FORENSICS_LOG: &str = "TOADSTOOL_FORENSICS_LOG";

/// Display IPC: full `host:port` TCP endpoint override.
pub const TOADSTOOL_DISPLAY_IPC_ADDR: &str = "TOADSTOOL_DISPLAY_IPC_ADDR";
/// Display IPC: port-only override (pairs with loopback host from shared network constants).
pub const TOADSTOOL_DISPLAY_IPC_PORT: &str = "TOADSTOOL_DISPLAY_IPC_PORT";

/// Localhost discovery: HTTP port for native compute when no service registry is available.
pub const TOADSTOOL_LOCAL_PORT: &str = "TOADSTOOL_LOCAL_PORT";

// systemd socket activation / notification.
pub const NOTIFY_SOCKET: &str = "NOTIFY_SOCKET";
pub const LISTEN_FDS: &str = "LISTEN_FDS";
pub const LISTEN_PID: &str = "LISTEN_PID";
pub const LISTEN_FDNAMES: &str = "LISTEN_FDNAMES";

// Server identity (unibin).
pub const TOADSTOOL_GATE_ID: &str = "TOADSTOOL_GATE_ID";
pub const TOADSTOOL_AUTH_MODE: &str = "TOADSTOOL_AUTH_MODE";
pub const TOADSTOOL_DEPLOYMENT_MODEL: &str = "TOADSTOOL_DEPLOYMENT_MODEL";
/// Gate id that owns GPUs on this node (guest nodes set to the remote owner).
pub const TOADSTOOL_HARDWARE_OWNER_GATE_ID: &str = "TOADSTOOL_HARDWARE_OWNER_GATE_ID";

// Domain discovery (DNS-based fallback naming).
pub const COMPUTE_DOMAIN: &str = "COMPUTE_DOMAIN";
pub const TOADSTOOL_DOMAIN: &str = "TOADSTOOL_DOMAIN";
pub const COORDINATION_DOMAIN: &str = "COORDINATION_DOMAIN";
pub const SECURITY_DOMAIN: &str = "SECURITY_DOMAIN";
pub const STORAGE_DOMAIN: &str = "STORAGE_DOMAIN";
pub const AI_PROCESSING_DOMAIN: &str = "AI_PROCESSING_DOMAIN";
pub const BIOMEOS_DOMAIN: &str = "BIOMEOS_DOMAIN";
/// **Deprecated** (identity-based). Prefer `COORDINATION_DOMAIN`.
#[deprecated(since = "0.4.0", note = "use COORDINATION_DOMAIN")]
pub const SONGBIRD_DOMAIN: &str = "SONGBIRD_DOMAIN";
/// **Deprecated** (identity-based). Prefer `SECURITY_DOMAIN`.
#[deprecated(since = "0.4.0", note = "use SECURITY_DOMAIN")]
pub const BEARDOG_DOMAIN: &str = "BEARDOG_DOMAIN";
/// **Deprecated** (identity-based). Prefer `STORAGE_DOMAIN`.
#[deprecated(since = "0.4.0", note = "use STORAGE_DOMAIN")]
pub const NESTGATE_DOMAIN: &str = "NESTGATE_DOMAIN";
/// **Deprecated** (identity-based). Prefer `AI_PROCESSING_DOMAIN`.
#[deprecated(since = "0.4.0", note = "use AI_PROCESSING_DOMAIN")]
pub const SQUIRREL_DOMAIN: &str = "SQUIRREL_DOMAIN";

// DNS and server config.
pub const TOADSTOOL_DNS_SERVERS: &str = "TOADSTOOL_DNS_SERVERS";
pub const TOADSTOOL_DNS_SEARCH_DOMAINS: &str = "TOADSTOOL_DNS_SEARCH_DOMAINS";
pub const TOADSTOOL_TEMP_DIR: &str = "TOADSTOOL_TEMP_DIR";
pub const TOADSTOOL_SECURITY_WARNING_ACKNOWLEDGED: &str = "TOADSTOOL_SECURITY_WARNING_ACKNOWLEDGED";

// Crypto provider public key discovery (capability-based, then legacy identity fallback).
pub const CRYPTO_PROVIDER_PUBLIC_KEY: &str = "CRYPTO_PROVIDER_PUBLIC_KEY";
pub const STORAGE_PROVIDER_PUBLIC_KEY: &str = "STORAGE_PROVIDER_PUBLIC_KEY";
pub const DISCOVERY_PROVIDER_PUBLIC_KEY: &str = "DISCOVERY_PROVIDER_PUBLIC_KEY";
/// **Deprecated** (identity-based). Prefer `CRYPTO_PROVIDER_PUBLIC_KEY`.
#[deprecated(since = "0.4.0", note = "use CRYPTO_PROVIDER_PUBLIC_KEY")]
pub const BEARDOG_PUBLIC_KEY: &str = "BEARDOG_PUBLIC_KEY";
/// **Deprecated** (identity-based). Prefer `STORAGE_PROVIDER_PUBLIC_KEY`.
#[deprecated(since = "0.4.0", note = "use STORAGE_PROVIDER_PUBLIC_KEY")]
pub const NESTGATE_PUBLIC_KEY: &str = "NESTGATE_PUBLIC_KEY";
/// **Deprecated** (identity-based). Prefer `DISCOVERY_PROVIDER_PUBLIC_KEY`.
#[deprecated(since = "0.4.0", note = "use DISCOVERY_PROVIDER_PUBLIC_KEY")]
pub const SONGBIRD_PUBLIC_KEY: &str = "SONGBIRD_PUBLIC_KEY";

// Cylinder hardware paths and ember lifecycle.
/// sysfs mount root for portable deployments, tests, and mock trees.
pub const TOADSTOOL_SYSFS_ROOT: &str = "TOADSTOOL_SYSFS_ROOT";
/// procfs mount root for portable deployments and kernel release probing.
pub const TOADSTOOL_PROC_ROOT: &str = "TOADSTOOL_PROC_ROOT";
pub const TOADSTOOL_EMBER_GATE: &str = "TOADSTOOL_EMBER_GATE";
pub const TOADSTOOL_DRI_RENDER_PREFIX: &str = "TOADSTOOL_DRI_RENDER_PREFIX";
pub const TOADSTOOL_EMBER_SOCKET: &str = "TOADSTOOL_EMBER_SOCKET";
pub const BIOMEOS_ECOSYSTEM_NAMESPACE: &str = "BIOMEOS_ECOSYSTEM_NAMESPACE";

// Vulkan / GPU tooling.
pub const VK_ICD_FILENAMES: &str = "VK_ICD_FILENAMES";
/// CI environment detection (GitHub Actions, etc.).
pub const CI: &str = "CI";

// Cloud / deployment layer hints (AWS, GCP, Azure — used by environment detection).
pub const AWS_EXECUTION_ENV: &str = "AWS_EXECUTION_ENV";
pub const AWS_LAMBDA_FUNCTION_NAME: &str = "AWS_LAMBDA_FUNCTION_NAME";
pub const ECS_CONTAINER_METADATA_URI: &str = "ECS_CONTAINER_METADATA_URI";
pub const AWS_INSTANCE_TYPE: &str = "AWS_INSTANCE_TYPE";
pub const EC2_INSTANCE_TYPE: &str = "EC2_INSTANCE_TYPE";
pub const AWS_REGION: &str = "AWS_REGION";
pub const AWS_DEFAULT_REGION: &str = "AWS_DEFAULT_REGION";

pub const GCP_PROJECT: &str = "GCP_PROJECT";
pub const GOOGLE_CLOUD_PROJECT: &str = "GOOGLE_CLOUD_PROJECT";
pub const GCLOUD_PROJECT: &str = "GCLOUD_PROJECT";
pub const GCE_MACHINE_TYPE: &str = "GCE_MACHINE_TYPE";
pub const GCE_ZONE: &str = "GCE_ZONE";
pub const GOOGLE_CLOUD_ZONE: &str = "GOOGLE_CLOUD_ZONE";

pub const AZURE_SUBSCRIPTION_ID: &str = "AZURE_SUBSCRIPTION_ID";
pub const WEBSITE_INSTANCE_ID: &str = "WEBSITE_INSTANCE_ID";
pub const FUNCTIONS_WORKER_RUNTIME: &str = "FUNCTIONS_WORKER_RUNTIME";
pub const AZURE_VM_SIZE: &str = "AZURE_VM_SIZE";
pub const AZURE_LOCATION: &str = "AZURE_LOCATION";
pub const AZURE_REGION: &str = "AZURE_REGION";

// ── Environment / runtime mode ──────────────────────────────────────────
pub const TOADSTOOL_ENVIRONMENT: &str = "TOADSTOOL_ENVIRONMENT";
pub const ENVIRONMENT: &str = "ENVIRONMENT";
pub const ENV: &str = "ENV";
pub const HOST: &str = "HOST";
pub const DISPLAY: &str = "DISPLAY";
pub const WAYLAND_DISPLAY: &str = "WAYLAND_DISPLAY";

// ── Discovery infrastructure ────────────────────────────────────────────
pub const TOADSTOOL_DISCOVERY_CONFIG: &str = "TOADSTOOL_DISCOVERY_CONFIG";
pub const TOADSTOOL_DISCOVERY_FALLBACK_PORT: &str = "TOADSTOOL_DISCOVERY_FALLBACK_PORT";
pub const TOADSTOOL_DISCOVERY_FALLBACK_ENABLED: &str = "TOADSTOOL_DISCOVERY_FALLBACK_ENABLED";
pub const TOADSTOOL_SERVICE_DIR: &str = "TOADSTOOL_SERVICE_DIR";
pub const TOADSTOOL_REGISTRY_ENDPOINT: &str = "TOADSTOOL_REGISTRY_ENDPOINT";
pub const BIOMEOS_RUNTIME_DIR: &str = "BIOMEOS_RUNTIME_DIR";

// ── Service URLs (capability-based, not primal-named) ────────────────
pub const TOADSTOOL_COORDINATION_SERVICE_URL: &str = "TOADSTOOL_COORDINATION_SERVICE_URL";
pub const TOADSTOOL_CRYPTO_SERVICE_URL: &str = "TOADSTOOL_CRYPTO_SERVICE_URL";
pub const TOADSTOOL_STORAGE_SERVICE_URL: &str = "TOADSTOOL_STORAGE_SERVICE_URL";
pub const TOADSTOOL_AI_SERVICE_URL: &str = "TOADSTOOL_AI_SERVICE_URL";
pub const TOADSTOOL_COORDINATOR: &str = "TOADSTOOL_COORDINATOR";
pub const TOADSTOOL_STORAGE: &str = "TOADSTOOL_STORAGE";
pub const TOADSTOOL_SERVICES: &str = "TOADSTOOL_SERVICES";

// ── K8s / container orchestration (external env vars) ─────────────────
pub const KUBERNETES_SERVICE_HOST: &str = "KUBERNETES_SERVICE_HOST";
pub const POD_NAMESPACE: &str = "POD_NAMESPACE";
pub const COMPOSE_PROJECT_NAME: &str = "COMPOSE_PROJECT_NAME";
pub const CONSUL_HTTP_ADDR: &str = "CONSUL_HTTP_ADDR";
pub const ETCD_ENDPOINTS: &str = "ETCD_ENDPOINTS";

// ── Monitoring / observability ──────────────────────────────────────────
pub const TOADSTOOL_TELEMETRY: &str = "TOADSTOOL_TELEMETRY";
pub const TOADSTOOL_PROMETHEUS_PORT: &str = "TOADSTOOL_PROMETHEUS_PORT";
pub const PROMETHEUS_PORT: &str = "PROMETHEUS_PORT";
pub const TOADSTOOL_PROMETHEUS_HOST: &str = "TOADSTOOL_PROMETHEUS_HOST";
pub const PROMETHEUS_HOST: &str = "PROMETHEUS_HOST";
pub const TOADSTOOL_JAEGER_ENDPOINT: &str = "TOADSTOOL_JAEGER_ENDPOINT";
pub const TOADSTOOL_AUDIT_LOG_PATH: &str = "TOADSTOOL_AUDIT_LOG_PATH";

// ── TLS / certificate paths ────────────────────────────────────────────
pub const TOADSTOOL_CA_CERT: &str = "TOADSTOOL_CA_CERT";
pub const TOADSTOOL_SERVICE_CERT: &str = "TOADSTOOL_SERVICE_CERT";
pub const TOADSTOOL_SERVICE_KEY: &str = "TOADSTOOL_SERVICE_KEY";

// ── Client configuration ───────────────────────────────────────────────
pub const TOADSTOOL_SERVER_URL: &str = "TOADSTOOL_SERVER_URL";
pub const TOADSTOOL_REQUEST_TIMEOUT_MS: &str = "TOADSTOOL_REQUEST_TIMEOUT_MS";
pub const TOADSTOOL_MAX_RETRIES: &str = "TOADSTOOL_MAX_RETRIES";
pub const TOADSTOOL_RETRY_BACKOFF_MS: &str = "TOADSTOOL_RETRY_BACKOFF_MS";

// ── Discovery / auto-config ────────────────────────────────────────────
pub const TOADSTOOL_SKIP_DISCOVERY: &str = "TOADSTOOL_SKIP_DISCOVERY";
pub const TOADSTOOL_DISCOVERY_BIND_ADDR: &str = "TOADSTOOL_DISCOVERY_BIND_ADDR";
pub const TOADSTOOL_DISCOVERY_HTTP_PORT: &str = "TOADSTOOL_DISCOVERY_HTTP_PORT";
pub const TOADSTOOL_SCAN_SUBNET: &str = "TOADSTOOL_SCAN_SUBNET";
pub const TOADSTOOL_AUTO_DISCOVER: &str = "TOADSTOOL_AUTO_DISCOVER";
pub const TOADSTOOL_SIDECAR_IMAGE: &str = "TOADSTOOL_SIDECAR_IMAGE";
pub const TOADSTOOL_CREDENTIALS: &str = "TOADSTOOL_CREDENTIALS";
pub const SERVICE_REGISTRY_URL: &str = "SERVICE_REGISTRY_URL";

// ── IPC / biomeOS ──────────────────────────────────────────────────────
pub const BIOMEOS_IPC_PORT: &str = "BIOMEOS_IPC_PORT";
pub const TOADSTOOL_ENDPOINT: &str = "TOADSTOOL_ENDPOINT";
pub const TOADSTOOL_FEDERATION_ENDPOINT: &str = "TOADSTOOL_FEDERATION_ENDPOINT";
pub const TOADSTOOL_CRYPTO_PERMISSIONS_STORE: &str = "TOADSTOOL_CRYPTO_PERMISSIONS_STORE";
pub const DBUS_SESSION_BUS_ADDRESS: &str = "DBUS_SESSION_BUS_ADDRESS";

// ── Profiler configuration ─────────────────────────────────────────────
pub const TOADSTOOL_PROFILER_WARMUP: &str = "TOADSTOOL_PROFILER_WARMUP";
pub const TOADSTOOL_PROFILER_BENCH_ITERS: &str = "TOADSTOOL_PROFILER_BENCH_ITERS";
pub const TOADSTOOL_PROFILER_TIMEOUT_MS: &str = "TOADSTOOL_PROFILER_TIMEOUT_MS";
pub const TOADSTOOL_PROFILER_PARALLEL: &str = "TOADSTOOL_PROFILER_PARALLEL";
pub const TOADSTOOL_PROFILER_DETAILED: &str = "TOADSTOOL_PROFILER_DETAILED";
pub const TOADSTOOL_PROFILER_OUTPUT: &str = "TOADSTOOL_PROFILER_OUTPUT";

// ── Substrate detection ────────────────────────────────────────────────
pub const TOADSTOOL_SUBSTRATE_PREFERRED: &str = "TOADSTOOL_SUBSTRATE_PREFERRED";
pub const TOADSTOOL_POWER_BUDGET: &str = "TOADSTOOL_POWER_BUDGET";
pub const TOADSTOOL_PERFORMANCE_TARGET: &str = "TOADSTOOL_PERFORMANCE_TARGET";
pub const BIND_ADDRESS: &str = "BIND_ADDRESS";
pub const PRIMAL_CAPABILITIES_PATH: &str = "PRIMAL_CAPABILITIES_PATH";

// ── Auth ────────────────────────────────────────────────────────────────
pub const TOADSTOOL_AUTH_AUDIENCE: &str = "TOADSTOOL_AUTH_AUDIENCE";

// ── Ember / hardware-vendor lifecycle ──────────────────────────────────
pub const TOADSTOOL_INTEL_SETTLE_SECS: &str = "TOADSTOOL_INTEL_SETTLE_SECS";

// ── GPU testing ─────────────────────────────────────────────────────────
pub const TOADSTOOL_WGPU_SAFE: &str = "TOADSTOOL_WGPU_SAFE";

// ── Cross-platform ─────────────────────────────────────────────────────
pub const COMPUTERNAME: &str = "COMPUTERNAME";
pub const ANDROID_ROOT: &str = "ANDROID_ROOT";
pub const OS: &str = "OS";

// ── Mainframe terminal ─────────────────────────────────────────────────
pub const TOADSTOOL_MAINFRAME_3270_HOST: &str = "TOADSTOOL_MAINFRAME_3270_HOST";
pub const TOADSTOOL_MAINFRAME_5250_HOST: &str = "TOADSTOOL_MAINFRAME_5250_HOST";

// ── External SDK / third-party substrate detection ─────────────────────
pub const XILINX_XRT: &str = "XILINX_XRT";
pub const QUARTUS_ROOTDIR: &str = "QUARTUS_ROOTDIR";
pub const AKIDA_DEVICE_ID: &str = "AKIDA_DEVICE_ID";
pub const AKIDA_DRIVER_PATH: &str = "AKIDA_DRIVER_PATH";
pub const SPINNAKER_ROOT: &str = "SPINNAKER_ROOT";
pub const QISKIT_HOME: &str = "QISKIT_HOME";
pub const CIRQ_HOME: &str = "CIRQ_HOME";
pub const IBM_QUANTUM_TOKEN: &str = "IBM_QUANTUM_TOKEN";
pub const RIGETTI_QCS_TOKEN: &str = "RIGETTI_QCS_TOKEN";
pub const TWIST_BIOSCIENCE_API_KEY: &str = "TWIST_BIOSCIENCE_API_KEY";

// ── Deprecated legacy ────────────────────────────────────────────────
#[deprecated(since = "0.5.0", note = "use FAMILY_SEED")]
pub const BEARDOG_FAMILY_SEED: &str = "BEARDOG_FAMILY_SEED";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_socket_vars_match_const_name() {
        assert_eq!(BIOMEOS_CRYPTO_SOCKET, "BIOMEOS_CRYPTO_SOCKET");
        assert_eq!(BIOMEOS_COORDINATION_SOCKET, "BIOMEOS_COORDINATION_SOCKET");
        assert_eq!(BIOMEOS_STORAGE_SOCKET, "BIOMEOS_STORAGE_SOCKET");
        assert_eq!(BIOMEOS_ROUTING_SOCKET, "BIOMEOS_ROUTING_SOCKET");
        assert_eq!(TOADSTOOL_SECURITY_SOCKET, "TOADSTOOL_SECURITY_SOCKET");
        assert_eq!(
            TOADSTOOL_COORDINATION_SOCKET,
            "TOADSTOOL_COORDINATION_SOCKET"
        );
        assert_eq!(TOADSTOOL_STORAGE_SOCKET, "TOADSTOOL_STORAGE_SOCKET");
        assert_eq!(
            TOADSTOOL_INTELLIGENCE_SOCKET,
            "TOADSTOOL_INTELLIGENCE_SOCKET"
        );
    }

    #[test]
    fn xdg_vars_match_specification() {
        assert_eq!(XDG_RUNTIME_DIR, "XDG_RUNTIME_DIR");
        assert_eq!(XDG_DATA_HOME, "XDG_DATA_HOME");
        assert_eq!(XDG_CACHE_HOME, "XDG_CACHE_HOME");
        assert_eq!(XDG_CONFIG_HOME, "XDG_CONFIG_HOME");
    }

    #[test]
    fn core_identity_vars_use_toadstool_prefix() {
        assert!(TOADSTOOL_FAMILY_ID.starts_with("TOADSTOOL_"));
        assert!(TOADSTOOL_NODE_ID.starts_with("TOADSTOOL_"));
        assert!(TOADSTOOL_AUTH_ISSUER.starts_with("TOADSTOOL_"));
    }

    #[test]
    fn biomeos_socket_dir_env_matches() {
        assert_eq!(BIOMEOS_SOCKET_DIR, "BIOMEOS_SOCKET_DIR");
    }

    #[test]
    fn toadstool_env_var_matches() {
        assert_eq!(TOADSTOOL_ENV, "TOADSTOOL_ENV");
    }

    #[test]
    #[expect(deprecated)]
    fn legacy_vars_are_identity_based() {
        assert!(LEGACY_BEARDOG_SOCKET_ENV.contains("BEARDOG"));
        assert!(LEGACY_NESTGATE_SOCKET_ENV.contains("NESTGATE"));
    }

    #[test]
    fn socket_mode_env_name_matches() {
        assert_eq!(TOADSTOOL_SOCKET_MODE, "TOADSTOOL_SOCKET_MODE");
    }
}
