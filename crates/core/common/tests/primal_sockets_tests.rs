// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for primal socket path resolution
//!
//! Extracted from `primal_sockets.rs` to reduce file size.

use std::path::PathBuf;
use toadstool_common::primal_sockets::*;

/// Mutex to serialize tests that modify environment variables.
#[expect(dead_code)]
static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_runtime_dir_from_xdg() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    assert_eq!(resolve_runtime_dir(&env), "/run/user/1000");
}

#[test]
fn test_runtime_dir_fallback() {
    let env = SocketPathEnv {
        user: Some("testuser".to_string()),
        ..Default::default()
    };
    let runtime_dir = resolve_runtime_dir(&env);
    // Could be /run/user/<uid> if exists, or fallback
    assert!(
        runtime_dir.starts_with("/run/user/") || runtime_dir == "/tmp/toadstool-runtime-testuser",
        "Expected /run/user/<uid> or /tmp fallback, got: {runtime_dir}"
    );
}

#[test]
fn test_security_socket_legacy_from_env() {
    let env = SocketPathEnv {
        legacy_security_socket: Some("/custom/security-via-legacy.sock".to_string()),
        ..Default::default()
    };
    assert_eq!(
        resolve_capability_socket_fallback("crypto", &env),
        PathBuf::from("/custom/security-via-legacy.sock")
    );
}

#[test]
fn test_security_socket_default_biomeos_path() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        biomeos_family_id: Some("nat0".to_string()),
        ..Default::default()
    };
    let path = resolve_capability_socket_fallback("crypto", &env);
    assert_eq!(path, PathBuf::from("/run/user/1000/biomeos/crypto.sock"));
}

#[test]
fn test_coordination_socket_biomeos_standard() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let path = resolve_capability_socket_fallback("coordination", &env);
    assert_eq!(
        path,
        PathBuf::from("/run/user/1000/biomeos/coordination.sock")
    );
}

#[test]
fn test_toadstool_socket_biomeos_standard() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let path = resolve_toadstool_socket(&env);
    assert_eq!(path, PathBuf::from("/run/user/1000/biomeos/compute.sock"));
}

#[test]
fn test_biomeos_directory_path() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    assert_eq!(
        resolve_biomeos_dir(&env),
        PathBuf::from("/run/user/1000/biomeos")
    );
}

#[test]
fn test_all_capability_sockets_have_unique_paths() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let security = resolve_capability_socket_fallback("crypto", &env);
    let coordination = resolve_capability_socket_fallback("coordination", &env);
    let storage = resolve_capability_socket_fallback("storage", &env);
    let routing = resolve_routing_socket(&env);
    let toadstool = resolve_toadstool_socket(&env);

    assert_ne!(security, coordination);
    assert_ne!(security, storage);
    assert_ne!(security, routing);
    assert_ne!(security, toadstool);
    assert_ne!(coordination, storage);
    assert_ne!(coordination, routing);
    assert_ne!(coordination, toadstool);
    assert_ne!(storage, routing);
    assert_ne!(storage, toadstool);
    assert_ne!(routing, toadstool);

    // All should be in biomeos subdirectory
    for path in [&security, &coordination, &storage, &routing, &toadstool] {
        assert!(
            path.to_str().unwrap().contains("/biomeos/"),
            "Path should contain /biomeos/: {}",
            path.display()
        );
    }
}

#[test]
fn test_family_id_from_biomeos() {
    let env = SocketPathEnv {
        biomeos_family_id: Some("test-family".to_string()),
        ..Default::default()
    };
    assert_eq!(resolve_family_id(&env), "test-family");
}

#[test]
fn test_family_id_default() {
    let env = SocketPathEnv::default();
    assert_eq!(resolve_family_id(&env), "default");
}

#[test]
fn test_socket_path_for_service_known() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let path = resolve_socket_path_for_service("toadstool", &env, None);
    assert_eq!(path, PathBuf::from("/run/user/1000/biomeos/compute.sock"));
}

#[test]
fn test_socket_path_for_service_unknown() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let path = resolve_socket_path_for_service("myservice", &env, None);
    assert_eq!(path, PathBuf::from("/run/user/1000/biomeos/myservice.sock"));
}

#[test]
fn test_socket_path_for_service_with_override() {
    let env = SocketPathEnv::default();
    let override_path = PathBuf::from("/custom/override.sock");
    let path = resolve_socket_path_for_service("beardog", &env, Some(override_path.clone()));
    assert_eq!(path, override_path);
}

#[test]
fn test_storage_socket_legacy_from_env() {
    let env = SocketPathEnv {
        legacy_storage_socket: Some("/custom/storage-via-legacy.sock".to_string()),
        ..Default::default()
    };
    assert_eq!(
        resolve_capability_socket_fallback("storage", &env),
        PathBuf::from("/custom/storage-via-legacy.sock")
    );
}

#[test]
fn test_storage_socket_default_biomeos_path() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let path = resolve_capability_socket_fallback("storage", &env);
    assert_eq!(path, PathBuf::from("/run/user/1000/biomeos/storage.sock"));
}

#[test]
fn test_intelligence_socket_legacy_from_env() {
    let env = SocketPathEnv {
        legacy_intelligence_socket: Some("/custom/intelligence-via-legacy.sock".to_string()),
        ..Default::default()
    };
    assert_eq!(
        resolve_routing_socket(&env),
        PathBuf::from("/custom/intelligence-via-legacy.sock")
    );
}

