// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primal sockets tests

use std::path::PathBuf;

use super::*;

fn test_env() -> SocketPathEnv {
    SocketPathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        user: Some("testuser".to_string()),
        biomeos_family_id: None,
        beardog_socket: None,
        songbird_socket: None,
        nestgate_socket: None,
        squirrel_socket: None,
        toadstool_socket: None,
        biomeos_socket_path: None,
        nucleus_socket: None,
    }
}

#[test]
fn socket_path_env_default_is_all_none() {
    let env = SocketPathEnv::default();
    assert!(env.xdg_runtime_dir.is_none());
    assert!(env.user.is_none());
    assert!(env.biomeos_family_id.is_none());
    assert!(env.beardog_socket.is_none());
    assert!(env.songbird_socket.is_none());
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
    let _ = format!("{:?}", env);
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
fn resolve_beardog_socket_uses_env_override() {
    let env = SocketPathEnv {
        beardog_socket: Some("/custom/beardog.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_beardog_socket_fallback(&env),
        PathBuf::from("/custom/beardog.sock")
    );
}

#[test]
fn resolve_beardog_socket_uses_biomeos_fallback() {
    let env = test_env();
    assert_eq!(
        resolve_beardog_socket_fallback(&env),
        PathBuf::from("/run/user/1000/biomeos/beardog.sock")
    );
}

#[test]
fn resolve_songbird_socket_uses_env_override() {
    let env = SocketPathEnv {
        songbird_socket: Some("/custom/songbird.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_songbird_socket_fallback(&env),
        PathBuf::from("/custom/songbird.sock")
    );
}

#[test]
fn resolve_songbird_socket_uses_biomeos_fallback() {
    let env = test_env();
    assert_eq!(
        resolve_songbird_socket_fallback(&env),
        PathBuf::from("/run/user/1000/biomeos/songbird.sock")
    );
}

#[test]
fn resolve_nestgate_socket_uses_env_override() {
    let env = SocketPathEnv {
        nestgate_socket: Some("/custom/nestgate.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_nestgate_socket_fallback(&env),
        PathBuf::from("/custom/nestgate.sock")
    );
}

#[test]
fn resolve_nestgate_socket_uses_biomeos_fallback() {
    let env = test_env();
    assert_eq!(
        resolve_nestgate_socket_fallback(&env),
        PathBuf::from("/run/user/1000/biomeos/nestgate.sock")
    );
}

#[test]
fn resolve_squirrel_socket_uses_env_override() {
    let env = SocketPathEnv {
        squirrel_socket: Some("/custom/squirrel.sock".to_string()),
        ..test_env()
    };
    assert_eq!(
        resolve_squirrel_socket(&env),
        PathBuf::from("/custom/squirrel.sock")
    );
}

#[test]
fn resolve_squirrel_socket_uses_biomeos_fallback() {
    let env = test_env();
    assert_eq!(
        resolve_squirrel_socket(&env),
        PathBuf::from("/run/user/1000/biomeos/squirrel.sock")
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
        PathBuf::from("/run/user/1000/biomeos/toadstool.sock")
    );
}

#[test]
#[allow(deprecated)]
fn resolve_service_socket_override_takes_precedence() {
    let env = test_env();
    let override_path = PathBuf::from("/override/custom.sock");
    let result = resolve_socket_path_for_service("beardog", &env, Some(override_path.clone()));
    assert_eq!(result, override_path);
}

#[test]
#[allow(deprecated)]
fn resolve_service_socket_beardog() {
    let env = test_env();
    let result = resolve_socket_path_for_service("beardog", &env, None);
    assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/beardog.sock"));
}

#[test]
#[allow(deprecated)]
fn resolve_service_socket_beardog_alias() {
    let env = test_env();
    let result = resolve_socket_path_for_service("bear-dog", &env, None);
    assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/beardog.sock"));
}

#[test]
#[allow(deprecated)]
fn resolve_service_socket_songbird() {
    let env = test_env();
    let result = resolve_socket_path_for_service("songbird", &env, None);
    assert_eq!(
        result,
        PathBuf::from("/run/user/1000/biomeos/songbird.sock")
    );
}

#[test]
#[allow(deprecated)]
fn resolve_service_socket_toadstool() {
    let env = test_env();
    let result = resolve_socket_path_for_service("toadstool", &env, None);
    assert_eq!(
        result,
        PathBuf::from("/run/user/1000/biomeos/toadstool.sock")
    );
}

#[test]
#[allow(deprecated)]
fn resolve_service_socket_nucleus() {
    let env = test_env();
    let result = resolve_socket_path_for_service("nucleus", &env, None);
    assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/nucleus.sock"));
}

#[test]
#[allow(deprecated)]
fn resolve_service_socket_unknown_falls_through() {
    let env = test_env();
    let result = resolve_socket_path_for_service("myservice", &env, None);
    assert_eq!(
        result,
        PathBuf::from("/run/user/1000/biomeos/myservice.sock")
    );
}

#[test]
#[allow(deprecated)]
fn resolve_service_socket_case_insensitive() {
    let env = test_env();
    let result = resolve_socket_path_for_service("BearDog", &env, None);
    assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/beardog.sock"));
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
#[allow(deprecated)]
fn get_beardog_socket_path_is_path() {
    let path = get_beardog_socket_path();
    assert!(path.to_string_lossy().contains("beardog"));
}

#[test]
#[allow(deprecated)]
fn get_songbird_socket_path_is_path() {
    let path = get_songbird_socket_path();
    assert!(path.to_string_lossy().contains("songbird"));
}

#[test]
#[allow(deprecated)]
fn get_nestgate_socket_path_is_path() {
    let path = get_nestgate_socket_path();
    assert!(path.to_string_lossy().contains("nestgate"));
}

#[test]
fn get_squirrel_socket_path_is_path() {
    let path = get_squirrel_socket_path();
    assert!(path.to_string_lossy().contains("squirrel"));
}

#[test]
fn get_toadstool_socket_path_is_path() {
    let path = get_toadstool_socket_path();
    assert!(path.to_string_lossy().contains("toadstool"));
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
            assert!(result.is_ok(), "ensure_biomeos_dir failed: {:?}", result);
            let path = result.unwrap();
            assert!(path.to_string_lossy().contains("biomeos"));
            assert!(path.exists());
        },
    );
}

#[test]
#[allow(deprecated)]
fn test_get_socket_path_for_service_unknown_with_env_override() {
    let service_name = format!("testsvc_{}", std::process::id());
    let test_key = format!("TESTSVC_{}_SOCKET", std::process::id());
    let custom_path = "/tmp/custom-test.sock";
    std::env::set_var(&test_key, custom_path);

    let path = get_socket_path_for_service(&service_name);

    std::env::remove_var(&test_key);

    assert_eq!(path.to_string_lossy(), custom_path);
}

#[test]
#[allow(deprecated)]
fn test_resolve_service_socket_empty_service_name() {
    let env = test_env();
    let result = resolve_socket_path_for_service("", &env, None);
    assert!(result.to_string_lossy().ends_with(".sock"));
}
