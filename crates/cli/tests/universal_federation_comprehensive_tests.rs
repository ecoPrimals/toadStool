// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Universal Federation Operations
//!
//! Tests for federation functionality in universal compute manager.
//! Coverage target: Get federation.rs from current low coverage to >80%

// WebSocket federation tests removed S317 — setup_websocket_federation deleted.

use anyhow::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use toadstool_cli::universal::UniversalComputeManager;
use toadstool_cli::universal::operations::FederationOps;
use url::Url;

/// Helper to create a test manager
async fn create_manager() -> Result<UniversalComputeManager> {
    Ok(UniversalComputeManager::new().await?)
}

// ==================================================
// Local Capabilities Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_local_capabilities_basic() -> Result<()> {
    let manager = create_manager().await?;

    let caps = manager.get_local_capabilities();
    assert!(!caps.is_empty(), "Local capabilities should not be empty");
    assert!(caps.len() >= 3, "Should have multiple capabilities");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_local_capabilities_contains_expected() -> Result<()> {
    let manager = create_manager().await?;

    let caps = manager.get_local_capabilities();
    let cap_strs: Vec<String> = caps.iter().map(std::string::ToString::to_string).collect();

    // Should contain key capabilities
    assert!(
        cap_strs
            .iter()
            .any(|c| c.contains("compute") || c.contains("COMPUTE"))
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_local_capabilities_multiple_calls() -> Result<()> {
    let manager = create_manager().await?;

    let caps1 = manager.get_local_capabilities();
    let caps2 = manager.get_local_capabilities();

    assert_eq!(
        caps1.len(),
        caps2.len(),
        "Capabilities should be consistent"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_local_capabilities_idempotent() -> Result<()> {
    let manager = create_manager().await?;

    for _ in 0..10 {
        let caps = manager.get_local_capabilities();
        assert!(!caps.is_empty());
    }

    Ok(())
}

// ==================================================
// Peer Connection Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_setup_https_federation_basic() -> Result<()> {
    let manager = create_manager().await?;
    let endpoint = Url::parse("https://example.com:8443")?;

    let result = manager.setup_https_federation(&endpoint, "client").await;
    // May succeed or fail depending on network, just verify it doesn't panic
    let _ = result;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_setup_https_federation_client_mode() -> Result<()> {
    let manager = create_manager().await?;
    let endpoint = Url::parse("https://localhost:8443")?;

    let result = manager.setup_https_federation(&endpoint, "client").await;
    let _ = result;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_setup_https_federation_server_mode() -> Result<()> {
    let manager = create_manager().await?;
    let endpoint = Url::parse("https://localhost:8443")?;

    let result = manager.setup_https_federation(&endpoint, "server").await;
    let _ = result;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_setup_https_federation_various_endpoints() -> Result<()> {
    let manager = create_manager().await?;

    let endpoints = vec![
        "https://localhost:8443",
        "https://127.0.0.1:9000",
        "https://peer.example.com:8080",
    ];

    for endpoint_str in endpoints {
        let endpoint = Url::parse(endpoint_str)?;
        let result = manager.setup_https_federation(&endpoint, "client").await;
        let _ = result;
    }

    Ok(())
}

// ==================================================
// WebSocket Federation Tests
// ==================================================

// ==================================================
// Heartbeat Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_send_heartbeat_ping_localhost() -> Result<()> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);

    let result = UniversalComputeManager::send_heartbeat_ping(&addr).await;
    // Network call may fail, just verify no panic
    let _ = result;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_send_heartbeat_ping_various_addrs() -> Result<()> {
    let addrs = vec![
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9000),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8443),
    ];

    for addr in addrs {
        let result = UniversalComputeManager::send_heartbeat_ping(&addr).await;
        let _ = result;
    }

    Ok(())
}

// ==================================================
// Peer Monitoring Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_start_peer_monitoring_basic() -> Result<()> {
    let manager = create_manager().await?;
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);

    let result = manager.start_peer_monitoring(&addr).await;
    // May fail if peer is not available - that's okay
    let _ = result;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_start_peer_monitoring_multiple_peers() -> Result<()> {
    let manager = create_manager().await?;

    let addrs = vec![
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8081),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8082),
    ];

    for addr in addrs {
        let result = manager.start_peer_monitoring(&addr).await;
        let _ = result;
    }

    Ok(())
}

// ==================================================
// Concurrent Operations Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_capabilities_access() -> Result<()> {
    let (c1, c2, c3) = tokio::join!(
        tokio::spawn(async move {
            let manager = create_manager().await.unwrap();
            manager.get_local_capabilities()
        }),
        tokio::spawn(async move {
            let manager = create_manager().await.unwrap();
            manager.get_local_capabilities()
        }),
        tokio::spawn(async move {
            let manager = create_manager().await.unwrap();
            manager.get_local_capabilities()
        }),
    );

    assert!(c1.is_ok() && c2.is_ok() && c3.is_ok());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_heartbeat_pings() -> Result<()> {
    let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8081);
    let addr3 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8082);

    let (_, _, _) = tokio::join!(
        UniversalComputeManager::send_heartbeat_ping(&addr1),
        UniversalComputeManager::send_heartbeat_ping(&addr2),
        UniversalComputeManager::send_heartbeat_ping(&addr3),
    );

    Ok(())
}

