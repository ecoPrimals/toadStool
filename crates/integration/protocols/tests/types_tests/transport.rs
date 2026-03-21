// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn test_transport_type_http() {
    let transport = TransportType::Http;
    assert_eq!(transport, TransportType::Http);
}

#[test]
fn test_transport_type_websocket() {
    let transport = TransportType::TRpc;
    assert_eq!(transport, TransportType::TRpc);
}

#[test]
fn test_transport_type_trpc() {
    let transport = TransportType::TRpc;
    assert_eq!(transport, TransportType::TRpc);
}

#[test]
fn test_transport_type_tcp() {
    let transport = TransportType::Tcp;
    assert_eq!(transport, TransportType::Tcp);
}

#[test]
fn test_transport_type_udp() {
    let transport = TransportType::Udp;
    assert_eq!(transport, TransportType::Udp);
}

#[test]
fn test_transport_type_custom() {
    let transport = TransportType::Custom("grpc".to_string());
    if let TransportType::Custom(name) = transport {
        assert_eq!(name, "grpc");
    } else {
        panic!("Expected Custom transport");
    }
}

#[test]
fn test_transport_type_tcp_serialization() {
    let transport = TransportType::Tcp;
    let serialized = serde_json::to_string(&transport).expect("Failed to serialize");
    let deserialized: TransportType =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(transport, deserialized);
}

#[test]
fn test_transport_type_udp_serialization() {
    let transport = TransportType::Udp;
    let serialized = serde_json::to_string(&transport).expect("Failed to serialize");
    let deserialized: TransportType =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(transport, deserialized);
}

#[test]
fn test_transport_type_custom_serialization() {
    let transport = TransportType::Custom("quic".to_string());
    let serialized = serde_json::to_string(&transport).expect("Failed to serialize");
    let deserialized: TransportType =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let TransportType::Custom(name) = deserialized {
        assert_eq!(name, "quic");
    } else {
        panic!("Expected Custom transport type");
    }
}
