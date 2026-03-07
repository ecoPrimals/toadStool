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
//! Comprehensive coverage tests for biomeos auth module
//! Target: exercise discover, with_beardog, initialize, token refresh, etc.

use std::sync::Arc;
use std::time::Duration;

use toadstool::biomeos_integration::auth::{AuthManagerConfig, AuthenticationManager};
use toadstool::biomeos_integration::InMemoryAuthBackend;
use toadstool_common::constants::timeouts::{TIMESTAMP_VALIDATION_WINDOW, TOKEN_REFRESH_INTERVAL};

fn base_config() -> AuthManagerConfig {
    AuthManagerConfig {
        beardog_endpoint: "http://localhost:9090".to_string(),
        token_refresh_interval: TOKEN_REFRESH_INTERVAL,
        signature_validation: true,
        timestamp_window: TIMESTAMP_VALIDATION_WINDOW,
        replay_protection: true,
        signing_key_seed: None,
        token_audience: vec!["songbird".to_string(), "nestgate".to_string()],
    }
}

// ─── Constructor and with_inmemory ───────────────────────────────────────────

#[test]
fn auth_manager_new() {
    let config = base_config();
    let backend = InMemoryAuthBackend::new();
    let _m = AuthenticationManager::new(config, Arc::new(backend));
}

#[test]
fn auth_manager_with_inmemory() {
    let config = base_config();
    let _m = AuthenticationManager::with_inmemory(config);
}

// ─── with_beardog (deprecated) ───────────────────────────────────────────────

#[test]
#[allow(deprecated)]
fn with_beardog_creates_manager() {
    let mut config = base_config();
    config.beardog_endpoint = "http://localhost:9876".to_string();
    let _m = AuthenticationManager::with_beardog(config);
}

// ─── get_current_token and sign ─────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn get_current_token_inmemory() {
    let config = base_config();
    let manager = AuthenticationManager::with_inmemory(config);
    let result = manager.get_current_token().await;
    assert!(result.is_ok());
    let token = result.unwrap();
    assert!(!token.id.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn sign_token_request_with_inmemory() {
    let mut config = base_config();
    config.signature_validation = false;
    let manager = AuthenticationManager::with_inmemory(config);
    let token = manager.get_current_token().await.unwrap();
    let sig = manager.sign_token_request(&token, "target");
    assert!(sig.is_ok());
    assert_eq!(sig.unwrap(), "signature_disabled");
}

#[tokio::test(flavor = "current_thread")]
async fn sign_verification_request_with_inmemory() {
    let mut config = base_config();
    config.signature_validation = false;
    let manager = AuthenticationManager::with_inmemory(config);
    let sig = manager.sign_verification_request("primal");
    assert!(sig.is_ok());
    assert_eq!(sig.unwrap(), "signature_disabled");
}

// ─── signature_validation disabled ───────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn sign_token_request_disabled_returns_literal() {
    let mut config = base_config();
    config.signature_validation = false;
    let manager = AuthenticationManager::with_inmemory(config);
    let token = manager.get_current_token().await.unwrap();
    let sig = manager.sign_token_request(&token, "x");
    assert!(sig.is_ok());
    assert_eq!(sig.unwrap(), "signature_disabled");
}

#[tokio::test(flavor = "current_thread")]
async fn sign_verification_request_disabled_returns_literal() {
    let mut config = base_config();
    config.signature_validation = false;
    let manager = AuthenticationManager::with_inmemory(config);
    let sig = manager.sign_verification_request("x");
    assert!(sig.is_ok());
    assert_eq!(sig.unwrap(), "signature_disabled");
}

// ─── get_public_key ─────────────────────────────────────────────────────────

#[test]
fn get_public_key_none_when_no_signing_key() {
    let manager = AuthenticationManager::with_inmemory(base_config());
    assert!(manager.get_public_key().is_none());
}

// ─── start_token_refresh and stop ──────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn start_and_stop_token_refresh() {
    let mut config = base_config();
    config.token_refresh_interval = Duration::from_secs(3600);
    let mut manager = AuthenticationManager::with_inmemory(config);
    let start_result = manager.start_token_refresh();
    assert!(start_result.is_ok());
    manager.stop_token_refresh();
}

// ─── initialize_beardog_connection ───────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn initialize_beardog_inmemory_succeeds() {
    let manager = AuthenticationManager::with_inmemory(base_config());
    let result = manager.initialize_beardog_connection().await;
    assert!(result.is_ok());
}
