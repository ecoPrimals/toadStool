// SPDX-License-Identifier: AGPL-3.0-or-later
//! Type Adapters for Security Integration
//!
//! Converts between generic SecurityProvider types and Security-specific types.
//! This allows Security to work with the generic SecurityProvider interface.

use toadstool::error::ToadStoolResult;
use toadstool_common::interned_strings::capabilities;

use crate::security::types::{
    EncryptionOperation, EncryptionRequest, EncryptionResponse, KeyManagementRequest,
    KeyManagementResponse, KeyOperation, SecurityLevel,
};
use crate::security_provider::EncryptionOptions;
use crate::security_provider::types::{
    EncryptionMetadata, EncryptionResult, PermissionRequest, ProviderMetadata, SecurityPermission,
    SecurityProof, SignatureAlgorithm,
};

/// Convert generic PermissionRequest to Security-specific request
pub fn to_security_permission_request(_request: &PermissionRequest) -> KeyManagementRequest {
    // Pending: Full conversion when security KeyManagementRequest supports
    // PermissionRequest fields (target, scope, operations). Currently KeyOperation::Generate
    // and SecurityLevel are the only mappable parts.
    KeyManagementRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: KeyOperation::Generate,
        key_id: None,
        security_level: Some(SecurityLevel::Enhanced),
    }
}

/// Convert Security permission response to generic SecurityPermission
pub fn from_security_permission(
    _response: &KeyManagementResponse,
    request: &PermissionRequest,
) -> ToadStoolResult<SecurityPermission> {
    let now = std::time::SystemTime::now();

    // NOTE: Using default permission structure - full extraction in future enhancement
    Ok(SecurityPermission {
        permission_id: uuid::Uuid::new_v4(),
        holder_id: request.requester_id.clone(),
        target: request.target.clone(),
        scope: request.scope.clone(),
        valid_from: now,
        valid_until: now + request.validity_duration,
        proof: SecurityProof {
            signature: vec![],
            algorithm: SignatureAlgorithm::EcdsaP256,
            public_key_id: format!("{}-key", capabilities::CRYPTO),
            signed_at: now,
        },
        provider_metadata: ProviderMetadata {
            provider_id: uuid::Uuid::new_v4().to_string(),
            provider_type: capabilities::CRYPTO.to_string(),
            provider_version: "2.0.0".to_string(),
            metadata: std::collections::HashMap::new(),
        },
    })
}

/// Convert generic EncryptionOptions to Security-specific request
pub fn to_security_encryption_request(
    data: &[u8],
    _options: Option<EncryptionOptions>,
) -> EncryptionRequest {
    EncryptionRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: EncryptionOperation::Encrypt,
        data: data.to_vec(),
        key_id: None,
        algorithm: Some("AES-256-GCM".to_string()),
        security_level: SecurityLevel::Standard,
    }
}

/// Convert Security encryption response to generic EncryptionResult
pub fn from_security_encryption_response(response: EncryptionResponse) -> EncryptionResult {
    // Extract IV and auth tag from metadata if present
    let metadata_obj = response.metadata.as_object();
    let iv = metadata_obj
        .and_then(|m| m.get("iv"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect()
        });

    let auth_tag = metadata_obj
        .and_then(|m| m.get("auth_tag"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect()
        });

    EncryptionResult {
        ciphertext: response.data,
        iv,
        auth_tag,
        metadata: EncryptionMetadata {
            algorithm: response.algorithm,
            key_id: response.key_id,
            encrypted_at: std::time::SystemTime::now(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_request_conversion() {
        let data = b"test data";
        let request = to_security_encryption_request(data, None);

        assert_eq!(request.data, data);
        assert_eq!(request.algorithm, Some("AES-256-GCM".to_string()));
    }

    #[test]
    fn test_encryption_response_conversion() {
        let response = EncryptionResponse {
            request_id: uuid::Uuid::new_v4(),
            data: vec![1, 2, 3, 4],
            key_id: "test-key".to_string(),
            algorithm: "AES-256-GCM".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = from_security_encryption_response(response);

        assert_eq!(result.ciphertext, vec![1, 2, 3, 4]);
        assert_eq!(result.metadata.algorithm, "AES-256-GCM");
        assert_eq!(result.metadata.key_id, "test-key");
    }
}
