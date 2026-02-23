//! Tamper-evident audit logging for secure enclave operations
//!
//! Provides cryptographically-secured audit trails for all security-relevant
//! operations in the secure enclave.

use crate::error::{Error, Result};
use blake3::Hash;
use std::time::{SystemTime, UNIX_EPOCH};

/// Audit event type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditEventType {
    /// Memory region allocated
    MemoryAllocated,

    /// Memory region deallocated
    MemoryDeallocated,

    /// Key stored in key store
    KeyStored,

    /// Key wiped from key store
    KeyWiped,

    /// Data decompressed
    DataDecompressed,

    /// Processing started
    ProcessingStarted,

    /// Processing completed
    ProcessingCompleted,

    /// Security violation detected
    SecurityViolation,
}

impl AuditEventType {
    /// Get string representation
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::MemoryAllocated => "memory_allocated",
            Self::MemoryDeallocated => "memory_deallocated",
            Self::KeyStored => "key_stored",
            Self::KeyWiped => "key_wiped",
            Self::DataDecompressed => "data_decompressed",
            Self::ProcessingStarted => "processing_started",
            Self::ProcessingCompleted => "processing_completed",
            Self::SecurityViolation => "security_violation",
        }
    }
}

/// Audit event with tamper-evident properties
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Event sequence number (monotonic)
    pub sequence: u64,

    /// Timestamp (microseconds since epoch)
    pub timestamp_micros: u64,

    /// Event type
    pub event_type: AuditEventType,

    /// Event details (JSON-serializable)
    pub details: String,

    /// Hash of previous event (tamper detection)
    pub prev_hash: Option<Hash>,

    /// Hash of this event (tamper detection)
    pub event_hash: Hash,
}

impl AuditEvent {
    /// Create a new audit event
    fn new(
        sequence: u64,
        event_type: AuditEventType,
        details: String,
        prev_hash: Option<Hash>,
    ) -> Self {
        // Use saturating conversion to handle potential overflow (years 2262+)
        #[allow(clippy::cast_possible_truncation)]
        // Intentional: saturates at u64::MAX for far future timestamps
        let timestamp_micros = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros()
            .min(u128::from(u64::MAX)) as u64;

        // Compute event hash (includes previous hash for chain integrity)
        let event_hash =
            Self::compute_hash(sequence, timestamp_micros, &event_type, &details, prev_hash);

        Self {
            sequence,
            timestamp_micros,
            event_type,
            details,
            prev_hash,
            event_hash,
        }
    }

    /// Compute hash of event data
    fn compute_hash(
        sequence: u64,
        timestamp_micros: u64,
        event_type: &AuditEventType,
        details: &str,
        prev_hash: Option<Hash>,
    ) -> Hash {
        let mut hasher = blake3::Hasher::new();

        // Include sequence number
        hasher.update(&sequence.to_le_bytes());

        // Include timestamp
        hasher.update(&timestamp_micros.to_le_bytes());

        // Include event type
        hasher.update(event_type.as_str().as_bytes());

        // Include details
        hasher.update(details.as_bytes());

        // Include previous hash (chain integrity)
        if let Some(prev) = prev_hash {
            hasher.update(prev.as_bytes());
        }

        hasher.finalize()
    }

    /// Verify event integrity
    #[must_use]
    pub fn verify(&self) -> bool {
        let computed_hash = Self::compute_hash(
            self.sequence,
            self.timestamp_micros,
            &self.event_type,
            &self.details,
            self.prev_hash,
        );

        computed_hash == self.event_hash
    }
}

/// Tamper-evident audit logger
#[derive(Debug)]
pub struct AuditLogger {
    /// Event log (append-only)
    events: Vec<AuditEvent>,

    /// Next sequence number
    next_sequence: u64,
}

impl AuditLogger {
    /// Create a new audit logger
    #[must_use]
    pub const fn new() -> Self {
        Self {
            events: Vec::new(),
            next_sequence: 0,
        }
    }

    /// Log an audit event
    ///
    /// # Arguments
    ///
    /// * `event_type` - Type of event
    /// * `details` - Event details (should be JSON for structured logging)
    ///
    /// # Returns
    ///
    /// Returns the sequence number of the logged event
    ///
    /// # Errors
    ///
    /// Currently infallible, but returns Result for future error handling.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut logger = AuditLogger::new();
    ///
    /// logger.log(
    ///     AuditEventType::MemoryAllocated,
    ///     r#"{"size": 4096, "purpose": "decompression"}"#,
    /// )?;
    /// ```
    pub fn log(&mut self, event_type: AuditEventType, details: impl Into<String>) -> Result<u64> {
        let sequence = self.next_sequence;
        let prev_hash = self.events.last().map(|e| e.event_hash);

        let event = AuditEvent::new(sequence, event_type, details.into(), prev_hash);

        self.events.push(event);
        self.next_sequence += 1;

        Ok(sequence)
    }

    /// Get all audit events
    #[must_use]
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Verify audit log integrity
    ///
    /// Checks:
    /// 1. Each event's hash is valid
    /// 2. Chain integrity (each event references previous)
    /// 3. Sequence numbers are monotonic
    ///
    /// # Errors
    ///
    /// Returns error if tampering detected (invalid hashes, broken chain, or sequence issues).
    pub fn verify_integrity(&self) -> Result<()> {
        if self.events.is_empty() {
            return Ok(());
        }

        for (i, event) in self.events.iter().enumerate() {
            // Verify event hash
            if !event.verify() {
                return Err(Error::audit_log(format!(
                    "Event {} hash verification failed",
                    event.sequence
                )));
            }

            // Verify sequence number
            if event.sequence != i as u64 {
                return Err(Error::audit_log(format!(
                    "Sequence number mismatch at index {}: expected {}, got {}",
                    i, i, event.sequence
                )));
            }

            // Verify chain integrity
            if i > 0 {
                let prev_event = &self.events[i - 1];
                match event.prev_hash {
                    Some(prev_hash) if prev_hash == prev_event.event_hash => {
                        // OK: Chain is intact
                    }
                    Some(_) => {
                        return Err(Error::audit_log(format!(
                            "Chain broken at event {}: previous hash mismatch",
                            event.sequence
                        )));
                    }
                    None => {
                        return Err(Error::audit_log(format!(
                            "Chain broken at event {}: missing previous hash",
                            event.sequence
                        )));
                    }
                }
            } else {
                // First event should have no previous hash
                if event.prev_hash.is_some() {
                    return Err(Error::audit_log(
                        "First event has previous hash (should be None)".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get number of events in log
    #[must_use]
    pub const fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if log is empty
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Clear the audit log (for testing only)
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.events.clear();
        self.next_sequence = 0;
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
}
