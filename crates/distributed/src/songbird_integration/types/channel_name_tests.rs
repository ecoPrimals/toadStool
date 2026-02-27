use super::*;

#[test]
fn test_channel_name_capability_update() {
    let msg = SongbirdBroadcastMessage::CapabilityUpdate {
        node_id: "n1".to_string(),
        capabilities: NodeCapabilities {
            cpu_cores: 4.0,
            memory_gb: 8.0,
            storage_gb: 100.0,
            gpu_count: 0,
            specialized_hardware: vec![],
            software_capabilities: vec![],
        },
        timestamp: std::time::SystemTime::now(),
    };
    assert_eq!(msg.channel_name(), "capability-updates");
}

#[test]
fn test_channel_name_health_update() {
    let msg = SongbirdBroadcastMessage::HealthUpdate {
        node_id: "n2".to_string(),
        health_status: "healthy".to_string(),
        timestamp: std::time::SystemTime::now(),
    };
    assert_eq!(msg.channel_name(), "health-updates");
}

#[test]
fn test_channel_name_custom_message() {
    let msg = SongbirdBroadcastMessage::CustomMessage {
        message_type: "job-complete".to_string(),
        payload: serde_json::json!({}),
        timestamp: std::time::SystemTime::now(),
    };
    assert_eq!(msg.channel_name(), "job-complete");
}
