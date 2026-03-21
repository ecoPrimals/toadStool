// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn test_message_format_json() {
    let format = MessageFormat::Json;
    assert_eq!(format, MessageFormat::Json);
}

#[test]
fn test_message_format_messagepack() {
    let format = MessageFormat::MessagePack;
    assert_eq!(format, MessageFormat::MessagePack);
}

#[test]
fn test_message_format_cbor() {
    let format = MessageFormat::Cbor;
    assert_eq!(format, MessageFormat::Cbor);
}

#[test]
fn test_message_format_custom() {
    let format = MessageFormat::Custom("protobuf".to_string());
    if let MessageFormat::Custom(name) = format {
        assert_eq!(name, "protobuf");
    } else {
        panic!("Expected Custom format");
    }
}

#[test]
fn test_message_format_messagepack_serialization() {
    let format = MessageFormat::MessagePack;
    let serialized = serde_json::to_string(&format).expect("Failed to serialize");
    let deserialized: MessageFormat =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(format, deserialized);
}

#[test]
fn test_message_format_cbor_serialization() {
    let format = MessageFormat::Cbor;
    let serialized = serde_json::to_string(&format).expect("Failed to serialize");
    let deserialized: MessageFormat =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(format, deserialized);
}

#[test]
fn test_message_format_custom_serialization() {
    let format = MessageFormat::Custom("avro".to_string());
    let serialized = serde_json::to_string(&format).expect("Failed to serialize");
    let deserialized: MessageFormat =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let MessageFormat::Custom(name) = deserialized {
        assert_eq!(name, "avro");
    } else {
        panic!("Expected Custom format");
    }
}
