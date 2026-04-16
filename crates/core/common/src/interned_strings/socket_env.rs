// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(
    missing_docs,
    reason = "each pub const is the env var name string; module docs describe usage"
)]
//! Environment variable names for runtime socket discovery (`SocketPathEnv::from_env`).
//!
//! Prefer capability-prefixed vars (`BIOMEOS_*_SOCKET`, `TOADSTOOL_*_SOCKET`). Legacy
//! `LEGACY_*_SOCKET_ENV` names are identity-based fallbacks only; see
//! `capabilities::*` and capability discovery in the wateringHole standards.

pub const XDG_RUNTIME_DIR: &str = "XDG_RUNTIME_DIR";
pub const USER: &str = "USER";
/// Standard home directory path (shell / POSIX).
pub const HOME: &str = "HOME";

pub const TOADSTOOL_FAMILY_ID: &str = "TOADSTOOL_FAMILY_ID";
pub const TOADSTOOL_FAMILY: &str = "TOADSTOOL_FAMILY";
pub const BIOMEOS_FAMILY_ID: &str = "BIOMEOS_FAMILY_ID";
/// Node identity for multi-instance deployments (orchestration / logging).
pub const TOADSTOOL_NODE_ID: &str = "TOADSTOOL_NODE_ID";

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
pub const LEGACY_BEARDOG_SOCKET_ENV: &str = "BEARDOG_SOCKET";
/// **Deprecated** (identity-based). Prefer `BIOMEOS_COORDINATION_SOCKET` or
/// `interned_strings::capabilities::COORDINATION`.
pub const LEGACY_SONGBIRD_SOCKET_ENV: &str = "SONGBIRD_SOCKET";
/// **Deprecated** (identity-based). Prefer `BIOMEOS_STORAGE_SOCKET` or
/// `interned_strings::capabilities::STORAGE`.
pub const LEGACY_NESTGATE_SOCKET_ENV: &str = "NESTGATE_SOCKET";
/// **Deprecated** (identity-based). Prefer `BIOMEOS_ROUTING_SOCKET` or
/// `interned_strings::capabilities::ROUTING`.
pub const LEGACY_SQUIRREL_SOCKET_ENV: &str = "SQUIRREL_SOCKET";

pub const TOADSTOOL_SOCKET: &str = "TOADSTOOL_SOCKET";
/// **Deprecated** (identity-based). Prefer `TOADSTOOL_SOCKET` or capability discovery.
pub const PRIMAL_SOCKET: &str = "PRIMAL_SOCKET";
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
pub const LEGACY_SONGBIRD_URL: &str = "SONGBIRD_URL";
/// **Deprecated** (identity-based). Prefer `COORDINATION_ENDPOINT`.
pub const LEGACY_SONGBIRD_ENDPOINT: &str = "SONGBIRD_ENDPOINT";

pub const TOADSTOOL_SECURITY_ENDPOINT: &str = "TOADSTOOL_SECURITY_ENDPOINT";
pub const SECURITY_URL: &str = "SECURITY_URL";
pub const SECURITY_ENDPOINT: &str = "SECURITY_ENDPOINT";
/// **Deprecated** (identity-based). Prefer `SECURITY_URL`.
pub const LEGACY_BEARDOG_URL: &str = "BEARDOG_URL";
/// **Deprecated** (identity-based). Prefer `SECURITY_ENDPOINT`.
pub const LEGACY_BEARDOG_ENDPOINT: &str = "BEARDOG_ENDPOINT";

pub const TOADSTOOL_STORAGE_ENDPOINT: &str = "TOADSTOOL_STORAGE_ENDPOINT";
pub const STORAGE_URL: &str = "STORAGE_URL";
pub const STORAGE_ENDPOINT: &str = "STORAGE_ENDPOINT";
/// **Deprecated** (identity-based). Prefer `STORAGE_URL`.
pub const LEGACY_NESTGATE_URL: &str = "NESTGATE_URL";
/// **Deprecated** (identity-based). Prefer `STORAGE_ENDPOINT`.
pub const LEGACY_NESTGATE_ENDPOINT: &str = "NESTGATE_ENDPOINT";