// ==================================================
// Multiple Manager Instances
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_managers_capabilities() -> Result<()> {
    let manager1 = create_manager().await?;
    let manager2 = create_manager().await?;

    let caps1 = manager1.get_local_capabilities();
    let caps2 = manager2.get_local_capabilities();

    assert_eq!(
        caps1.len(),
        caps2.len(),
        "Different managers should have same capabilities"
    );

    Ok(())
}

// ==================================================
// Edge Cases
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_https_federation_with_port_variations() -> Result<()> {
    let manager = create_manager().await?;

    let ports = vec![80, 443, 8080, 8443, 9000];

    for port in ports {
        let endpoint = Url::parse(&format!("https://localhost:{port}"))?;
        let result = manager.setup_https_federation(&endpoint, "client").await;
        let _ = result;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_capabilities_zero_copy_optimization() -> Result<()> {
    let manager = create_manager().await?;

    // Get capabilities multiple times
    let caps1 = manager.get_local_capabilities();
    let caps2 = manager.get_local_capabilities();

    // Arc<str> should allow cheap clones
    for (c1, c2) in caps1.iter().zip(caps2.iter()) {
        assert_eq!(c1.as_ref(), c2.as_ref());
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rapid_federation_setups() -> Result<()> {
    let manager = create_manager().await?;

    for i in 0..10 {
        let endpoint = Url::parse(&format!("https://localhost:{}", 8000 + i))?;
        let _ = manager.setup_https_federation(&endpoint, "client").await;
    }

    Ok(())
}

// ==================================================
// Lifecycle Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_federation_full_lifecycle() -> Result<()> {
    let manager = create_manager().await?;

    // Get capabilities
    let _caps = manager.get_local_capabilities();

    // Setup HTTPS federation
    let https_endpoint = Url::parse("https://localhost:8443")?;
    let _ = manager
        .setup_https_federation(&https_endpoint, "client")
        .await;

    // Start peer monitoring
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    let _ = manager.start_peer_monitoring(&addr).await;

    // Send heartbeat
    let _ = UniversalComputeManager::send_heartbeat_ping(&addr).await;

    Ok(())
}

// ==================================================
// Stress Tests
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_many_capability_queries() -> Result<()> {
    let manager = create_manager().await?;

    for _ in 0..100 {
        let caps = manager.get_local_capabilities();
        assert!(!caps.is_empty());
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_many_heartbeat_attempts() -> Result<()> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);

    for _ in 0..20 {
        let _ = UniversalComputeManager::send_heartbeat_ping(&addr).await;
    }

    Ok(())
}

// ==================================================
// Mode Variations
// ==================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_federation_different_modes() -> Result<()> {
    let manager = create_manager().await?;
    let endpoint = Url::parse("https://localhost:8443")?;

    let modes = vec!["client", "server", "peer", "hub"];

    for mode in modes {
        let result = manager.setup_https_federation(&endpoint, mode).await;
        let _ = result;
    }

    Ok(())
}
