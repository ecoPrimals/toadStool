// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure socket path resolution (no env access — inject `SocketPathEnv`).
//!
//! ## Discovery Escalation Hierarchy (primalSpring cross-cutting standard)
//!
//! primalSpring discovers composition members in this order:
//!
//! 1. **Songbird `ipc.resolve`** — coordination plane returns socket paths
//! 2. **biomeOS Neural API** — `capability.discover` on the NUCLEUS socket
//! 3. **UDS filesystem convention** — `{capability}.sock` under biomeOS runtime dir
//! 4. **Socket registry / manifests** — `registry.json` under XDG config
//! 5. **TCP probing** — well-known ports from `tolerances`
//!
//! toadStool's `resolve_capability_socket_fallback` implements tiers 1–4
//! (tier 1 via `DISCOVERY_SOCKET`, tier 2 via `BIOMEOS_*_SOCKET`, tier 3 via
//! filesystem convention, tier 4 via connection hints). TCP probing (tier 5)
//! is not used for local IPC — toadStool prefers Unix domain sockets.
//!
//! No primal is required to support all tiers. If Songbird can resolve a
//! socket path, tier 1 gives the highest-fidelity routing with cross-gate
//! capability.

use std::path::PathBuf;

use super::env::SocketPathEnv;
use crate::interned_strings::CapabilityDomain;

/// Domain-based socket name per `PRIMAL_SELF_KNOWLEDGE_STANDARD.md` v1.1.
pub(super) const SELF_SOCKET: &str = "compute.sock";

/// Map a *peer* service label (legacy primal name or capability ID) to a canonical capability id.
///
/// Self-names ("toadstool", "toad-stool") and platform names ("nucleus", "biomeos")
/// are intentionally excluded — they have dedicated socket resolution paths.
#[must_use]
pub fn service_label_to_capability_id(label: &str) -> Option<&'static str> {
    let lower = label.to_ascii_lowercase();
    match lower.as_str() {
        "toadstool" | "toad-stool" | "nucleus" | "biomeos" => None,
        _ => CapabilityDomain::from_label(&lower).map(CapabilityDomain::as_str),
    }
}

/// Pure logic: resolve runtime dir from environment snapshot.
///
/// Resolution tiers:
/// 1. `XDG_RUNTIME_DIR` (XDG standard — `/run/user/<uid>`)
/// 2. `/run/membrane/<user>` when running under systemd (`INVOCATION_ID` set) —
///    avoids `/tmp` which is blocked by `ProtectSystem=strict` (DH-1)
/// 3. `temp_dir()/toadstool-runtime-<user>` (development / non-systemd fallback)
#[must_use]
pub fn resolve_runtime_dir(env: &SocketPathEnv) -> String {
    if let Some(ref xdg) = env.xdg_runtime_dir {
        return xdg.clone();
    }

    let username = env.user.as_deref().unwrap_or("default");

    if env.invocation_id.is_some() {
        return format!("/run/membrane/{username}");
    }

    let temp_dir = std::env::temp_dir();
    temp_dir
        .join(format!("toadstool-runtime-{username}"))
        .to_string_lossy()
        .to_string()
}

/// Pure logic: resolve biomeos dir.
///
/// Resolution: `BIOMEOS_SOCKET_DIR` > `{runtime_dir}/biomeos/`
#[must_use]
pub fn resolve_biomeos_dir(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref dir) = env.biomeos_socket_dir {
        return PathBuf::from(dir);
    }
    PathBuf::from(resolve_runtime_dir(env)).join("biomeos")
}

/// Pure logic: resolve family ID from environment snapshot
#[must_use]
pub fn resolve_family_id(env: &SocketPathEnv) -> String {
    env.biomeos_family_id
        .clone()
        .unwrap_or_else(|| "default".to_string())
}

