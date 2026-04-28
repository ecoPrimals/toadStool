// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primal sockets tests

use std::path::PathBuf;

use super::*;

fn test_env() -> SocketPathEnv {
    SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        user: Some("testuser".to_string()),
        ..Default::default()
    }
}

#[test]
fn socket_path_env_default_is_all_none() {
    let env = SocketPathEnv::default();
    assert!(env.xdg_runtime_dir.is_none());
    assert!(env.user.is_none());
    assert!(env.biomeos_family_id.is_none());
    assert!(env.legacy_security_socket.is_none());
    assert!(env.legacy_coordination_socket.is_none());
    assert!(env.toadstool_socket.is_none());
}

#[test]
fn socket_path_env_with_runtime_dir() {
    let env = SocketPathEnv::with_runtime_dir("/tmp/test-runtime");
    assert_eq!(env.xdg_runtime_dir, Some("/tmp/test-runtime".to_string()));
    assert!(env.user.is_none());
}

#[test]
fn socket_path_env_from_env_captures_vars() {
    let env = SocketPathEnv::from_env();
    let _ = format!("{env:?}");
}

#[test]
fn resolve_runtime_dir_uses_xdg_when_set() {
    let env = test_env();
    assert_eq!(resolve_runtime_dir(&env), "/run/user/1000");
}

#[test]
fn resolve_runtime_dir_falls_back_to_tmp_with_username() {
    let env = SocketPathEnv {
        xdg_runtime_dir: None,
        user: Some("alice".to_string()),
        ..Default::default()
    };
    let dir = resolve_runtime_dir(&env);
    assert!(dir.contains("alice") || dir.starts_with("/run/user/"));
}

#[test]
fn resolve_runtime_dir_falls_back_to_default_user() {
    let env = SocketPathEnv::default();
    let dir = resolve_runtime_dir(&env);
    assert!(!dir.is_empty());
}

#[test]
fn resolve_biomeos_dir_appends_biomeos() {
    let env = test_env();
    let dir = resolve_biomeos_dir(&env);
    assert_eq!(dir, PathBuf::from("/run/user/1000/biomeos"));
}

#[test]
fn resolve_family_id_uses_env_value() {
    let env = SocketPathEnv {
        biomeos_family_id: Some("my-family".to_string()),
        ..test_env()
    };
    assert_eq!(resolve_family_id(&env), "my-family");
}

#[test]
fn resolve_family_id_defaults_to_default() {
    let env = test_env();
    assert_eq!(resolve_family_id(&env), "default");
}

