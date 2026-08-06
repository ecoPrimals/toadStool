// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from paths.rs (S333).

use super::env::SocketPathEnv;
use super::paths::*;
use std::path::PathBuf;

fn test_env() -> SocketPathEnv {
    SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        user: Some("testuser".to_string()),
        ..Default::default()
    }
}

#[test]
fn test_resolve_runtime_dir_xdg_takes_precedence() {
    let env = test_env();
    assert_eq!(resolve_runtime_dir(&env), "/run/user/1000");
}

#[test]
fn test_resolve_runtime_dir_no_xdg_uses_temp_and_user() {
    let env = SocketPathEnv {
        xdg_runtime_dir: None,
        user: Some("alice".to_string()),
        ..Default::default()
    };
    let dir = resolve_runtime_dir(&env);
    assert!(dir.contains("toadstool-runtime-alice"));
}

#[test]
fn test_resolve_runtime_dir_no_user_uses_default() {
    let env = SocketPathEnv {
        xdg_runtime_dir: None,
        user: None,
        ..Default::default()
    };
    let dir = resolve_runtime_dir(&env);
    assert!(dir.contains("toadstool-runtime-default"));
}

#[test]
fn test_resolve_runtime_dir_systemd_uses_run_membrane() {
    let env = SocketPathEnv {
        xdg_runtime_dir: None,
        user: Some("ecoprimals".to_string()),
        invocation_id: Some("abc123".to_string()),
        ..Default::default()
    };
    let dir = resolve_runtime_dir(&env);
    assert_eq!(dir, "/run/membrane/ecoprimals");
}

#[test]
fn test_resolve_runtime_dir_xdg_wins_over_systemd() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        invocation_id: Some("abc123".to_string()),
        user: Some("testuser".to_string()),
        ..Default::default()
    };
    let dir = resolve_runtime_dir(&env);
    assert_eq!(dir, "/run/user/1000");
}

#[test]
fn test_resolve_runtime_dir_systemd_default_user() {
    let env = SocketPathEnv {
        xdg_runtime_dir: None,
        user: None,
        invocation_id: Some("def456".to_string()),
        ..Default::default()
    };
    let dir = resolve_runtime_dir(&env);
    assert_eq!(dir, "/run/membrane/default");
}

#[test]
fn test_resolve_biomeos_dir_under_systemd() {
    let env = SocketPathEnv {
        xdg_runtime_dir: None,
        user: Some("svc".to_string()),
        invocation_id: Some("xyz".to_string()),
        ..Default::default()
    };
    let dir = resolve_biomeos_dir(&env);
    assert_eq!(dir, PathBuf::from("/run/membrane/svc/biomeos"));
}

#[test]
fn test_resolve_nucleus_socket_prefers_nucleus_over_biomeos_path() {
    let env = SocketPathEnv {
        nucleus_socket: Some("/custom/nucleus.sock".to_string()),
        biomeos_socket_path: Some("/var/run/biomeos.sock".to_string()),
        ..test_env()
    };
    let path = resolve_nucleus_socket(&env);
    assert_eq!(path, PathBuf::from("/custom/nucleus.sock"));
}

#[test]
fn test_resolve_toadstool_socket_prefers_toadstool_over_biomeos_path() {
    let env = SocketPathEnv {
        toadstool_socket: Some("/custom/toad.sock".to_string()),
        biomeos_socket_path: Some("/var/run/biomeos.sock".to_string()),
        ..test_env()
    };
    let path = resolve_toadstool_socket(&env);
    assert_eq!(path, PathBuf::from("/custom/toad.sock"));
}

#[test]
fn test_resolve_socket_path_override_takes_precedence() {
    let env = test_env();
    let override_path = PathBuf::from("/override/custom.sock");
    // legacy orchestrator label still resolves via CapabilityDomain
    let result = resolve_socket_path_for_service("beardog", &env, Some(override_path.clone()));
    assert_eq!(result, override_path);
}

#[test]
fn test_resolve_crypto_socket_with_legacy_env_override() {
    let env = SocketPathEnv {
        legacy_security_socket: Some("/custom/crypto-via-legacy.sock".to_string()),
        ..test_env()
    };
    let path = resolve_capability_socket_fallback("crypto", &env);
    assert_eq!(path, PathBuf::from("/custom/crypto-via-legacy.sock"));
}

#[test]
fn test_resolve_coordination_socket_with_legacy_env_override() {
    let env = SocketPathEnv {
        legacy_coordination_socket: Some("/custom/coordination-via-legacy.sock".to_string()),
        ..test_env()
    };
    let path = resolve_capability_socket_fallback("coordination", &env);
    assert_eq!(path, PathBuf::from("/custom/coordination-via-legacy.sock"));
}

#[test]
fn test_resolve_storage_socket_with_legacy_env_override() {
    let env = SocketPathEnv {
        legacy_storage_socket: Some("/custom/storage-via-legacy.sock".to_string()),
        ..test_env()
    };
    let path = resolve_capability_socket_fallback("storage", &env);
    assert_eq!(path, PathBuf::from("/custom/storage-via-legacy.sock"));
}

