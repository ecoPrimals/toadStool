// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_auth_type_none() {
    let auth = AuthType::None;
    assert!(matches!(auth, AuthType::None));
}

#[test]
fn test_auth_type_bearer() {
    let auth = AuthType::Bearer;
    assert!(matches!(auth, AuthType::Bearer));
}

#[test]
fn test_auth_type_custom() {
    let auth = AuthType::Custom("oauth2".to_string());
    if let AuthType::Custom(name) = auth {
        assert_eq!(name, "oauth2");
    } else {
        panic!("Expected Custom auth type");
    }
}

#[test]
fn test_auth_type_api_key() {
    assert!(matches!(AuthType::ApiKey, AuthType::ApiKey));
}

#[test]
fn test_auth_type_mutual_tls() {
    assert!(matches!(AuthType::MutualTls, AuthType::MutualTls));
}

#[test]
fn test_auth_type_jwt() {
    assert!(matches!(AuthType::Jwt, AuthType::Jwt));
}

#[test]
fn test_auth_type_custom_serialization() {
    let auth = AuthType::Custom("saml".to_string());
    let serialized = serde_json::to_string(&auth).expect("Failed to serialize");
    let deserialized: AuthType = serde_json::from_str(&serialized).expect("Failed to deserialize");

    if let AuthType::Custom(name) = deserialized {
        assert_eq!(name, "saml");
    } else {
        panic!("Expected Custom auth type");
    }
}
