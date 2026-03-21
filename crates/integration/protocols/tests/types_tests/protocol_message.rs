// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

#[test]
fn test_protocol_message_creation() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("request"),
        source: Arc::from("service-a"),
        destination: Some(Arc::from("service-b")),
        payload: serde_json::json!({"action": "compute"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: Some(Uuid::new_v4()),
        reply_to: None,
        ttl: Some(Duration::from_secs(300)),
        priority: MessagePriority::Normal,
    };

    assert_eq!(&*message.source, "service-a");
    assert_eq!(message.destination.as_deref(), Some("service-b"));
    assert_eq!(&*message.message_type, "request");
    assert_eq!(message.priority, MessagePriority::Normal);
}

#[test]
fn test_protocol_message_with_headers() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer token123".to_string());
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("auth_request"),
        source: Arc::from("client"),
        destination: Some(Arc::from("auth-server")),
        payload: serde_json::json!({}),
        headers,
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::High,
    };

    assert_eq!(message.headers.len(), 2);
    assert!(message.headers.contains_key("Authorization"));
}

#[test]
fn test_protocol_message_with_ttl() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("command"),
        source: Arc::from("controller"),
        destination: Some(Arc::from("worker")),
        payload: serde_json::json!({"cmd": "execute"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: Some(Duration::from_secs(60)),
        priority: MessagePriority::High,
    };

    assert!(message.ttl.is_some());
    assert_eq!(message.ttl.unwrap(), Duration::from_secs(60));
}

#[test]
fn test_protocol_message_without_ttl() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("notification"),
        source: Arc::from("notifier"),
        destination: None,
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    assert!(message.ttl.is_none());
}

#[test]
fn test_protocol_message_correlation_id() {
    let corr_id = Uuid::new_v4();
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("reply"),
        source: Arc::from("responder"),
        destination: Some(Arc::from("requester")),
        payload: serde_json::json!({"status": "ok"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: Some(corr_id),
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    assert_eq!(message.correlation_id, Some(corr_id));
}

#[test]
fn test_protocol_message_reply_to() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("request"),
        source: Arc::from("client"),
        destination: Some(Arc::from("server")),
        payload: serde_json::json!({"query": "data"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: Some(Arc::from("client-queue")),
        ttl: Some(Duration::from_secs(300)),
        priority: MessagePriority::Normal,
    };

    assert_eq!(message.reply_to.as_deref(), Some("client-queue"));
}

#[test]
fn test_protocol_message_broadcast() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("broadcast"),
        source: Arc::from("broadcaster"),
        destination: None,
        payload: serde_json::json!({"announcement": "System update"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::High,
    };

    assert!(message.destination.is_none());
    assert_eq!(&*message.message_type, "broadcast");
}

#[test]
fn test_protocol_message_empty_payload() {
    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("ping"),
        source: Arc::from("pinger"),
        destination: Some(Arc::from("target")),
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: Some(Duration::from_secs(30)),
        priority: MessagePriority::Low,
    };

    assert_eq!(message.payload, serde_json::json!({}));
}

#[test]
fn test_protocol_message_complex_payload() {
    let payload = serde_json::json!({
        "workload": {
            "type": "computation",
            "params": {
                "iterations": 1000,
                "precision": "high"
            }
        },
        "resources": {
            "cpu": 4,
            "memory": "8GB"
        }
    });

    let message = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("workload_request"),
        source: Arc::from("scheduler"),
        destination: Some(Arc::from("worker-01")),
        payload,
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: Some(Arc::from("scheduler-queue")),
        ttl: Some(Duration::from_secs(600)),
        priority: MessagePriority::Normal,
    };

    assert!(message.payload.is_object());
    assert!(message.payload["workload"]["type"].is_string());
}

#[test]
fn test_protocol_message_unique_ids() {
    let msg1 = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("test"),
        source: Arc::from("source"),
        destination: None,
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    let msg2 = ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from("test"),
        source: Arc::from("source"),
        destination: None,
        payload: serde_json::json!({}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: None,
        priority: MessagePriority::Normal,
    };

    assert_ne!(msg1.id, msg2.id);
}