#[test]
fn test_resolve_routing_socket_with_legacy_env_override() {
    let env = SocketPathEnv {
        legacy_intelligence_socket: Some("/custom/intelligence-via-legacy.sock".to_string()),
        ..test_env()
    };
    let path = resolve_routing_socket(&env);
    assert_eq!(path, PathBuf::from("/custom/intelligence-via-legacy.sock"));
}

#[test]
fn test_resolve_nucleus_socket_uses_biomeos_fallback() {
    let env = SocketPathEnv {
        nucleus_socket: None,
        biomeos_socket_path: None,
        ..test_env()
    };
    let path = resolve_nucleus_socket(&env);
    assert!(path.to_string_lossy().contains("nucleus"));
    assert!(path.to_string_lossy().ends_with("nucleus.sock"));
}

#[test]
fn test_resolve_toadstool_socket_uses_biomeos_fallback() {
    let env = SocketPathEnv {
        toadstool_socket: None,
        biomeos_socket_path: None,
        ..test_env()
    };
    let path = resolve_toadstool_socket(&env);
    assert!(
        path.to_string_lossy().ends_with("compute.sock"),
        "Self-Knowledge v1.1: domain-based socket name, got: {}",
        path.display()
    );
}

#[test]
fn test_resolve_toadstool_tarpc_socket_override() {
    let env = SocketPathEnv {
        toadstool_tarpc_socket: Some("/custom/tarpc.sock".to_string()),
        ..test_env()
    };
    let path = resolve_toadstool_tarpc_socket(&env);
    assert_eq!(path, PathBuf::from("/custom/tarpc.sock"));
}

#[test]
fn test_resolve_toadstool_tarpc_socket_default_family() {
    let env = SocketPathEnv {
        toadstool_tarpc_socket: None,
        biomeos_family_id: None,
        ..test_env()
    };
    let path = resolve_toadstool_tarpc_socket(&env);
    assert!(
        path.to_string_lossy().ends_with("compute.tarpc.sock"),
        "expected compute.tarpc.sock, got: {}",
        path.display()
    );
}

#[test]
fn test_resolve_toadstool_tarpc_socket_custom_family() {
    let env = SocketPathEnv {
        toadstool_tarpc_socket: None,
        biomeos_family_id: Some("nat0".to_string()),
        ..test_env()
    };
    let path = resolve_toadstool_tarpc_socket(&env);
    assert!(
        path.to_string_lossy().ends_with("compute-nat0.tarpc.sock"),
        "expected compute-nat0.tarpc.sock, got: {}",
        path.display()
    );
}

#[test]
fn test_resolve_socket_path_unknown_service() {
    let env = test_env();
    let path = resolve_socket_path_for_service("unknown_service", &env, None);
    assert!(path.to_string_lossy().contains("unknown_service"));
    assert!(path.to_string_lossy().ends_with("unknown_service.sock"));
}

#[test]
fn test_resolve_socket_path_capability_and_legacy_aliases() {
    let env = test_env();
    let bear_dog_aliased = resolve_socket_path_for_service("bear-dog", &env, None);
    let crypto = resolve_socket_path_for_service("crypto", &env, None);
    assert_eq!(bear_dog_aliased, crypto);
    assert!(crypto.to_string_lossy().ends_with("crypto.sock"));

    let path = resolve_socket_path_for_service("toad-stool", &env, None);
    assert!(path.to_string_lossy().ends_with("compute.sock"));
}

#[test]
fn test_resolve_biomeos_dir() {
    let env = test_env();
    let path = resolve_biomeos_dir(&env);
    assert!(path.to_string_lossy().contains("biomeos"));
}

#[test]
fn test_resolve_family_id_default() {
    let env = SocketPathEnv {
        biomeos_family_id: None,
        ..test_env()
    };
    assert_eq!(resolve_family_id(&env), "default");
}

#[test]
fn test_resolve_family_id_from_env() {
    let env = SocketPathEnv {
        biomeos_family_id: Some("custom-family".to_string()),
        ..test_env()
    };
    assert_eq!(resolve_family_id(&env), "custom-family");
}

#[test]
fn test_biomeos_crypto_socket_precedence_over_legacy() {
    let env = SocketPathEnv {
        biomeos_crypto_socket: Some("/via/biomeos/crypto.sock".to_string()),
        legacy_security_socket: Some("/legacy/crypto-fallback.sock".to_string()),
        ..test_env()
    };
    let path = resolve_capability_socket_fallback("crypto", &env);
    assert_eq!(path, PathBuf::from("/via/biomeos/crypto.sock"));
}

#[test]
fn test_unix_path_from_connection_hint() {
    assert_eq!(
        unix_path_from_connection_hint("unix:///run/crypto.sock"),
        Some(PathBuf::from("/run/crypto.sock"))
    );
    assert_eq!(
        unix_path_from_connection_hint("/var/run/coordination.sock"),
        Some(PathBuf::from("/var/run/coordination.sock"))
    );
    assert!(unix_path_from_connection_hint("http://localhost:8080").is_none());
}

#[test]
fn test_resolve_crypto_socket_from_legacy_endpoint_unix_scheme() {
    let env = SocketPathEnv {
        security_connection_hint: Some("unix:///custom/from-endpoint.sock".to_string()),
        ..test_env()
    };
    let path = resolve_capability_socket_fallback("crypto", &env);
    assert_eq!(path, PathBuf::from("/custom/from-endpoint.sock"));
}