pub const TOADSTOOL_AI_ENDPOINT: &str = "TOADSTOOL_AI_ENDPOINT";
pub const TOADSTOOL_INTELLIGENCE_ENDPOINT: &str = "TOADSTOOL_INTELLIGENCE_ENDPOINT";
pub const AI_PROCESSING_ENDPOINT: &str = "AI_PROCESSING_ENDPOINT";
pub const TOADSTOOL_AI_PROCESSING_ENDPOINT: &str = "TOADSTOOL_AI_PROCESSING_ENDPOINT";
pub const INTELLIGENCE_URL: &str = "INTELLIGENCE_URL";
pub const INTELLIGENCE_ENDPOINT: &str = "INTELLIGENCE_ENDPOINT";
pub const AI_PROCESSING_URL: &str = "AI_PROCESSING_URL";
/// **Deprecated** (identity-based). Prefer `INTELLIGENCE_URL`.
pub const LEGACY_SQUIRREL_URL: &str = "SQUIRREL_URL";
/// **Deprecated** (identity-based). Prefer `INTELLIGENCE_ENDPOINT`.
pub const LEGACY_SQUIRREL_ENDPOINT: &str = "SQUIRREL_ENDPOINT";

// HTTP URL overrides (parallel naming to `*_ENDPOINT`; used by discovery fallbacks and demos).
pub const TOADSTOOL_COORDINATION_URL: &str = "TOADSTOOL_COORDINATION_URL";
pub const TOADSTOOL_SECURITY_URL: &str = "TOADSTOOL_SECURITY_URL";
pub const TOADSTOOL_STORAGE_URL: &str = "TOADSTOOL_STORAGE_URL";

// Capability service ports (HTTP fallback / local bind hints).
pub const TOADSTOOL_COORDINATION_PORT: &str = "TOADSTOOL_COORDINATION_PORT";
pub const TOADSTOOL_SECURITY_PORT: &str = "TOADSTOOL_SECURITY_PORT";
pub const TOADSTOOL_STORAGE_PORT: &str = "TOADSTOOL_STORAGE_PORT";
/// **Deprecated** (identity-based). Prefer `TOADSTOOL_COORDINATION_PORT`.
pub const TOADSTOOL_SONGBIRD_PORT: &str = "TOADSTOOL_SONGBIRD_PORT";

// Discovery bootstrap and bind hints.
pub const TOADSTOOL_MDNS_ENABLE: &str = "TOADSTOOL_MDNS_ENABLE";
pub const TOADSTOOL_MDNS_REQUIRE: &str = "TOADSTOOL_MDNS_REQUIRE";
pub const TOADSTOOL_DISCOVERY_FALLBACKS: &str = "TOADSTOOL_DISCOVERY_FALLBACKS";
pub const TOADSTOOL_ENV: &str = "TOADSTOOL_ENV";
pub const TOADSTOOL_BIND_HOST: &str = "TOADSTOOL_BIND_HOST";
pub const BIND_HOST: &str = "BIND_HOST";
pub const TOADSTOOL_BIND_ADDRESS: &str = "TOADSTOOL_BIND_ADDRESS";
pub const TOADSTOOL_TCP_BIND_ADDRESS: &str = "TOADSTOOL_TCP_BIND_ADDRESS";
/// Idle timeout (seconds) for Pure JSON-RPC TCP connections.
pub const TOADSTOOL_TCP_IDLE_TIMEOUT_SECS: &str = "TOADSTOOL_TCP_IDLE_TIMEOUT_SECS";
pub const TOADSTOOL_STANDALONE: &str = "TOADSTOOL_STANDALONE";

// BTSP family seed (handshake key material).
pub const FAMILY_SEED: &str = "FAMILY_SEED";

// Auth tokens (coordination plane registration).
pub const COORDINATION_AUTH_TOKEN: &str = "COORDINATION_AUTH_TOKEN";
/// **Deprecated** (identity-based). Prefer `COORDINATION_AUTH_TOKEN`.
pub const LEGACY_SONGBIRD_AUTH_TOKEN: &str = "SONGBIRD_AUTH_TOKEN";

// Optional capability endpoint env names (not socket-specific).
pub const AUTHENTICATION_URL: &str = "AUTHENTICATION_URL";
pub const NLP_URL: &str = "NLP_URL";
pub const TOADSTOOL_ENTROPY_SERVICE_URL: &str = "TOADSTOOL_ENTROPY_SERVICE_URL";

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
