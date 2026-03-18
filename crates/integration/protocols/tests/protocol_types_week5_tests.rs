// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration Protocols Types Tests - Week 5
//! Comprehensive tests for protocol types and configuration

// Removed unused imports
use toadstool_integration_protocols::types::{
    AuthType, MessageFormat, MessagePriority, TransportType,
};

// ============================================================================
// TransportType Tests
// ============================================================================

#[test]
fn test_transport_type_variants() {
    let types = vec![
        TransportType::Http,
        TransportType::TRpc,
        TransportType::TRpc,
        TransportType::Tcp,
        TransportType::Custom("test".to_string()),
    ];

    assert_eq!(types.len(), 5);
}

#[test]
fn test_transport_type_clone() {
    let original = TransportType::Http;
    let cloned = original.clone();

    assert!(matches!(cloned, TransportType::Http));
}

#[test]
fn test_transport_type_debug() {
    let transport = TransportType::TRpc;
    let debug_str = format!("{transport:?}");

    assert!(debug_str.contains("TRpc"));
}

#[test]
fn test_transport_type_custom() {
    let custom = TransportType::Custom("mqtt".to_string());

    if let TransportType::Custom(name) = custom {
        assert_eq!(name, "mqtt");
    } else {
        panic!("Expected Custom variant");
    }
}

// ============================================================================
// MessageFormat Tests
// ============================================================================

#[test]
fn test_message_format_variants() {
    let formats = vec![
        MessageFormat::Json,
        MessageFormat::MessagePack,
        MessageFormat::Cbor,
        MessageFormat::Custom("proto".to_string()),
    ];

    assert_eq!(formats.len(), 4);
}

#[test]
fn test_message_format_clone() {
    let format = MessageFormat::Json;
    let cloned = format.clone();

    assert_eq!(cloned, format);
}

#[test]
fn test_message_format_debug() {
    let format = MessageFormat::MessagePack;
    let debug_str = format!("{format:?}");

    assert!(debug_str.contains("MessagePack"));
}

#[test]
fn test_message_format_custom() {
    let custom = MessageFormat::Custom("avro".to_string());

    if let MessageFormat::Custom(name) = custom {
        assert_eq!(name, "avro");
    } else {
        panic!("Expected Custom variant");
    }
}

// ============================================================================
// MessagePriority Tests
// ============================================================================

#[test]
fn test_message_priority_variants() {
    let priorities = vec![
        MessagePriority::Low,
        MessagePriority::Normal,
        MessagePriority::High,
        MessagePriority::Critical,
    ];

    assert_eq!(priorities.len(), 4);
}

#[test]
fn test_message_priority_ordering() {
    assert!(MessagePriority::Critical > MessagePriority::High);
    assert!(MessagePriority::High > MessagePriority::Normal);
    assert!(MessagePriority::Normal > MessagePriority::Low);
}

#[test]
fn test_message_priority_default() {
    let priority = MessagePriority::default();
    assert_eq!(priority, MessagePriority::Normal);
}

#[test]
fn test_message_priority_clone() {
    let priority = MessagePriority::High;
    let cloned = priority.clone();

    assert_eq!(cloned, priority);
}

// ============================================================================
// AuthType Tests
// ============================================================================

#[test]
fn test_auth_type_variants() {
    let types = vec![
        AuthType::None,
        AuthType::Bearer,
        AuthType::ApiKey,
        AuthType::MutualTls,
        AuthType::Jwt,
        AuthType::Custom("oauth2".to_string()),
    ];

    assert_eq!(types.len(), 6);
}

#[test]
fn test_auth_type_clone() {
    let auth = AuthType::Bearer;
    let cloned = auth.clone();

    assert!(matches!(cloned, AuthType::Bearer));
}

#[test]
fn test_auth_type_debug() {
    let auth = AuthType::MutualTls;
    let debug_str = format!("{auth:?}");

    assert!(debug_str.contains("MutualTls"));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_transport_http_flow() {
    let transport = TransportType::Http;
    let format = MessageFormat::Json;

    assert!(matches!(transport, TransportType::Http));
    assert_eq!(format, MessageFormat::Json);
}

#[test]
fn test_transport_websocket_flow() {
    let transport = TransportType::TRpc;
    let format = MessageFormat::MessagePack;

    assert!(matches!(transport, TransportType::TRpc));
    assert_eq!(format, MessageFormat::MessagePack);
}

#[test]
fn test_transport_trpc_flow() {
    let transport = TransportType::TRpc;
    let format = MessageFormat::Json;

    assert!(matches!(transport, TransportType::TRpc));
    assert_eq!(format, MessageFormat::Json);
}

#[test]
fn test_auth_bearer_flow() {
    let auth = AuthType::Bearer;

    assert!(matches!(auth, AuthType::Bearer));
}

#[test]
fn test_auth_jwt_flow() {
    let auth = AuthType::Jwt;

    assert!(matches!(auth, AuthType::Jwt));
}

#[test]
fn test_priority_comparison() {
    let critical = MessagePriority::Critical;
    let low = MessagePriority::Low;

    assert!(critical > low);
    assert!(low < critical);
}

#[test]
fn test_format_equality() {
    let json1 = MessageFormat::Json;
    let json2 = MessageFormat::Json;
    let msgpack = MessageFormat::MessagePack;

    assert_eq!(json1, json2);
    assert_ne!(json1, msgpack);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_custom_transport_equality() {
    let custom1 = TransportType::Custom("amqp".to_string());
    let custom2 = TransportType::Custom("amqp".to_string());
    let custom3 = TransportType::Custom("kafka".to_string());

    assert_eq!(custom1, custom2);
    assert_ne!(custom1, custom3);
}

#[test]
fn test_custom_format_equality() {
    let fmt1 = MessageFormat::Custom("proto".to_string());
    let fmt2 = MessageFormat::Custom("proto".to_string());
    let fmt3 = MessageFormat::Custom("thrift".to_string());

    assert_eq!(fmt1, fmt2);
    assert_ne!(fmt1, fmt3);
}

#[test]
fn test_transport_type_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(TransportType::Http);
    set.insert(TransportType::TRpc);
    set.insert(TransportType::Http); // Duplicate

    assert_eq!(set.len(), 2);
}