/// BTSP Protocol Standard §Compliance: validate insecure guard.
///
/// When `FAMILY_ID` is set (non-"default") **and** `BIOMEOS_INSECURE=1`,
/// the primal **must refuse to start**. This combination is contradictory:
/// `FAMILY_ID` activates production-mode BTSP handshake, while
/// `BIOMEOS_INSECURE` disables all security. Allowing both would create
/// a false sense of security.
///
/// Returns `Ok(())` if the environment is consistent, or `Err(message)`
/// describing the conflict.
pub fn validate_insecure_guard(env: &SocketPathEnv) -> Result<(), String> {
    let family_id = resolve_family_id(env);
    let is_production_family = family_id != "default";

    let is_insecure = env
        .biomeos_insecure
        .as_deref()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    if is_production_family && is_insecure {
        return Err(format!(
            "BTSP security conflict: FAMILY_ID={family_id:?} (production) \
             with BIOMEOS_INSECURE=1 (development). These are mutually exclusive. \
             Either unset BIOMEOS_INSECURE for production, or unset FAMILY_ID for development. \
             See ecoPrimals/infra/wateringHole/BTSP_PROTOCOL_STANDARD.md §Compliance Checklist."
        ));
    }

    Ok(())
}

/// Returns `true` when `FAMILY_ID` is set to a non-default value,
/// indicating BTSP handshake is expected on incoming connections.
#[must_use]
pub fn is_btsp_required(env: &SocketPathEnv) -> bool {
    let family_id = resolve_family_id(env);
    family_id != "default"
}

/// If `hint` is `unix:///path` or an absolute filesystem path, returns that path for IPC.
/// HTTP(S) URLs return [`None`] — those are not local socket paths.
#[must_use]
pub fn unix_path_from_connection_hint(hint: &str) -> Option<PathBuf> {
    let t = hint.trim();
    if let Some(rest) = t.strip_prefix("unix://") {
        if !rest.is_empty() {
            return Some(PathBuf::from(rest));
        }
        return None;
    }
    if t.starts_with('/') && !t.contains("://") {
        return Some(PathBuf::from(t));
    }
    None
}

fn connection_hint_for_capability<'a>(cap: &str, env: &'a SocketPathEnv) -> Option<&'a str> {
    match cap {
        "crypto" | "security" => env.security_connection_hint.as_deref(),
        "coordination" => env.coordination_connection_hint.as_deref(),
        "storage" => env.storage_connection_hint.as_deref(),
        "routing" | "intelligence" | "ai" => env.routing_connection_hint.as_deref(),
        _ => None,
    }
}

