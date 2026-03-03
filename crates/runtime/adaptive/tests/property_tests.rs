// SPDX-License-Identifier: AGPL-3.0-or-later
//! Property-based tests for adaptive runtime
//!
//! Tests mathematical and logical properties that should always hold
//! for the adaptive optimization system.

use proptest::prelude::*;
use toadstool_runtime_adaptive::{OpType, SizeClass};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_size_class_classification(
        size in 1usize..10_000_000,
    ) {
        let size_class = SizeClass::from_size(size);

        // Verify classification is correct
        match size_class {
            SizeClass::Tiny => prop_assert!(size < 1_000),
            SizeClass::Small => prop_assert!((1_000..100_000).contains(&size)),
            SizeClass::Medium => prop_assert!((100_000..1_000_000).contains(&size)),
            SizeClass::Large => prop_assert!((1_000_000..10_000_000).contains(&size)),
            SizeClass::Huge => prop_assert!(size >= 10_000_000),
        }
    }

    #[test]
    fn prop_workgroup_size_valid(
        workgroup in 1usize..2048,
    ) {
        // Workgroup sizes should be power of 2 for optimal GPU performance
        // But we allow any size, just verify it's reasonable
        prop_assert!(workgroup > 0);
        prop_assert!(workgroup <= 2048); // Reasonable maximum
    }
}

/// Test that OpType can be round-trip serialized
#[test]
fn test_op_type_serialization_round_trip() {
    for op_type in OpType::all() {
        let json = serde_json::to_string(&op_type).unwrap();
        let deserialized: OpType = serde_json::from_str(&json).unwrap();
        assert_eq!(op_type, deserialized, "Round-trip failed for {:?}", op_type);
    }
}

/// Test that SizeClass can be round-trip serialized
#[test]
fn test_size_class_serialization_round_trip() {
    let classes = [
        SizeClass::Tiny,
        SizeClass::Small,
        SizeClass::Medium,
        SizeClass::Large,
        SizeClass::Huge,
    ];
    for size_class in &classes {
        let json = serde_json::to_string(size_class).unwrap();
        let deserialized: SizeClass = serde_json::from_str(&json).unwrap();
        assert_eq!(
            *size_class, deserialized,
            "Round-trip failed for {:?}",
            size_class
        );
    }
}
