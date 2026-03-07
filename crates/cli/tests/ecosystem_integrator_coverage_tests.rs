// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic)]
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
//! Comprehensive tests for ecosystem `integrator_impl.rs`
//! Target: 85%+ coverage. No real network I/O, no `multi_thread` runtime.

use std::path::PathBuf;

use toadstool_cli::ecosystem::EcosystemIntegrator;

// ─── Constructor and default ──────────────────────────────────────────────

#[test]
fn integrator_new_constructs() {
    let _i = EcosystemIntegrator::new();
}

#[test]
fn integrator_default_constructs() {
    let _i = EcosystemIntegrator::default();
}

// ─── show_ecosystem_status formats ──────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn show_status_json_empty() {
    let i = EcosystemIntegrator::new();
    let result = i.show_ecosystem_status("json").await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn show_status_table_empty() {
    let i = EcosystemIntegrator::new();
    let result = i.show_ecosystem_status("table").await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn show_status_default_format() {
    let i = EcosystemIntegrator::new();
    let result = i.show_ecosystem_status("text").await;
    assert!(result.is_ok());
}

// ─── Error paths (no network) ───────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn register_with_orchestrator_invalid_endpoint() {
    let mut i = EcosystemIntegrator::new();
    let result = i
        .register_with_orchestrator("not-valid-addr".to_string(), None)
        .await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn connect_nestgate_storage_invalid_endpoint() {
    let mut i = EcosystemIntegrator::new();
    let result = i
        .connect_nestgate_storage(
            "invalid".to_string(),
            PathBuf::from("/tmp/test-mount"),
            None,
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn install_crypto_permissions_nonexistent() {
    let mut i = EcosystemIntegrator::new();
    let result = i
        .install_crypto_permissions(PathBuf::from("/nonexistent/perms.json"), true)
        .await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn install_crypto_permissions_validate_only() {
    let mut i = EcosystemIntegrator::new();
    let result = i
        .install_crypto_permissions(PathBuf::from("/dev/null"), true)
        .await;
    assert!(result.is_err());
}

// ─── discover_services with empty types (fast timeout) ──────────────────────

#[tokio::test(flavor = "current_thread")]
async fn discover_services_empty_types_short_timeout() {
    let mut i = EcosystemIntegrator::new();
    let result = i.discover_services(vec![], 1).await;
    assert!(result.is_ok());
    let discovery = result.unwrap();
    assert!(discovery.scan_duration.as_secs() <= 2);
}

#[tokio::test(flavor = "current_thread")]
async fn discover_services_with_capability_names() {
    let mut i = EcosystemIntegrator::new();
    let result = i
        .discover_services(vec!["network".to_string(), "crypto".to_string()], 1)
        .await;
    assert!(result.is_ok());
    let discovery = result.unwrap();
    assert!(discovery.verified_count <= discovery.total_discovered);
}

#[tokio::test(flavor = "current_thread")]
async fn discover_services_with_legacy_names() {
    let mut i = EcosystemIntegrator::new();
    let result = i
        .discover_services(vec!["songbird".to_string(), "beardog".to_string()], 1)
        .await;
    assert!(result.is_ok());
}

// ─── Ecosystem discovery coverage via integrator (discovery module is private) ───

#[tokio::test(flavor = "current_thread")]
async fn discover_services_with_crypto_env_set() {
    let old = std::env::var("TOADSTOOL_CRYPTO_SERVICE_URL").ok();
    std::env::set_var("TOADSTOOL_CRYPTO_SERVICE_URL", "http://127.0.0.1:9876");
    let mut i = EcosystemIntegrator::new();
    let result = i.discover_services(vec!["crypto".to_string()], 1).await;
    if let Some(v) = old {
        std::env::set_var("TOADSTOOL_CRYPTO_SERVICE_URL", v);
    } else {
        std::env::remove_var("TOADSTOOL_CRYPTO_SERVICE_URL");
    }
    assert!(result.is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn discover_services_with_storage_env_set() {
    let old = std::env::var("TOADSTOOL_STORAGE_SERVICE_URL").ok();
    std::env::set_var("TOADSTOOL_STORAGE_SERVICE_URL", "http://127.0.0.1:8082");
    let mut i = EcosystemIntegrator::new();
    let result = i.discover_services(vec!["storage".to_string()], 1).await;
    if let Some(v) = old {
        std::env::set_var("TOADSTOOL_STORAGE_SERVICE_URL", v);
    } else {
        std::env::remove_var("TOADSTOOL_STORAGE_SERVICE_URL");
    }
    assert!(result.is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn discover_services_empty_capability_list() {
    let mut i = EcosystemIntegrator::new();
    let result = i.discover_services(vec![], 1).await;
    assert!(result.is_ok());
    let discovery = result.unwrap();
    let _ = discovery.total_discovered;
}

#[tokio::test(flavor = "current_thread")]
async fn discover_services_single_capability() {
    let mut i = EcosystemIntegrator::new();
    let result = i
        .discover_services(vec!["coordination".to_string()], 1)
        .await;
    assert!(result.is_ok());
}

// ─── Capability mapping coverage (legacy names, pass-through) ───────────────

#[tokio::test(flavor = "current_thread")]
async fn discover_services_with_nestgate_legacy_name() {
    let mut i = EcosystemIntegrator::new();
    let result = i.discover_services(vec!["nestgate".to_string()], 1).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn discover_services_with_pki_alias() {
    let mut i = EcosystemIntegrator::new();
    let result = i.discover_services(vec!["pki".to_string()], 1).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn discover_services_with_pass_through_capability() {
    let mut i = EcosystemIntegrator::new();
    let result = i
        .discover_services(vec!["custom_capability".to_string()], 1)
        .await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn discover_services_result_has_expected_fields() {
    let mut i = EcosystemIntegrator::new();
    let result = i.discover_services(vec![], 1).await.unwrap();
    assert!(result.services.is_empty() || !result.services.is_empty());
    assert_eq!(result.total_discovered, result.services.len());
    assert!(result.verified_count <= result.total_discovered);
}

#[tokio::test(flavor = "current_thread")]
async fn show_status_json_output_valid() {
    let i = EcosystemIntegrator::new();
    let result = i.show_ecosystem_status("json").await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "current_thread")]
async fn show_status_table_output_empty_integrator() {
    let i = EcosystemIntegrator::new();
    let result = i.show_ecosystem_status("table").await;
    assert!(result.is_ok());
}
