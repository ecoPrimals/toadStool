// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for common utilities

use std::time::{Duration, SystemTime};
use toadstool_common::{
    format_bytes, format_duration, generate_id, StringExt, Timestamp, ToadStoolId,
};

// ============================================================================
// ToadStoolId Tests
// ============================================================================

#[test]
fn test_toadstool_id_new() {
    let id1 = ToadStoolId::new();
    let id2 = ToadStoolId::new();
    assert_ne!(id1.inner(), id2.inner());
}

#[test]
fn test_toadstool_id_default() {
    let id1 = ToadStoolId::default();
    let id2 = ToadStoolId::default();
    assert_ne!(id1.inner(), id2.inner());
}

#[test]
fn test_toadstool_id_inner() {
    let id = ToadStoolId::new();
    let uuid = id.inner();
    assert_eq!(uuid.get_version_num(), 4); // UUID v4
}

#[test]
fn test_toadstool_id_clone() {
    let id1 = ToadStoolId::new();
    let id2 = id1;
    assert_eq!(id1.inner(), id2.inner());
}

#[test]
fn test_toadstool_id_equality() {
    let id1 = ToadStoolId::new();
    let id2 = id1;
    let id3 = ToadStoolId::new();

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

#[test]
fn test_toadstool_id_debug() {
    let id = ToadStoolId::new();
    let debug_str = format!("{id:?}");
    assert!(!debug_str.is_empty());
}

// ============================================================================
// generate_id Tests
// ============================================================================

#[test]
fn test_generate_id_uniqueness() {
    let id1 = generate_id();
    let id2 = generate_id();
    assert_ne!(id1, id2);
}

#[test]
fn test_generate_id_version() {
    let id = generate_id();
    assert_eq!(id.get_version_num(), 4);
}

// ============================================================================
// Timestamp Tests
// ============================================================================

#[test]
fn test_timestamp_now() {
    let ts1 = Timestamp::now();
    let ts2 = Timestamp::now();

    // Timestamps should be close but not necessarily equal
    let diff = ts2
        .inner()
        .duration_since(ts1.inner())
        .unwrap_or(Duration::from_secs(0));
    assert!(diff < Duration::from_secs(1));
}

#[test]
fn test_timestamp_current() {
    let ts = Timestamp::current();
    let system_now = SystemTime::now();

    let diff = system_now
        .duration_since(ts.inner())
        .unwrap_or(Duration::from_secs(0));
    assert!(diff < Duration::from_secs(1));
}

#[test]
fn test_timestamp_inner() {
    let ts = Timestamp::now();
    let system_time = ts.inner();
    assert!(system_time <= SystemTime::now());
}

#[test]
fn test_timestamp_clone() {
    let ts1 = Timestamp::now();
    let ts2 = ts1;
    assert_eq!(ts1.inner(), ts2.inner());
}

#[test]
fn test_timestamp_debug() {
    let ts = Timestamp::now();
    let debug_str = format!("{ts:?}");
    assert!(!debug_str.is_empty());
}

// ============================================================================
// format_bytes Tests
// ============================================================================

#[test]
fn test_format_bytes_zero() {
    assert_eq!(format_bytes(0), "0 B");
}

#[test]
fn test_format_bytes_bytes() {
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(1023), "1023 B");
}

#[test]
fn test_format_bytes_kilobytes() {
    assert_eq!(format_bytes(1024), "1.0 KB");
    assert_eq!(format_bytes(1536), "1.5 KB");
    assert_eq!(format_bytes(2048), "2.0 KB");
}

#[test]
fn test_format_bytes_megabytes() {
    assert_eq!(format_bytes(1_048_576), "1.0 MB");
    assert_eq!(format_bytes(1_572_864), "1.5 MB");
}

#[test]
fn test_format_bytes_gigabytes() {
    assert_eq!(format_bytes(1_073_741_824), "1.0 GB");
    assert_eq!(format_bytes(2_147_483_648), "2.0 GB");
}

#[test]
fn test_format_bytes_terabytes() {
    assert_eq!(format_bytes(1_099_511_627_776), "1.0 TB");
}

#[test]
fn test_format_bytes_petabytes() {
    assert_eq!(format_bytes(1_125_899_906_842_624), "1.0 PB");
}

