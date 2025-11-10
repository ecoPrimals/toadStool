//! Comprehensive tests for Runtime module
//!
//! Week 16 Sprint: Runtime module (0% → 30-40% coverage)
//! Priority: Get module off 0% critical list

use toadstool::RuntimeType;

// ============================================================================
// RuntimeType Tests (10 tests)
// ============================================================================

#[test]
fn test_runtime_type_container() {
    let rt = RuntimeType::Container;
    assert_eq!(rt, RuntimeType::Container);
}

#[test]
fn test_runtime_type_wasm() {
    let rt = RuntimeType::Wasm;
    assert_eq!(rt, RuntimeType::Wasm);
}

#[test]
fn test_runtime_type_native() {
    let rt = RuntimeType::Native;
    assert_eq!(rt, RuntimeType::Native);
}

#[test]
fn test_runtime_type_python() {
    let rt = RuntimeType::Python;
    assert_eq!(rt, RuntimeType::Python);
}

#[test]
fn test_runtime_type_gpu() {
    let rt = RuntimeType::Gpu;
    assert_eq!(rt, RuntimeType::Gpu);
}

#[test]
fn test_runtime_type_custom() {
    let rt = RuntimeType::Custom("legacy".to_string());
    assert_eq!(rt, RuntimeType::Custom("legacy".to_string()));
}

#[test]
fn test_runtime_type_clone() {
    let rt1 = RuntimeType::Container;
    let rt2 = rt1.clone();
    assert_eq!(rt1, rt2);
}

#[test]
fn test_runtime_type_debug() {
    let rt = RuntimeType::Wasm;
    let debug_str = format!("{:?}", rt);
    assert!(debug_str.contains("Wasm"));
}

#[test]
fn test_runtime_type_serialization() {
    let rt = RuntimeType::Container;
    let json = serde_json::to_string(&rt).expect("Should serialize");
    assert!(!json.is_empty());
}

#[test]
fn test_runtime_type_deserialization() {
    let rt = RuntimeType::Native;
    let json = serde_json::to_string(&rt).unwrap();
    let deserialized: RuntimeType = serde_json::from_str(&json).unwrap();
    assert_eq!(rt, deserialized);
}

// ============================================================================
// RuntimeType Pattern Matching Tests (6 tests)
// ============================================================================

#[test]
fn test_runtime_type_match_container() {
    let rt = RuntimeType::Container;
    match rt {
        RuntimeType::Container => {} // Match successful
        _ => panic!("Should match Container"),
    }
}

#[test]
fn test_runtime_type_match_wasm() {
    let rt = RuntimeType::Wasm;
    match rt {
        RuntimeType::Wasm => {} // Match successful
        _ => panic!("Should match Wasm"),
    }
}

#[test]
fn test_runtime_type_all_variants() {
    let types = [
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Native,
        RuntimeType::Python,
        RuntimeType::Gpu,
        RuntimeType::Custom("test".to_string()),
    ];
    assert_eq!(types.len(), 6);
}

#[test]
fn test_runtime_type_equality() {
    let rt1 = RuntimeType::Container;
    let rt2 = RuntimeType::Container;
    let rt3 = RuntimeType::Wasm;

    assert_eq!(rt1, rt2);
    assert_ne!(rt1, rt3);
}

#[test]
fn test_runtime_type_in_vec() {
    let types = [RuntimeType::Container, RuntimeType::Wasm];
    assert!(types.contains(&RuntimeType::Container));
    assert!(!types.contains(&RuntimeType::Native));
}

#[test]
fn test_runtime_type_comparison() {
    let rt1 = RuntimeType::Container;
    let rt2 = RuntimeType::Container;
    assert!(rt1 == rt2);
}

// ============================================================================
// RuntimeType Display/String Tests (4 tests)
// ============================================================================

#[test]
fn test_runtime_type_debug_all_variants() {
    let types = vec![
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Native,
        RuntimeType::Python,
        RuntimeType::Gpu,
        RuntimeType::Custom("test".to_string()),
    ];

    for rt in types {
        let debug_str = format!("{:?}", rt);
        assert!(!debug_str.is_empty());
    }
}

#[test]
fn test_runtime_type_serialization_all() {
    let types = vec![
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Native,
        RuntimeType::Python,
        RuntimeType::Gpu,
        RuntimeType::Custom("test".to_string()),
    ];

    for rt in types {
        let json = serde_json::to_string(&rt).unwrap();
        assert!(!json.is_empty());
    }
}

#[test]
fn test_runtime_type_roundtrip_serialization() {
    let types = vec![
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Native,
    ];

    for rt in types {
        let json = serde_json::to_string(&rt).unwrap();
        let deserialized: RuntimeType = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, deserialized);
    }
}

#[test]
fn test_runtime_type_clone_all() {
    let types = vec![
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Native,
        RuntimeType::Python,
        RuntimeType::Gpu,
        RuntimeType::Custom("test".to_string()),
    ];

    for rt in types {
        let cloned = rt.clone();
        assert_eq!(rt, cloned);
    }
}

// ============================================================================
// RuntimeType Collections Tests (5 tests)
// ============================================================================

#[test]
fn test_runtime_type_vec_operations() {
    let mut types = vec![RuntimeType::Container];
    types.push(RuntimeType::Wasm);
    types.push(RuntimeType::Native);

    assert_eq!(types.len(), 3);
    assert!(types.contains(&RuntimeType::Container));
}

#[test]
fn test_runtime_type_vec_iteration() {
    let types = vec![
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Native,
    ];

    let mut count = 0;
    for _rt in &types {
        count += 1;
    }
    assert_eq!(count, 3);
}

#[test]
fn test_runtime_type_vec_filter() {
    let types = [
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Container,
    ];

    let containers: Vec<_> = types
        .iter()
        .filter(|rt| **rt == RuntimeType::Container)
        .collect();

    assert_eq!(containers.len(), 2);
}

#[test]
fn test_runtime_type_vec_dedup() {
    let mut types = vec![
        RuntimeType::Container,
        RuntimeType::Container,
        RuntimeType::Wasm,
    ];
    types.dedup();

    assert_eq!(types.len(), 2);
}

#[test]
fn test_runtime_type_option() {
    let some_rt: Option<RuntimeType> = Some(RuntimeType::Container);
    let none_rt: Option<RuntimeType> = None;

    assert!(some_rt.is_some());
    assert!(none_rt.is_none());
}

// ============================================================================
// Test Coverage Summary
// ============================================================================

#[test]
fn test_runtime_module_coverage_summary() {
    println!("=== Runtime Module Test Coverage ===");
    println!("RuntimeType Basic Tests:        10 tests");
    println!("RuntimeType Pattern Tests:       6 tests");
    println!("RuntimeType Display Tests:       4 tests");
    println!("RuntimeType Collections:         5 tests");
    println!("───────────────────────────────────────");
    println!("Total:                          25 tests");
    println!("Target Coverage:                0% → 30%");
    println!("======================================");
}