/// Pure logic: resolve socket path for a capability id (`crypto`, `coordination`, …).
///
/// ## Capability mapping (PRIMAL_SELF_KNOWLEDGE_STANDARD)
///
/// | Capability id | Domain | Legacy identity env (socket / endpoint fallbacks) |
/// | --- | --- | --- |
/// | `crypto`, `security` | Security / auth | `BEARDOG_*` |
/// | `coordination` | Registry / mesh | `SONGBIRD_*` |
/// | `storage` | Object / artifact storage | `NESTGATE_*` |
/// | `routing`, `intelligence`, `ai` | AI / MCP-style IPC | `SQUIRREL_*` |
///
/// Precedence: `DISCOVERY_SOCKET` (coordination/discovery only) →
/// `BIOMEOS_{CAP}_SOCKET` → `TOADSTOOL_*_SOCKET` / legacy `*_SOCKET` →
/// connection hints that resolve to Unix paths (`TOADSTOOL_*_ENDPOINT`, `*_ENDPOINT`, legacy `*_URL`) →
/// `{capability}.sock` under the biomeOS runtime directory (never product-code filenames).
#[must_use]
pub fn resolve_capability_socket_fallback(capability: &str, env: &SocketPathEnv) -> PathBuf {
    let cap = capability.to_lowercase();
    let cap = cap.as_str();

    // Highest precedence: DISCOVERY_SOCKET (set by composition_nucleus.sh → Songbird).
    if matches!(cap, "coordination" | "discovery")
        && let Some(ref p) = env.discovery_socket
    {
        return PathBuf::from(p);
    }

    if let Some(p) = match cap {
        "crypto" | "security" => env.biomeos_crypto_socket.as_ref(),
        "coordination" | "discovery" => env.biomeos_coordination_socket.as_ref(),
        "storage" => env.biomeos_storage_socket.as_ref(),
        "routing" | "intelligence" | "ai" => env.biomeos_routing_socket.as_ref(),
        _ => None,
    } {
        return PathBuf::from(p);
    }

    // Legacy env field names map crypto→security, coordination, storage, intelligence sockets.
    if let Some(p) = match cap {
        "crypto" | "security" => env.legacy_security_socket.as_ref(), // legacy: BEARDOG_SOCKET
        "coordination" | "discovery" => env.legacy_coordination_socket.as_ref(), // legacy: SONGBIRD_SOCKET
        "storage" => env.legacy_storage_socket.as_ref(), // legacy: NESTGATE_SOCKET
        "routing" | "intelligence" | "ai" => env.legacy_intelligence_socket.as_ref(), // legacy: SQUIRREL_SOCKET
        _ => None,
    } {
        return PathBuf::from(p);
    }

    if let Some(hint) = connection_hint_for_capability(cap, env)
        && let Some(p) = unix_path_from_connection_hint(hint)
    {
        return p;
    }

    resolve_biomeos_dir(env).join(format!("{cap}.sock"))
}

/// Pure logic: resolve routing capability socket (non-discovery path).
#[must_use]
pub fn resolve_routing_socket(env: &SocketPathEnv) -> PathBuf {
    resolve_capability_socket_fallback("routing", env)
}

/// Pure logic: resolve nucleus socket
#[must_use]
pub fn resolve_nucleus_socket(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.nucleus_socket {
        return PathBuf::from(socket);
    }
    if let Some(ref socket) = env.biomeos_socket_path {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join("nucleus.sock")
}

/// Pure logic: resolve toadstool JSON-RPC socket (primary protocol)
#[must_use]
pub fn resolve_toadstool_socket(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.toadstool_socket {
        return PathBuf::from(socket);
    }
    if let Some(ref socket) = env.biomeos_socket_path {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join(SELF_SOCKET)
}

/// Pure logic: resolve toadstool tarpc socket (secondary, hot-path protocol).
///
/// C2 dual-socket pattern: tarpc on `compute.tarpc.sock` (or
/// `compute-{family_id}.tarpc.sock` for non-default families).
#[must_use]
pub fn resolve_toadstool_tarpc_socket(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.toadstool_tarpc_socket {
        return PathBuf::from(socket);
    }
    let family_id = resolve_family_id(env);
    let domain = crate::constants::primal_identity::CAPABILITY_DOMAIN;
    let filename = if family_id.is_empty() || family_id == "default" {
        format!("{domain}.tarpc.sock")
    } else {
        format!("{domain}-{family_id}.tarpc.sock")
    };
    resolve_biomeos_dir(env).join(filename)
}

/// Pure logic: resolve socket path for any service label (capability id or legacy primal alias).
#[must_use]
pub fn resolve_socket_path_for_service(
    service_name: &str,
    env: &SocketPathEnv,
    service_socket_override: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = service_socket_override {
        return path;
    }
    let lower = service_name.to_lowercase();
    let s = lower.as_str();

    if let Some(cap) = service_label_to_capability_id(s) {
        return resolve_capability_socket_fallback(cap, env);
    }

    match s {
        "toadstool" | "toad-stool" => resolve_toadstool_socket(env),
        "nucleus" | "biomeos" => resolve_nucleus_socket(env),
        _ => resolve_biomeos_dir(env).join(format!("{s}.sock")),
    }
}