// ============================================================================
// format_duration Tests
// ============================================================================

#[test]
fn test_format_duration_seconds_only() {
    assert_eq!(format_duration(Duration::from_secs(0)), "0s");
    assert_eq!(format_duration(Duration::from_secs(30)), "30s");
    assert_eq!(format_duration(Duration::from_secs(59)), "59s");
}

#[test]
fn test_format_duration_minutes_and_seconds() {
    assert_eq!(format_duration(Duration::from_secs(60)), "1m 0s");
    assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
    assert_eq!(format_duration(Duration::from_secs(150)), "2m 30s");
}

#[test]
fn test_format_duration_hours_minutes_seconds() {
    assert_eq!(format_duration(Duration::from_secs(3600)), "1h 0m 0s");
    assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
    assert_eq!(format_duration(Duration::from_secs(7200)), "2h 0m 0s");
    assert_eq!(format_duration(Duration::from_secs(7325)), "2h 2m 5s");
}

#[test]
fn test_format_duration_large() {
    assert_eq!(format_duration(Duration::from_secs(86400)), "24h 0m 0s");
    assert_eq!(format_duration(Duration::from_secs(90061)), "25h 1m 1s");
}

// ============================================================================
// StringExt Tests
// ============================================================================

#[test]
fn test_string_ext_is_blank_empty() {
    let s = "";
    assert!(s.is_blank());
}

#[test]
fn test_string_ext_is_blank_whitespace() {
    assert!("   ".is_blank());
    assert!("\t\t".is_blank());
    assert!("\n\n".is_blank());
    assert!(" \t\n ".is_blank());
}

#[test]
fn test_string_ext_is_blank_not_blank() {
    assert!(!"hello".is_blank());
    assert!(!"  hello  ".is_blank());
    assert!(!"a".is_blank());
}

#[test]
fn test_string_ext_is_blank_string_type() {
    let s = String::from("   ");
    assert!(s.is_blank());

    let s = String::from("hello");
    assert!(!s.is_blank());
}

#[test]
fn test_string_ext_truncate_to_short_string() {
    let s = "hello";
    assert_eq!(s.truncate_to(10), "hello");
    assert_eq!(s.truncate_to(5), "hello");
}

#[test]
fn test_string_ext_truncate_to_exact_length() {
    let s = "hello";
    assert_eq!(s.truncate_to(5), "hello");
}

#[test]
fn test_string_ext_truncate_to_long_string() {
    let s = "hello world this is a long string";
    let truncated = s.truncate_to(10);
    assert_eq!(truncated, "hello w...");
    assert_eq!(truncated.len(), 10);
}

#[test]
fn test_string_ext_truncate_to_zero() {
    let s = "hello";
    let truncated = s.truncate_to(0);
    assert_eq!(truncated, "...");
}

#[test]
fn test_string_ext_truncate_to_three() {
    let s = "hello";
    let truncated = s.truncate_to(3);
    assert_eq!(truncated, "...");
}

#[test]
fn test_string_ext_truncate_to_string_type() {
    let s = String::from("hello world");
    let truncated = s.truncate_to(8);
    assert_eq!(truncated, "hello...");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_id_generation_batch() {
    let ids: Vec<_> = (0..100).map(|_| generate_id()).collect();

    // All IDs should be unique
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert_ne!(ids[i], ids[j]);
        }
    }
}

#[test]
fn test_timestamp_ordering() {
    let ts1 = Timestamp::now();
    // ✅ MODERN: Immediate execution (sleep removed)
    let ts2 = Timestamp::now();

    assert!(ts2.inner() >= ts1.inner());
}

#[test]
fn test_format_bytes_edge_cases() {
    // Test boundary conditions
    assert_eq!(format_bytes(1023), "1023 B");
    assert_eq!(format_bytes(1024), "1.0 KB");
    assert_eq!(format_bytes(1025), "1.0 KB");
}

#[test]
fn test_string_ext_chaining() {
    let s = "  hello world  ";
    // Test that is_blank and truncate_to work on the same string
    assert!(!s.is_blank());
    let truncated = s.truncate_to(10);
    assert_eq!(truncated.len(), 10);
}
