// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from adapters.rs (S334).

use super::adapters::*;
use super::registry::Capability;

#[test]
fn test_coordination_adapter_new_with_endpoint() {
    let adapter = CoordinationAdapter::new_with_endpoint(
        "http://coordination:8080",
        "http://toadstool:9090".to_string(),
    )
    .unwrap();
    assert_eq!(adapter.primal_name(), "coordination");
    assert_eq!(adapter.endpoint(), "http://coordination:8080");
}

#[test]
fn test_coordination_adapter_new_requires_toadstool_endpoint() {
    temp_env::with_vars([("TOADSTOOL_ENDPOINT", None::<&str>)], || {
        let result = CoordinationAdapter::new("http://coordination:8080");
        match result {
            Err(e) => assert!(e.to_string().contains("TOADSTOOL_ENDPOINT")),
            Ok(_) => unreachable!("expected error when TOADSTOOL_ENDPOINT not set"),
        }
    });
}

#[test]
fn test_coordination_adapter_new_with_env() {
    temp_env::with_var("TOADSTOOL_ENDPOINT", Some("http://self:9090"), || {
        let result = CoordinationAdapter::new("http://coordination:8080");
        assert!(result.is_ok());
        let adapter = result.unwrap();
        assert_eq!(adapter.primal_name(), "coordination");
        assert_eq!(adapter.endpoint(), "http://coordination:8080");
    });
}

fn make_test_adapter() -> CoordinationAdapter {
    CoordinationAdapter::new_with_endpoint(
        "http://coordination:8080",
        "http://toadstool:9090".to_string(),
    )
    .unwrap()
}

#[tokio::test]
async fn test_coordination_adapter_send_heartbeat() {
    let adapter = make_test_adapter();
    let result = adapter.send_heartbeat().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_coordination_adapter_notify_capability_change() {
    let adapter = make_test_adapter();
    let cap = Capability::compute_heavy();
    let result = adapter.notify_capability_change(&cap, false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_coordination_adapter_deregister() {
    let adapter = make_test_adapter();
    let result = adapter.deregister().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_coordination_adapter_register_capabilities_fails_without_socket() {
    let adapter = make_test_adapter();
    let caps = vec![Capability::compute_heavy()];
    let result = adapter.register_capabilities(caps).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_coordination_adapter_register_capabilities_empty() {
    let adapter = make_test_adapter();
    let result = adapter.register_capabilities(vec![]).await;
    assert!(result.is_err());
}

#[test]
fn test_coordination_adapter_primal_adapter_trait() {
    const NOTIFY_TEST_PORT: u16 = 9090;
    let http_fallback = format!(
        "{}{}:{}",
        toadstool_common::constants::network::HTTP_PROTOCOL,
        toadstool_common::constants::network::DEFAULT_HOSTNAME,
        NOTIFY_TEST_PORT,
    );
    let adapter =
        CoordinationAdapter::new_with_endpoint("unix:///tmp/coordination.sock", http_fallback)
            .unwrap();
    assert_eq!(adapter.primal_name(), "coordination");
    assert_eq!(adapter.endpoint(), "unix:///tmp/coordination.sock");
}

#[tokio::test]
async fn test_coordination_adapter_notify_gpu_capability() {
    let adapter = make_test_adapter();
    let cap = Capability::compute_gpu();
    let result = adapter.notify_capability_change(&cap, true).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_coordination_adapter_notify_capability_with_custom_id() {
    let adapter = make_test_adapter();
    let cap = Capability::compute_ml_training();
    let result = adapter.notify_capability_change(&cap, false).await;
    assert!(result.is_ok());
}

#[test]
fn test_coordination_adapter_endpoint_preserved() {
    let ep = "https://custom-coordination.example.com:9999";
    let adapter = CoordinationAdapter::new_with_endpoint(ep, "http://me:1".to_string()).unwrap();
    assert_eq!(adapter.endpoint(), ep);
}
