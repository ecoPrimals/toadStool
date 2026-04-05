// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_message_priority_low() {
    let priority = MessagePriority::Low;
    assert_eq!(priority, MessagePriority::Low);
}

#[test]
fn test_message_priority_normal() {
    let priority = MessagePriority::Normal;
    assert_eq!(priority, MessagePriority::Normal);
}

#[test]
fn test_message_priority_high() {
    let priority = MessagePriority::High;
    assert_eq!(priority, MessagePriority::High);
}

#[test]
fn test_message_priority_critical() {
    let priority = MessagePriority::Critical;
    assert_eq!(priority, MessagePriority::Critical);
}

#[test]
fn test_message_priority_emergency() {
    let priority = MessagePriority::Emergency;
    assert_eq!(priority, MessagePriority::Emergency);
}

#[test]
fn test_message_priority_ordering() {
    assert!(MessagePriority::Low < MessagePriority::Normal);
    assert!(MessagePriority::Normal < MessagePriority::High);
    assert!(MessagePriority::High < MessagePriority::Critical);
    assert!(MessagePriority::Critical < MessagePriority::Emergency);
}

#[test]
fn test_message_priority_default() {
    let priority = MessagePriority::default();
    assert_eq!(priority, MessagePriority::Normal);
}

#[test]
fn test_message_priority_comparison() {
    assert!(MessagePriority::Emergency > MessagePriority::Critical);
    assert!(MessagePriority::Critical > MessagePriority::High);
    assert!(MessagePriority::High > MessagePriority::Normal);
    assert!(MessagePriority::Normal > MessagePriority::Low);
}

#[test]
fn test_message_priority_serialization() {
    let priority = MessagePriority::Critical;
    let serialized = serde_json::to_string(&priority).expect("Failed to serialize");
    let deserialized: MessagePriority =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(priority, deserialized);
}
