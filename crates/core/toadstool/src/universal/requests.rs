//! # Request and Response Types
//!
//! Types for inter-primal communication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::types::PrimalContext;

/// Primal API endpoints
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimalEndpoints {
    /// Primary API endpoint
    pub primary: String,
    /// Health check endpoint
    pub health: String,
    /// Metrics endpoint
    pub metrics: Option<String>,
    /// Admin endpoint
    pub admin: Option<String>,
    /// Real-time events: use JSON-RPC 2.0 polling (biomeOS/songbird). WebSocket removed — deprecated (ring C-FFI).
    pub events_endpoint: Option<String>,
    /// Custom endpoints
    pub custom: HashMap<String, String>,
}

/// Inter-primal request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalRequest {
    /// Request ID
    pub id: Uuid,
    /// Source primal
    pub source: String,
    /// Target primal
    pub target: String,
    /// Request type
    pub request_type: String,
    /// Request payload
    pub payload: serde_json::Value,
    /// Request context
    pub context: PrimalContext,
    /// Request metadata
    pub metadata: HashMap<String, String>,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Response status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseStatus {
    /// Success
    Success,
    /// Error with details
    Error { code: String, message: String },
    /// Timeout
    Timeout,
    /// Service unavailable
    ServiceUnavailable,
}

/// Inter-primal response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalResponse {
    /// Request ID this response is for
    pub request_id: Uuid,
    /// Response status
    pub status: ResponseStatus,
    /// Response payload
    pub payload: serde_json::Value,
    /// Response metadata
    pub metadata: HashMap<String, String>,
    /// Response timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal::types::NetworkLocation;
    use crate::universal::types::PrimalContext;
    use crate::universal::types::SecurityLevel;
    use std::collections::HashMap;

    fn sample_primal_context() -> PrimalContext {
        PrimalContext {
            user_id: "user-1".to_string(),
            device_id: "device-1".to_string(),
            session_id: "session-1".to_string(),
            network_location: NetworkLocation {
                ip_address: "127.0.0.1".to_string(),
                subnet: None,
                network_id: None,
                geo_location: None,
            },
            security_level: SecurityLevel::Standard,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_primal_endpoints_construction() {
        let mut custom = HashMap::new();
        custom.insert("custom1".to_string(), "http://custom1".to_string());
        let eps = PrimalEndpoints {
            primary: "http://primary".to_string(),
            health: "http://health".to_string(),
            metrics: Some("http://metrics".to_string()),
            admin: Some("http://admin".to_string()),
            websocket: Some("ws://websocket".to_string()),
            custom,
        };
        assert_eq!(eps.primary, "http://primary");
        assert_eq!(eps.health, "http://health");
        assert_eq!(eps.metrics, Some("http://metrics".to_string()));
        assert_eq!(
            eps.custom.get("custom1"),
            Some(&"http://custom1".to_string())
        );
    }

    #[test]
    fn test_primal_endpoints_equality() {
        let a = PrimalEndpoints {
            primary: "http://a".to_string(),
            health: "http://health".to_string(),
            metrics: None,
            admin: None,
            websocket: None,
            custom: HashMap::new(),
        };
        let b = PrimalEndpoints {
            primary: "http://a".to_string(),
            health: "http://health".to_string(),
            metrics: None,
            admin: None,
            websocket: None,
            custom: HashMap::new(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_primal_endpoints_serialization_round_trip() {
        let eps = PrimalEndpoints {
            primary: "http://api".to_string(),
            health: "http://api/health".to_string(),
            metrics: Some("http://api/metrics".to_string()),
            admin: None,
            websocket: None,
            custom: HashMap::new(),
        };
        let json = serde_json::to_string(&eps).expect("serialize");
        let parsed: PrimalEndpoints = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(eps, parsed);
    }

    #[test]
    fn test_primal_request_construction() {
        let mut metadata = HashMap::new();
        metadata.insert("trace_id".to_string(), "abc123".to_string());
        let req = PrimalRequest {
            id: Uuid::new_v4(),
            source: "toadstool".to_string(),
            target: "bear-dog".to_string(),
            request_type: "auth-check".to_string(),
            payload: serde_json::json!({"token": "xyz"}),
            context: sample_primal_context(),
            metadata,
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(req.source, "toadstool");
        assert_eq!(req.target, "bear-dog");
        assert_eq!(req.request_type, "auth-check");
    }

    #[test]
    fn test_response_status_success() {
        let status = ResponseStatus::Success;
        assert!(matches!(status, ResponseStatus::Success));
    }

    #[test]
    fn test_response_status_error() {
        let status = ResponseStatus::Error {
            code: "E001".to_string(),
            message: "Something failed".to_string(),
        };
        if let ResponseStatus::Error { code, message } = status {
            assert_eq!(code, "E001");
            assert_eq!(message, "Something failed");
        } else {
            panic!("expected Error variant");
        }
    }

    #[test]
    fn test_response_status_timeout_and_service_unavailable() {
        let timeout = ResponseStatus::Timeout;
        let unavailable = ResponseStatus::ServiceUnavailable;
        assert!(matches!(timeout, ResponseStatus::Timeout));
        assert!(matches!(unavailable, ResponseStatus::ServiceUnavailable));
    }

    #[test]
    fn test_response_status_equality() {
        assert_eq!(ResponseStatus::Success, ResponseStatus::Success);
        assert_eq!(ResponseStatus::Timeout, ResponseStatus::Timeout);
        assert_eq!(
            ResponseStatus::Error {
                code: "E1".to_string(),
                message: "m".to_string(),
            },
            ResponseStatus::Error {
                code: "E1".to_string(),
                message: "m".to_string(),
            }
        );
    }

    #[test]
    fn test_primal_response_construction() {
        let mut metadata = HashMap::new();
        metadata.insert("duration_ms".to_string(), "42".to_string());
        let resp = PrimalResponse {
            request_id: Uuid::new_v4(),
            status: ResponseStatus::Success,
            payload: serde_json::json!({"result": "ok"}),
            metadata,
            timestamp: chrono::Utc::now(),
        };
        assert!(matches!(resp.status, ResponseStatus::Success));
        assert_eq!(resp.payload["result"], "ok");
    }

    #[test]
    fn test_primal_response_serialization_round_trip() {
        let resp = PrimalResponse {
            request_id: Uuid::new_v4(),
            status: ResponseStatus::Success,
            payload: serde_json::json!({"data": [1, 2, 3]}),
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&resp).expect("serialize");
        let parsed: PrimalResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(resp.request_id, parsed.request_id);
        assert!(matches!(parsed.status, ResponseStatus::Success));
        assert_eq!(parsed.payload["data"], serde_json::json!([1, 2, 3]));
    }
}