#[test]
fn test_routing_socket_default_biomeos_path() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let path = resolve_routing_socket(&env);
    assert_eq!(path, PathBuf::from("/run/user/1000/biomeos/routing.sock"));
}

#[test]
fn test_nucleus_socket_from_env() {
    let env = SocketPathEnv {
        nucleus_socket: Some("/custom/nucleus.sock".to_string()),
        ..Default::default()
    };
    assert_eq!(
        resolve_nucleus_socket(&env),
        PathBuf::from("/custom/nucleus.sock")
    );
}

#[test]
fn test_nucleus_socket_from_biomeos_path() {
    let env = SocketPathEnv {
        biomeos_socket_path: Some("/custom/biomeos.sock".to_string()),
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let path = resolve_nucleus_socket(&env);
    assert_eq!(path, PathBuf::from("/custom/biomeos.sock"));
}

#[test]
fn test_nucleus_socket_default() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let path = resolve_nucleus_socket(&env);
    assert_eq!(path, PathBuf::from("/run/user/1000/biomeos/nucleus.sock"));
}

#[test]
fn test_toadstool_socket_from_env() {
    let env = SocketPathEnv {
        toadstool_socket: Some("/custom/toadstool.sock".to_string()),
        ..Default::default()
    };
    assert_eq!(
        resolve_toadstool_socket(&env),
        PathBuf::from("/custom/toadstool.sock")
    );
}

#[test]
fn test_toadstool_socket_from_biomeos_path() {
    let env = SocketPathEnv {
        biomeos_socket_path: Some("/custom/biomeos.sock".to_string()),
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let path = resolve_toadstool_socket(&env);
    assert_eq!(path, PathBuf::from("/custom/biomeos.sock"));
}

#[test]
fn test_socket_path_for_service_aliases() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    assert_eq!(
        resolve_socket_path_for_service("bear-dog", &env, None),
        resolve_capability_socket_fallback("crypto", &env)
    );
    assert_eq!(
        resolve_socket_path_for_service("song-bird", &env, None),
        resolve_capability_socket_fallback("coordination", &env)
    );
    assert_eq!(
        resolve_socket_path_for_service("nest-gate", &env, None),
        resolve_capability_socket_fallback("storage", &env)
    );
    assert_eq!(
        resolve_socket_path_for_service("toad-stool", &env, None),
        resolve_toadstool_socket(&env)
    );
    assert_eq!(
        resolve_socket_path_for_service("nucleus", &env, None),
        resolve_nucleus_socket(&env)
    );
    assert_eq!(
        resolve_socket_path_for_service("biomeos", &env, None),
        resolve_nucleus_socket(&env)
    );
}

#[test]
fn test_socket_path_service_name_lowercase() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let path = resolve_socket_path_for_service("BearDog", &env, None);
    assert_eq!(path, PathBuf::from("/run/user/1000/biomeos/crypto.sock"));
}

#[test]
fn test_socket_path_env_default() {
    let env = SocketPathEnv::default();
    assert!(env.xdg_runtime_dir.is_none());
    assert!(env.user.is_none());
    assert!(env.legacy_security_socket.is_none());
}

#[test]
fn test_socket_path_env_with_runtime_dir() {
    let env = SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/42".to_string()),
        ..Default::default()
    };
    assert_eq!(env.xdg_runtime_dir.as_deref(), Some("/run/user/42"));
    assert_eq!(resolve_runtime_dir(&env), "/run/user/42");
}

#[test]
fn test_socket_discovery_error_display() {
    let err = SocketDiscoveryError::DiscoveryFailed("test".to_string());
    assert!(err.to_string().contains("Discovery"));
    assert!(err.to_string().contains("test"));

    let err = SocketDiscoveryError::NoSocketFound("crypto".to_string());
    assert!(err.to_string().contains("No Unix socket"));
    assert!(err.to_string().contains("crypto"));

    let err = SocketDiscoveryError::InvalidEndpoint("bad".to_string());
    assert!(err.to_string().contains("Invalid"));
    assert!(err.to_string().contains("bad"));
}

#[test]
fn test_get_runtime_dir() {
    let dir = get_runtime_dir();
    assert!(
        dir.starts_with("/run/user/") || dir.starts_with("/tmp/toadstool-runtime-"),
        "Unexpected runtime dir: {dir}"
    );
}

#[test]
fn test_get_biomeos_dir() {
    let path = get_biomeos_dir();
    assert!(path.to_str().unwrap().ends_with("/biomeos"));
}

#[test]
fn test_get_family_id() {
    let id = get_family_id();
    assert!(!id.is_empty());
}

#[test]
fn test_get_routing_socket_path() {
    let path = get_routing_socket_path();
    assert!(path.to_str().unwrap().contains(".sock"));
}

#[test]
fn test_get_nucleus_socket_path() {
    let path = get_nucleus_socket_path();
    assert!(path.to_str().unwrap().contains(".sock"));
}

#[test]
fn test_get_toadstool_socket_path() {
    let path = get_toadstool_socket_path();
    assert!(
        path.to_str().unwrap().contains("compute"),
        "Self-Knowledge v1.1: domain-based name, got: {}",
        path.display()
    );
    assert!(path.to_str().unwrap().contains(".sock"));
}
