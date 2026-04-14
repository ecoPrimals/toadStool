// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_create_logger() {
    let logger = AuditLogger::new();
    assert_eq!(logger.len(), 0);
    assert!(logger.is_empty());
}

#[test]
fn test_log_event() {
    let mut logger = AuditLogger::new();

    let seq = logger
        .log(AuditEventType::MemoryAllocated, r#"{"size": 4096}"#)
        .unwrap();

    assert_eq!(seq, 0);
    assert_eq!(logger.len(), 1);
    assert!(!logger.is_empty());
}

#[test]
fn test_event_verification() {
    let mut logger = AuditLogger::new();

    logger.log(AuditEventType::MemoryAllocated, "test").unwrap();

    let event = &logger.events()[0];
    assert!(event.verify());
}

#[test]
fn test_chain_integrity() {
    let mut logger = AuditLogger::new();

    // Log multiple events
    logger
        .log(AuditEventType::MemoryAllocated, "event 1")
        .unwrap();
    logger.log(AuditEventType::KeyStored, "event 2").unwrap();
    logger
        .log(AuditEventType::ProcessingStarted, "event 3")
        .unwrap();

    // Verify integrity
    assert!(logger.verify_integrity().is_ok());

    // Check chain
    assert_eq!(
        logger.events()[1].prev_hash,
        Some(logger.events()[0].event_hash)
    );
    assert_eq!(
        logger.events()[2].prev_hash,
        Some(logger.events()[1].event_hash)
    );
}

#[test]
fn test_sequence_numbers() {
    let mut logger = AuditLogger::new();

    let seq1 = logger.log(AuditEventType::MemoryAllocated, "1").unwrap();
    let seq2 = logger.log(AuditEventType::MemoryAllocated, "2").unwrap();
    let seq3 = logger.log(AuditEventType::MemoryAllocated, "3").unwrap();

    assert_eq!(seq1, 0);
    assert_eq!(seq2, 1);
    assert_eq!(seq3, 2);
}

#[test]
fn test_tamper_detection_modified_event() {
    let mut logger = AuditLogger::new();

    logger
        .log(AuditEventType::MemoryAllocated, "original")
        .unwrap();

    // Tamper with event details
    logger.events[0].details = "tampered".to_string();

    // Verification should fail
    assert!(!logger.events()[0].verify());
    assert!(logger.verify_integrity().is_err());
}

#[test]
fn test_tamper_detection_broken_chain() {
    let mut logger = AuditLogger::new();

    logger.log(AuditEventType::MemoryAllocated, "1").unwrap();
    logger.log(AuditEventType::KeyStored, "2").unwrap();

    // Break the chain by modifying first event
    logger.events[0].details = "tampered".to_string();

    // Chain integrity check should fail
    assert!(logger.verify_integrity().is_err());
}

#[test]
fn test_event_types() {
    let types = [
        AuditEventType::MemoryAllocated,
        AuditEventType::MemoryDeallocated,
        AuditEventType::KeyStored,
        AuditEventType::KeyWiped,
        AuditEventType::DataDecompressed,
        AuditEventType::ProcessingStarted,
        AuditEventType::ProcessingCompleted,
        AuditEventType::SecurityViolation,
    ];

    for event_type in &types {
        assert!(!event_type.as_str().is_empty());
    }
}

#[test]
fn test_security_violation_logging() {
    let mut logger = AuditLogger::new();

    logger
        .log(
            AuditEventType::SecurityViolation,
            r#"{"violation": "unauthorized_access", "severity": "high"}"#,
        )
        .unwrap();

    let event = &logger.events()[0];
    assert_eq!(event.event_type, AuditEventType::SecurityViolation);
    assert!(event.details.contains("unauthorized_access"));
}

#[test]
fn test_high_volume_logging() {
    let mut logger = AuditLogger::new();

    // Log 1000 events
    for i in 0..1000 {
        logger
            .log(AuditEventType::MemoryAllocated, format!("event {i}"))
            .unwrap();
    }

    assert_eq!(logger.len(), 1000);
    assert!(logger.verify_integrity().is_ok());
}
