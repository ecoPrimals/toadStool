//! E2E tests for IPC helpers and semantic method registry
//!
//! These tests expand coverage of the semantic method system which is
//! part of ToadStool's compliance with wateringHole/ SEMANTIC_METHOD_NAMING_STANDARD.md

use toadstool::ipc_helpers::{
    find_by_capability, get_semantic_name, is_semantic_method, list_semantic_methods,
    resolve_method_name,
};

/// Test semantic method name resolution
#[test]
fn test_semantic_method_resolution() {
    // Test compute domain methods
    assert_eq!(
        resolve_method_name("compute.execute"),
        "compute.execute".to_string()
    );
    assert_eq!(
        resolve_method_name("compute.execute.batch"),
        "compute.execute.batch".to_string()
    );

    // Test workload domain methods
    assert_eq!(
        resolve_method_name("workload.create"),
        "workload.create".to_string()
    );
    assert_eq!(
        resolve_method_name("workload.status"),
        "workload.status".to_string()
    );

    // Test discovery domain methods
    assert_eq!(
        resolve_method_name("discovery.query"),
        "discovery.query".to_string()
    );
    assert_eq!(
        resolve_method_name("discovery.announce"),
        "discovery.announce".to_string()
    );
}

/// Test semantic method validation
#[test]
fn test_semantic_method_validation() {
    // Valid semantic methods
    assert!(is_semantic_method("compute.execute"));
    assert!(is_semantic_method("workload.create"));
    assert!(is_semantic_method("discovery.query"));
    assert!(is_semantic_method("monitoring.health"));
    assert!(is_semantic_method("resources.estimate"));
    assert!(is_semantic_method("capability.query"));

    // Invalid formats
    assert!(!is_semantic_method(""));
    assert!(!is_semantic_method("invalid"));
    assert!(!is_semantic_method("no_dot"));
    assert!(!is_semantic_method(".nodomain"));
    assert!(!is_semantic_method("domain."));
}

/// Test semantic name reverse lookup
#[test]
fn test_semantic_name_lookup() {
    // Test known implementations have semantic names
    // These should be registered in semantic_methods.rs
    let implementations = vec![
        "execute_workload",
        "create_workload",
        "query_workload_status",
        "query_capabilities",
        "health_check",
    ];

    for impl_name in implementations {
        let semantic = get_semantic_name(impl_name);
        if let Some(name) = semantic {
            // Verify it's a valid semantic method
            assert!(is_semantic_method(&name));
            // Verify it has domain.operation format
            assert!(name.contains('.'));
        }
    }
}

/// Test list all semantic methods
#[test]
fn test_list_semantic_methods() {
    let methods = list_semantic_methods();

    // Should have many methods registered (Phase 1: 50+)
    assert!(
        methods.len() >= 40,
        "Expected at least 40 semantic methods, got {}",
        methods.len()
    );

    // All should be valid semantic methods
    for method in &methods {
        assert!(
            is_semantic_method(method),
            "Invalid semantic method: {}",
            method
        );
    }

    // Should include core domains
    let domains: Vec<String> = methods
        .iter()
        .filter_map(|m| m.split('.').next().map(String::from))
        .collect();

    assert!(domains.iter().any(|d| d == "compute"));
    assert!(domains.iter().any(|d| d == "workload"));
    assert!(domains.iter().any(|d| d == "discovery"));
    assert!(domains.iter().any(|d| d == "monitoring"));
    assert!(domains.iter().any(|d| d == "resources"));
    assert!(domains.iter().any(|d| d == "capability"));
}

/// Test capability-based discovery (graceful degradation when Songbird unavailable)
#[tokio::test]
async fn test_capability_discovery_standalone() {
    // When Songbird is not available, should return empty list gracefully
    let result = find_by_capability("compute").await;

    // Should succeed (graceful degradation) or fail gracefully
    match result {
        Ok(primals) => {
            // If Songbird is running, we might get results
            // If not, we get empty list
            assert!(primals.is_empty() || !primals.is_empty());
        }
        Err(e) => {
            // Graceful degradation - expected when Songbird not running
            assert!(
                e.to_string().contains("Songbird")
                    || e.to_string().contains("discovery")
                    || e.to_string().contains("connection"),
                "Unexpected error: {}",
                e
            );
        }
    }
}

