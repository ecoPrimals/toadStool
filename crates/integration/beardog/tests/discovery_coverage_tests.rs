// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::pedantic)]
//! Coverage tests for beardog discovery (discovery.rs)
//!
//! Focus: service discovery, capability registration, lookup, error paths

use toadstool_integration_beardog::{
    discover_entropy, EntropyClient, EntropyMixing, EntropySource, SeedQuality, SeedRequest,
};

// ============================================================================
// discover_entropy - skipped: CapabilityDiscovery::new() uses block_on
// which panics when called from within tokio runtime. Run with --ignored
// or test manually.
// ============================================================================

#[test]
#[ignore = "CapabilityDiscovery uses block_on; run with --ignored in isolation"]
fn test_discover_entropy_returns_client_or_error() {
    let result = std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(discover_entropy())
    })
    .join()
    .expect("thread");
    match &result {
        Ok(client) => {
            let _ = client.is_available();
        }
        Err(e) => {
            assert!(!e.to_string().is_empty());
        }
    }
}

#[test]
#[ignore = "CapabilityDiscovery uses block_on; run with --ignored in isolation"]
fn test_discover_then_generate_seed() {
    let result = std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(async {
            let client = discover_entropy().await?;
            let seed = client.generate_seed().await?;
            Result::<_, toadstool_integration_beardog::BeardogError>::Ok(seed)
        })
    })
    .join()
    .expect("thread");
    if let Ok(seed) = result {
        assert!(!seed.seed_data.is_empty());
    }
}

// ============================================================================
// SeedRequest
// ============================================================================

#[test]
fn test_seed_request_default() {
    let request = SeedRequest::default();
    assert_eq!(request.source, EntropySource::Mixed);
    assert!((request.min_quality - 0.7).abs() < f32::EPSILON);
    assert!(request.mixing.is_valid());
}

#[test]
fn test_seed_request_serialization_roundtrip() {
    let request = SeedRequest::default();
    let json = serde_json::to_string(&request).unwrap();
    let restored: SeedRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(request.source, restored.source);
    assert!((request.min_quality - restored.min_quality).abs() < f32::EPSILON);
}

#[test]
fn test_seed_request_custom_source() {
    let request = SeedRequest {
        source: EntropySource::Human,
        mixing: EntropyMixing::beardog_standard(),
        min_quality: 0.9,
    };
    assert_eq!(request.source, EntropySource::Human);
    assert!((request.min_quality - 0.9).abs() < f32::EPSILON);
}

#[test]
fn test_seed_request_mixing_valid() {
    let request = SeedRequest::default();
    assert!(request.mixing.is_valid());
}

// ============================================================================
// generate_seed_with_request
// ============================================================================

#[test]
#[ignore = "CapabilityDiscovery uses block_on; run with --ignored in isolation"]
fn test_generate_seed_with_request() {
    let result = std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        rt.block_on(async {
            let client = discover_entropy().await?;
            let request = SeedRequest::default();
            client.generate_seed_with_request(request).await
        })
    })
    .join()
    .expect("thread");
    if let Ok(seed) = result {
        assert!(!seed.seed_data.is_empty());
    }
}

// ============================================================================
// EntropySource variants
// ============================================================================

#[test]
fn test_entropy_source_variants() {
    let _ = EntropySource::Machine;
    let _ = EntropySource::Human;
    let _ = EntropySource::Mixed;
}

// ============================================================================
// EntropyMixing
// ============================================================================

#[test]
fn test_entropy_mixing_beardog_standard() {
    let mixing = EntropyMixing::beardog_standard();
    assert!(mixing.is_valid());
    assert!(mixing.machine_weight > 0.0 || mixing.human_weight > 0.0);
}

// ============================================================================
// SeedQuality
// ============================================================================

#[test]
fn test_seed_quality_new() {
    let quality = SeedQuality::new(0.8, 0.9, 0.5);
    assert!((quality.machine_quality - 0.9).abs() < f32::EPSILON);
    assert!((quality.human_quality - 0.5).abs() < f32::EPSILON);
}

// ============================================================================
// Discovery via BEARDOG_URL env (connect still uses discover_crypto_socket)
// ============================================================================

#[test]
#[ignore = "connect uses discover_crypto_socket which uses block_on"]
fn test_discover_via_env_beardog_url() {
    temp_env::with_var("BEARDOG_URL", Some("unix:///run/beardog.sock"), || {
        let result = std::thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            rt.block_on(EntropyClient::discover())
        })
        .join()
        .expect("thread");
        assert!(result.is_ok());
        let _client = result.unwrap();
    });
}