#[test]
fn resolve_security_socket_legacy_env_override() {
    let env = SocketPathEnv {
        legacy_security_socket: Some("/custom/security-via-legacy.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_capability_socket_fallback("crypto", &env),
        PathBuf::from("/custom/security-via-legacy.sock")
    );
}

#[test]
fn resolve_security_socket_uses_biomeos_fallback() {
    let env = test_env();
    assert_eq!(
        resolve_capability_socket_fallback("crypto", &env),
        PathBuf::from("/run/user/1000/biomeos/crypto.sock")
    );
}

#[test]
fn resolve_coordination_socket_legacy_env_override() {
    let env = SocketPathEnv {
        legacy_coordination_socket: Some("/custom/coordination-via-legacy.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_capability_socket_fallback("coordination", &env),
        PathBuf::from("/custom/coordination-via-legacy.sock")
    );
}

#[test]
fn resolve_coordination_socket_uses_biomeos_fallback() {
    let env = test_env();
    assert_eq!(
        resolve_capability_socket_fallback("coordination", &env),
        PathBuf::from("/run/user/1000/biomeos/coordination.sock")
    );
}

#[test]
fn resolve_storage_socket_legacy_env_override() {
    let env = SocketPathEnv {
        legacy_storage_socket: Some("/custom/storage-via-legacy.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_capability_socket_fallback("storage", &env),
        PathBuf::from("/custom/storage-via-legacy.sock")
    );
}

#[test]
fn resolve_storage_socket_uses_biomeos_fallback() {
    let env = test_env();
    assert_eq!(
        resolve_capability_socket_fallback("storage", &env),
        PathBuf::from("/run/user/1000/biomeos/storage.sock")
    );
}

#[test]
fn resolve_intelligence_socket_legacy_env_override() {
    let env = SocketPathEnv {
        legacy_intelligence_socket: Some("/custom/intelligence-via-legacy.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_routing_socket(&env),
        PathBuf::from("/custom/intelligence-via-legacy.sock")
    );
}

#[test]
fn resolve_intelligence_socket_uses_biomeos_fallback() {
    let env = test_env();
    assert_eq!(
        resolve_routing_socket(&env),
        PathBuf::from("/run/user/1000/biomeos/routing.sock")
    );
}

#[test]
fn resolve_nucleus_socket_uses_env_override() {
    let env = SocketPathEnv {
        nucleus_socket: Some("/custom/nucleus.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_nucleus_socket(&env),
        PathBuf::from("/custom/nucleus.sock")
    );
}

#[test]
fn resolve_nucleus_socket_uses_biomeos_socket_path() {
    let env = SocketPathEnv {
        biomeos_socket_path: Some("/var/run/biomeos.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_nucleus_socket(&env),
        PathBuf::from("/var/run/biomeos.sock")
    );
}

#[test]
fn resolve_nucleus_socket_uses_biomeos_fallback() {
    let env = test_env();
    assert_eq!(
        resolve_nucleus_socket(&env),
        PathBuf::from("/run/user/1000/biomeos/nucleus.sock")
    );
}

#[test]
fn resolve_toadstool_socket_uses_env_override() {
    let env = SocketPathEnv {
        toadstool_socket: Some("/custom/toadstool.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_toadstool_socket(&env),
        PathBuf::from("/custom/toadstool.sock")
    );
}

#[test]
fn resolve_toadstool_socket_uses_biomeos_socket_path() {
    let env = SocketPathEnv {
        biomeos_socket_path: Some("/var/run/toadstool.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_toadstool_socket(&env),
        PathBuf::from("/var/run/toadstool.sock")
    );
}

#[test]
fn resolve_toadstool_socket_uses_biomeos_fallback() {
    let env = test_env();
    assert_eq!(
        resolve_toadstool_socket(&env),
        PathBuf::from("/run/user/1000/biomeos/compute.sock")
    );
}

#[test]
fn resolve_service_socket_override_takes_precedence() {
    let env = test_env();
    let override_path = PathBuf::from("/override/custom.sock");
    // legacy route label still resolves via CapabilityDomain
    let result = resolve_socket_path_for_service("beardog", &env, Some(override_path.clone()));
    assert_eq!(result, override_path);
}

#[test]
fn resolve_service_socket_legacy_security_label() {
    let env = test_env();
    let result = resolve_socket_path_for_service("beardog", &env, None);
    assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/crypto.sock"));
}

#[test]
fn resolve_service_socket_security_alias_bear_dog() {
    let env = test_env();
    let result = resolve_socket_path_for_service("bear-dog", &env, None);
    assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/crypto.sock"));
}

#[test]
fn resolve_service_socket_legacy_coordination_label() {
    let env = test_env();
    let result = resolve_socket_path_for_service("songbird", &env, None);
    assert_eq!(
        result,
        PathBuf::from("/run/user/1000/biomeos/coordination.sock")
    );
}

#[test]
fn resolve_service_socket_toadstool() {
    let env = test_env();
    let result = resolve_socket_path_for_service("toadstool", &env, None);
    assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/compute.sock"));
}

#[test]
fn resolve_service_socket_nucleus() {
    let env = test_env();
    let result = resolve_socket_path_for_service("nucleus", &env, None);
    assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/nucleus.sock"));
}

#[test]
fn resolve_service_socket_unknown_falls_through() {
    let env = test_env();
    let result = resolve_socket_path_for_service("myservice", &env, None);
    assert_eq!(
        result,
        PathBuf::from("/run/user/1000/biomeos/myservice.sock")
    );
}

#[test]
fn resolve_service_socket_case_insensitive() {
    let env = test_env();
    let result = resolve_socket_path_for_service("BearDog", &env, None);
    assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/crypto.sock"));
}

#[test]
fn get_runtime_dir_returns_valid_path() {
    let dir = get_runtime_dir();
    assert!(!dir.is_empty());
}

#[test]
fn get_biomeos_dir_contains_biomeos() {
    let dir = get_biomeos_dir();
    assert!(dir.to_string_lossy().contains("biomeos"));
}

#[test]
fn get_family_id_returns_string() {
    let id = get_family_id();
    assert!(!id.is_empty());
}

#[test]
fn get_socket_path_for_capability_crypto_is_path() {
    let path = get_socket_path_for_capability("crypto");
    assert!(path.to_string_lossy().contains("crypto"));
}

#[test]
fn get_socket_path_for_capability_coordination_is_path() {
    let path = get_socket_path_for_capability("coordination");
    assert!(path.to_string_lossy().contains("coordination"));
}

#[test]
fn get_socket_path_for_capability_storage_is_path() {
    let path = get_socket_path_for_capability("storage");
    assert!(path.to_string_lossy().contains("storage"));
}

#[test]
fn get_routing_socket_path_is_path() {
    let path = get_routing_socket_path();
    assert!(path.to_string_lossy().contains("routing"));
}

#[test]
fn get_toadstool_socket_path_is_path() {
    let path = get_toadstool_socket_path();
    assert!(
        path.to_string_lossy().contains("compute"),
        "Self-Knowledge v1.1: domain-based name, got: {}",
        path.display()
    );
}

#[test]
fn get_nucleus_socket_path_is_path() {
    let path = get_nucleus_socket_path();
    let s = path.to_string_lossy();
    assert!(s.contains("nucleus") || s.contains("biomeos"));
}

#[test]
fn test_socket_discovery_error_display() {
    let err = SocketDiscoveryError::DiscoveryFailed("test failure".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Discovery") || msg.contains("failed"));

    let err2 = SocketDiscoveryError::NoSocketFound("Capability::Crypto".to_string());
    let msg2 = err2.to_string();
    assert!(msg2.contains("socket") || msg2.contains("Crypto"));

    let err3 = SocketDiscoveryError::InvalidEndpoint("bad path".to_string());
    let msg3 = err3.to_string();
    assert!(msg3.contains("Invalid") || msg3.contains("bad path"));
}

#[test]
fn test_ensure_biomeos_dir_creates_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let runtime_dir = tmp.path().join("run");
    temp_env::with_var(
        "XDG_RUNTIME_DIR",
        Some(runtime_dir.to_str().unwrap()),
        || {
            let result = ensure_biomeos_dir();
            assert!(result.is_ok(), "ensure_biomeos_dir failed: {result:?}");
            let path = result.unwrap();
            assert!(path.to_string_lossy().contains("biomeos"));
            assert!(path.exists());
        },
    );
}

#[test]
fn test_resolve_socket_path_for_service_unknown_with_env_override() {
    let service_name = format!("testsvc_{}", std::process::id());
    let env_var = format!("{}_SOCKET", service_name.to_uppercase().replace('-', "_"));
    let custom_path = "/tmp/custom-test.sock";
    temp_env::with_var(&env_var, Some(custom_path), || {
        let env = SocketPathEnv::from_env();
        let override_path = std::env::var(&env_var).ok().map(PathBuf::from);
        let path = resolve_socket_path_for_service(&service_name, &env, override_path);
        assert_eq!(path.to_string_lossy(), custom_path);
    });
}

#[test]
fn test_resolve_service_socket_empty_service_name() {
    let env = test_env();
    let result = resolve_socket_path_for_service("", &env, None);
    assert!(result.to_string_lossy().ends_with(".sock"));
}

// ── BTSP insecure guard tests ─────────────────────────────────────────────

#[test]
fn insecure_guard_allows_dev_mode_no_family() {
    let env = SocketPathEnv {
        biomeos_insecure: Some("1".to_string()),
        ..test_env()
    };
    assert!(validate_insecure_guard(&env).is_ok());
}

#[test]
fn insecure_guard_allows_production_family_without_insecure() {
    let env = SocketPathEnv {
        biomeos_family_id: Some("nat0".to_string()),
        ..test_env()
    };
    assert!(validate_insecure_guard(&env).is_ok());
}

#[test]
fn insecure_guard_refuses_family_plus_insecure() {
    let env = SocketPathEnv {
        biomeos_family_id: Some("nat0".to_string()),
        biomeos_insecure: Some("1".to_string()),
        ..test_env()
    };
    let err = validate_insecure_guard(&env).unwrap_err();
    assert!(err.contains("BTSP security conflict"), "got: {err}");
    assert!(err.contains("nat0"), "should mention the family ID");
}

#[test]
fn insecure_guard_refuses_family_plus_insecure_true() {
    let env = SocketPathEnv {
        biomeos_family_id: Some("production-1".to_string()),
        biomeos_insecure: Some("true".to_string()),
        ..test_env()
    };
    assert!(validate_insecure_guard(&env).is_err());
}

#[test]
fn insecure_guard_allows_default_family_with_insecure() {
    let env = SocketPathEnv {
        biomeos_family_id: Some("default".to_string()),
        biomeos_insecure: Some("1".to_string()),
        ..test_env()
    };
    assert!(validate_insecure_guard(&env).is_ok());
}

#[test]
fn insecure_guard_allows_no_family_no_insecure() {
    let env = test_env();
    assert!(validate_insecure_guard(&env).is_ok());
}

#[test]
fn is_btsp_required_with_family() {
    let env = SocketPathEnv {
        biomeos_family_id: Some("nat0".to_string()),
        ..test_env()
    };
    assert!(is_btsp_required(&env));
}

#[test]
fn is_btsp_required_without_family() {
    let env = test_env();
    assert!(!is_btsp_required(&env));
}

#[test]
fn is_btsp_required_default_family() {
    let env = SocketPathEnv {
        biomeos_family_id: Some("default".to_string()),
        ..test_env()
    };
    assert!(!is_btsp_required(&env));
}

// ═══════════════════════════════════════════════════════════
// DISCOVERY_SOCKET precedence tests (Phase 55)
// ═══════════════════════════════════════════════════════════

#[test]
fn discovery_socket_takes_precedence_for_coordination() {
    let env = SocketPathEnv {
        discovery_socket: Some("/run/songbird.sock".to_string()),
        biomeos_coordination_socket: Some("/run/biomeos-coord.sock".to_string()),
        legacy_coordination_socket: Some("/run/legacy-coord.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_capability_socket_fallback("coordination", &env),
        PathBuf::from("/run/songbird.sock")
    );
}

#[test]
fn discovery_socket_takes_precedence_for_discovery_capability() {
    let env = SocketPathEnv {
        discovery_socket: Some("/run/songbird.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_capability_socket_fallback("discovery", &env),
        PathBuf::from("/run/songbird.sock")
    );
}

#[test]
fn discovery_socket_absent_falls_through_to_biomeos() {
    let env = SocketPathEnv {
        biomeos_coordination_socket: Some("/run/biomeos-coord.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_capability_socket_fallback("coordination", &env),
        PathBuf::from("/run/biomeos-coord.sock")
    );
}

#[test]
fn discovery_socket_does_not_affect_crypto() {
    let env = SocketPathEnv {
        discovery_socket: Some("/run/songbird.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_capability_socket_fallback("crypto", &env),
        PathBuf::from("/run/user/1000/biomeos/crypto.sock")
    );
}

#[test]
fn discovery_capability_falls_to_biomeos_dir_when_no_env() {
    let env = test_env();
    assert_eq!(
        resolve_capability_socket_fallback("discovery", &env),
        PathBuf::from("/run/user/1000/biomeos/discovery.sock")
    );
}
