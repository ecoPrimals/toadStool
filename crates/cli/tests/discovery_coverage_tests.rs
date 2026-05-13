// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::pedantic,
    clippy::redundant_closure,
    unused_imports,
    clippy::let_unit_value
)]
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive coverage tests for discovery module
//! Target: exercise all branches including error paths and edge cases.

use std::sync::{Arc, Mutex};

use toadstool_cli::ecosystem::discovery::{
    discover_from_config, discover_from_environment, discover_service_by_capability, verify_service,
};

static CWD_LOCK: Mutex<()> = Mutex::new(());
#[expect(deprecated, reason = "testing legacy ecosystem discovery types during migration")]
use toadstool_cli::ecosystem::types::{EcosystemService, ServiceEndpoint, TrustLevel};

// ─── discover_from_environment ──────────────────────────────────────────────

#[test]
fn discover_from_environment_found() {
    temp_env::with_var(
        "TOADSTOOL_CRYPTO_SERVICE_URL",
        Some("http://10.0.0.5:9876"),
        || {
            let result = discover_from_environment("crypto");
            assert_eq!(result, Some("http://10.0.0.5:9876".to_string()));
        },
    );
}

#[test]
fn discover_from_environment_not_set() {
    temp_env::with_var_unset("TOADSTOOL_NONEXISTENT_SERVICE_URL", || {
        let result = discover_from_environment("nonexistent");
        assert!(result.is_none());
    });
}

#[test]
fn discover_from_environment_empty_value() {
    temp_env::with_var("TOADSTOOL_EMPTY_SERVICE_URL", Some(""), || {
        let result = discover_from_environment("empty");
        assert!(result.is_none());
    });
}

#[test]
fn discover_from_environment_uppercase_env_var() {
    temp_env::with_var(
        "TOADSTOOL_STORAGE_SERVICE_URL",
        Some("http://storage.local:8082"),
        || {
            let result = discover_from_environment("storage");
            assert_eq!(result, Some("http://storage.local:8082".to_string()));
        },
    );
}

#[test]
fn discover_from_environment_mixed_case_capability() {
    temp_env::with_var(
        "TOADSTOOL_COORDINATION_SERVICE_URL",
        Some("http://127.0.0.1:9999"),
        || {
            let result = discover_from_environment("coordination");
            assert!(result.is_some());
        },
    );
}

// ─── discover_from_config ───────────────────────────────────────────────────

#[test]
fn discover_from_config_no_config_returns_none() {
    let result = discover_from_config("nonexistent_capability_xyz");
    assert!(result.is_none());
}

#[test]
fn discover_from_config_with_valid_config_in_cwd() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config_dir = dir.path().join(".toadstool");
    std::fs::create_dir_all(&config_dir).expect("create dir");
    let config_path = config_dir.join("config.toml");
    let config_content = r#"
[services.crypto]
url = "http://127.0.0.1:9876"
priority = 90

[services.storage]
url = "http://127.0.0.1:8082"
"#;
    std::fs::write(&config_path, config_content).expect("write config");

    let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev_cwd = std::env::current_dir().ok();
    let set_ok = std::env::set_current_dir(dir.path()).is_ok();
    let result = if set_ok {
        discover_from_config("crypto")
    } else {
        None
    };
    if let Some(ref cwd) = prev_cwd {
        std::env::set_current_dir(cwd).ok();
    }
    if !set_ok {
        return;
    }
    assert!(
        result.is_some(),
        "should find crypto from ./.toadstool/config.toml"
    );
    assert_eq!(result.unwrap(), "http://127.0.0.1:9876");
}

#[test]
fn discover_from_config_missing_category_returns_none() {
    let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().expect("temp dir");
    let config_dir = dir.path().join(".toadstool");
    std::fs::create_dir_all(&config_dir).expect("create dir");
    let config_path = config_dir.join("config.toml");
    std::fs::write(
        &config_path,
        r#"[services.crypto]
url = "http://127.0.0.1:9876"
"#,
    )
    .expect("write");
    let prev_cwd = std::env::current_dir().ok();
    std::env::set_current_dir(dir.path()).expect("set cwd");
    let result = discover_from_config("nonexistent");
    if let Some(ref cwd) = prev_cwd {
        std::env::set_current_dir(cwd).ok();
    }
    assert!(result.is_none());
}

#[test]
fn discover_from_config_invalid_toml_returns_none() {
    let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().expect("temp dir");
    let config_dir = dir.path().join(".toadstool");
    std::fs::create_dir_all(&config_dir).expect("create dir");
    let config_path = config_dir.join("config.toml");
    std::fs::write(&config_path, "invalid toml [").expect("write");
    let prev_cwd = std::env::current_dir().ok();
    std::env::set_current_dir(dir.path()).expect("set cwd");
    let result = discover_from_config("crypto");
    if let Some(ref cwd) = prev_cwd {
        std::env::set_current_dir(cwd).ok();
    }
    assert!(result.is_none());
}

// ─── discover_service_by_capability ─────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discover_service_by_capability_from_env() {
    temp_env::async_with_vars(
        [(
            "TOADSTOOL_CRYPTO_SERVICE_URL",
            Some("http://127.0.0.1:9876"),
        )],
        async {
            let r: Result<Vec<ServiceEndpoint>, _> = discover_service_by_capability("crypto").await;
            assert!(r.is_ok());
            let services = r.expect("ok");
            assert!(!services.is_empty());
            assert!(matches!(services[0].trust_level, TrustLevel::Configured));
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discover_service_by_capability_no_source_returns_empty() {
    temp_env::async_with_vars([("TOADSTOOL_NOSVC_SERVICE_URL", None::<&str>)], async {
        let r: Result<Vec<ServiceEndpoint>, _> = discover_service_by_capability("nosvc").await;
        assert!(r.is_ok());
        let services = r.expect("ok");
        assert!(services.is_empty());
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discover_service_by_capability_invalid_env_url_skips_to_config() {
    temp_env::async_with_vars(
        [("TOADSTOOL_BADURL_SERVICE_URL", Some("not-a-valid-addr"))],
        async {
            let r: Result<Vec<ServiceEndpoint>, _> = discover_service_by_capability("badurl").await;
            assert!(r.is_ok());
            let services = r.expect("ok");
            assert!(services.is_empty());
        },
    )
    .await;
}

// ─── verify_service ─────────────────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
#[expect(deprecated, reason = "testing legacy verify_service during migration")]
async fn verify_service_unreachable_returns_false() {
    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::Unknown("test".to_string()),
        address: "192.0.2.1:1".parse().expect("parse"),
        version: Arc::from("1.0"),
        capabilities: vec![],
        trust_level: TrustLevel::Discovered,
    };
    let result: Result<bool, _> = verify_service(&endpoint).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[tokio::test(flavor = "current_thread")]
#[expect(deprecated, reason = "testing legacy verify_service during migration")]
async fn verify_service_localhost_unbound_returns_false() {
    // Bind an ephemeral port, capture it, then drop so nothing listens.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);

    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::Unknown("test".to_string()),
        address: format!("127.0.0.1:{port}").parse().expect("parse"),
        version: Arc::from("1.0"),
        capabilities: vec![],
        trust_level: TrustLevel::Configured,
    };
    let result: Result<bool, _> = verify_service(&endpoint).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}
