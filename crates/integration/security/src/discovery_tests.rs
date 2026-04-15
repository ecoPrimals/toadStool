// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[tokio::test]
async fn test_entropy_client_discovery() {
    // Test client construction without live discovery (avoids nested runtime)
    let socket_path = toadstool_common::primal_sockets::get_biomeos_dir().join("crypto.sock");
    let client = EntropyClient {
        rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        available: false,
    };
    // Client should exist even if entropy service unavailable (fallback)
    assert!(!client.available);
}

#[tokio::test]
async fn test_system_entropy_fallback() {
    // Test system entropy fallback (no client needed - static method)
    let seed = EntropyClient::system_entropy_fallback();
    // Verify seed has expected properties
    assert_eq!(seed.source, EntropySource::Machine);
    assert!(!seed.seed_data.is_empty());
}

#[tokio::test]
async fn test_generate_seed_fallback() {
    // Use biomeOS standard path directly (discovery requires network)
    let socket_path = toadstool_common::primal_sockets::get_biomeos_dir().join("crypto.sock");

    let client = EntropyClient {
        rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        available: false,
    };

    // Should fallback gracefully to system entropy
    let seed = client.generate_seed().await;
    assert!(seed.is_ok());

    let seed = seed.unwrap();
    assert_eq!(seed.source, EntropySource::Machine);
}

#[test]
fn test_seed_request_default() {
    let request = SeedRequest::default();
    assert_eq!(request.source, EntropySource::Mixed);
    assert!((request.min_quality - 0.7).abs() < f32::EPSILON);
    assert!(request.mixing.is_valid());
}

#[tokio::test]
async fn test_entropy_client_is_available() {
    let socket_path = toadstool_common::primal_sockets::get_biomeos_dir().join("crypto.sock");
    let client = EntropyClient {
        rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        available: true,
    };
    assert!(client.is_available());
}

#[tokio::test]
async fn test_entropy_client_not_available() {
    let socket_path = toadstool_common::primal_sockets::get_biomeos_dir().join("crypto.sock");
    let client = EntropyClient {
        rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        available: false,
    };
    assert!(!client.is_available());
}

#[tokio::test]
async fn test_generate_seed_with_request() {
    let socket_path = toadstool_common::primal_sockets::get_biomeos_dir().join("crypto.sock");
    let client = EntropyClient {
        rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        available: false,
    };

    let request = SeedRequest::default();
    let seed = client.generate_seed_with_request(request).await;
    assert!(seed.is_ok());
    let seed = seed.unwrap();
    assert_eq!(seed.source, EntropySource::Machine);
}

#[test]
fn test_seed_request_serialization() {
    let request = SeedRequest::default();
    let json = serde_json::to_string(&request).unwrap();
    let restored: SeedRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(request.source, restored.source);
    assert!((request.min_quality - restored.min_quality).abs() < f32::EPSILON);
}

#[test]
fn test_seed_request_mixing_valid() {
    let request = SeedRequest::default();
    assert!(request.mixing.is_valid());
}

#[test]
fn test_entropy_mixing_security_standard() {
    let mixing = EntropyMixing::security_standard();
    assert!(mixing.is_valid());
    assert!(mixing.machine_weight > 0.0 || mixing.human_weight > 0.0);
}

#[test]
fn test_discover_via_env_security_url() {
    temp_env::with_var("BEARDOG_URL", Some("unix:///run/security.sock"), || {
        let result = std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(EntropyClient::discover_via_capability())
        })
        .join()
        .expect("thread");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "unix:///run/security.sock");
    });
}

#[test]
fn test_ephemeral_seed_structure() {
    let seed = EntropyClient::system_entropy_fallback();
    assert!(!seed.seed_data.is_empty());
    assert_eq!(seed.source, EntropySource::Machine);
    assert!(seed.quality.machine_quality > 0.0);
}

#[test]
fn test_seed_quality_new() {
    let quality = SeedQuality::new(0.8, 0.9, 0.5);
    assert!((quality.machine_quality - 0.9).abs() < f32::EPSILON);
    assert!((quality.human_quality - 0.5).abs() < f32::EPSILON);
}

#[test]
fn test_seed_request_custom_source() {
    let request = SeedRequest {
        source: EntropySource::Human,
        mixing: EntropyMixing::security_standard(),
        min_quality: 0.9,
    };
    assert_eq!(request.source, EntropySource::Human);
    assert!((request.min_quality - 0.9).abs() < f32::EPSILON);
}

#[test]
fn test_entropy_source_variants() {
    let _ = EntropySource::Machine;
    let _ = EntropySource::Human;
    let _ = EntropySource::Mixed;
}

#[tokio::test]
async fn test_entropy_client_with_endpoint_available_false() {
    let socket_path = toadstool_common::primal_sockets::get_biomeos_dir().join("nonexistent.sock");
    let client = EntropyClient {
        rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        available: false,
    };
    let seed = client.generate_seed().await;
    assert!(seed.is_ok());
    assert_eq!(seed.unwrap().source, EntropySource::Machine);
}

#[test]
fn test_seed_request_serialization_roundtrip() {
    let request = SeedRequest {
        source: EntropySource::Mixed,
        mixing: EntropyMixing::security_standard(),
        min_quality: 0.8,
    };
    let json = serde_json::to_string(&request).unwrap();
    let parsed: SeedRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(request.source, parsed.source);
}
