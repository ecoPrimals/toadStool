// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_protocol_error_connection() {
    let error = ProtocolError::Connection("Connection refused".to_string());
    assert!(error.to_string().contains("Connection failed"));
}

#[test]
fn test_protocol_error_authentication() {
    let error = ProtocolError::Authentication("Invalid credentials".to_string());
    assert!(error.to_string().contains("Authentication failed"));
}

#[test]
fn test_protocol_error_timeout() {
    let error = ProtocolError::Timeout("Request timeout after 30s".to_string());
    assert!(error.to_string().contains("Timeout"));
}

#[test]
fn test_protocol_error_authorization() {
    let error = ProtocolError::Authorization("Access denied".to_string());
    assert!(error.to_string().contains("Authorization failed"));
}

#[test]
fn test_protocol_error_negotiation() {
    let error = ProtocolError::Negotiation("Protocol version mismatch".to_string());
    assert!(error.to_string().contains("Protocol negotiation failed"));
}

#[test]
fn test_protocol_error_serialization() {
    let error = ProtocolError::Serialization("Invalid JSON".to_string());
    assert!(error.to_string().contains("Serialization error"));
}

#[test]
fn test_protocol_error_transport() {
    let error = ProtocolError::Transport("Network unreachable".to_string());
    assert!(error.to_string().contains("Transport error"));
}

#[test]
fn test_protocol_error_discovery() {
    let error = ProtocolError::Discovery("Service not found".to_string());
    assert!(error.to_string().contains("Service discovery error"));
}

#[test]
fn test_protocol_error_routing() {
    let error = ProtocolError::Routing("No route to destination".to_string());
    assert!(error.to_string().contains("Message routing error"));
}

#[test]
fn test_protocol_error_internal() {
    let error = ProtocolError::Internal("Unexpected condition".to_string());
    assert!(error.to_string().contains("Internal error"));
}