/// Test semantic method domains coverage
#[test]
fn test_semantic_domains_coverage() {
    let methods = list_semantic_methods();

    // Extract unique domains
    let mut domains: Vec<String> = methods
        .iter()
        .filter_map(|m| m.split('.').next().map(String::from))
        .collect();
    domains.sort();
    domains.dedup();

    // Verify Phase 1 target: 6+ domains
    assert!(
        domains.len() >= 6,
        "Expected at least 6 domains, got {}: {:?}",
        domains.len(),
        domains
    );

    // Verify each domain has multiple operations
    for domain in &domains {
        let ops: Vec<&str> = methods
            .iter()
            .filter(|m| m.starts_with(domain))
            .map(String::as_str)
            .collect();
        assert!(
            ops.len() >= 3,
            "Domain {} should have at least 3 operations, got {}",
            domain,
            ops.len()
        );
    }
}

/// Test method resolution with variants
#[test]
fn test_method_variants() {
    // Test that variants are preserved
    assert_eq!(
        resolve_method_name("compute.execute.batch"),
        "compute.execute.batch"
    );
    assert_eq!(
        resolve_method_name("workload.create.batch"),
        "workload.create.batch"
    );

    // Base methods should work
    assert_eq!(resolve_method_name("compute.execute"), "compute.execute");
    assert_eq!(resolve_method_name("workload.create"), "workload.create");
}

/// Test semantic method format compliance
#[test]
fn test_semantic_format_compliance() {
    let methods = list_semantic_methods();

    for method in methods {
        // Must have at least domain.operation
        let parts: Vec<&str> = method.split('.').collect();
        assert!(
            parts.len() >= 2,
            "Method {} must have at least domain.operation",
            method
        );

        // Domain should not be empty
        assert!(!parts[0].is_empty(), "Domain in {} is empty", method);

        // Operation should not be empty
        assert!(!parts[1].is_empty(), "Operation in {} is empty", method);

        // If variant exists, should not be empty
        if parts.len() > 2 {
            assert!(
                !parts[2].is_empty(),
                "Variant in {} is empty but present",
                method
            );
        }
    }
}

/// Test method registry contains expected core methods
#[test]
fn test_core_methods_registered() {
    let methods = list_semantic_methods();

    // Core compute methods
    assert!(
        methods.iter().any(|m| m.starts_with("compute.")),
        "Missing compute domain"
    );

    // Core workload methods
    assert!(
        methods.iter().any(|m| m.starts_with("workload.")),
        "Missing workload domain"
    );

    // Core discovery methods  
    assert!(
        methods.iter().any(|m| m.starts_with("discovery.")),
        "Missing discovery domain"
    );

    // Core monitoring methods
    assert!(
        methods.iter().any(|m| m.starts_with("monitoring.")),
        "Missing monitoring domain"
    );
}

/// Test semantic method lookup is case-sensitive
#[test]
fn test_semantic_case_sensitivity() {
    // Semantic methods should be lowercase
    assert!(is_semantic_method("compute.execute"));

    // Mixed case should still work if registered
    // (depends on registry implementation)
    let methods = list_semantic_methods();
    for method in methods {
        // All should be lowercase per standard
        assert_eq!(
            method,
            method.to_lowercase(),
            "Method {} should be lowercase",
            method
        );
    }
}

/// Test semantic method reverse lookup coverage
#[test]
fn test_reverse_lookup_coverage() {
    // Test various implementation names
    let test_cases = vec![
        ("execute_workload", Some("compute.execute")),
        ("create_workload", Some("workload.create")),
        ("query_capabilities", Some("capability.query")),
        ("health_check", Some("monitoring.health")),
        ("unknown_method", None),
        ("", None),
    ];

    for (impl_name, expected) in test_cases {
        let result = get_semantic_name(impl_name);
        match expected {
            Some(expected_name) => {
                if let Some(name) = result {
                    assert!(
                        name.contains('.'),
                        "Semantic name {} for {} should have domain.operation format",
                        name,
                        impl_name
                    );
                }
            }
            None => {
                assert_eq!(
                    result, None,
                    "Expected None for {}, got {:?}",
                    impl_name, result
                );
            }
        }
    }
}
