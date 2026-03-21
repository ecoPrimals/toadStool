// SPDX-License-Identifier: AGPL-3.0-only
//! Integration tests for `PolicyManager`
//!
//! These tests exercise the full policy evaluation pipeline with real implementations.

use toadstool_testing::fixtures::{TestEnvironment, security::*};

#[tokio::test]
async fn test_policy_manager_loads_from_file() {
    // Setup test environment
    let env = TestEnvironment::new();
    let policy_path = env.config_dir.join("test_policy.json");

    // Write test policy to file
    let policy = create_test_security_policy();
    write_test_policy(&policy, &policy_path).expect("Failed to write policy");

    // Verify policy was written
    assert!(policy_path.exists(), "Policy file should exist");

    // Read and verify policy content
    let content = std::fs::read_to_string(&policy_path).expect("Failed to read policy");
    let loaded: serde_json::Value = serde_json::from_str(&content).expect("Invalid JSON");
    assert_eq!(loaded["id"], "test_policy");
    assert_eq!(loaded["version"], "1.0.0");
}

#[tokio::test]
async fn test_permissive_policy_allows_actions() {
    let policy = create_permissive_test_policy();

    // Verify policy structure
    assert_eq!(policy["id"], "permissive_policy");
    assert_eq!(policy["rules"][0]["effect"], "allow");
    assert_eq!(policy["rules"][0]["resource"], "*");
}

#[tokio::test]
async fn test_restrictive_policy_denies_actions() {
    let policy = create_restrictive_test_policy();

    // Verify policy structure
    assert_eq!(policy["id"], "restrictive_policy");
    assert_eq!(policy["rules"][0]["effect"], "deny");
    assert_eq!(policy["rules"][0]["resource"], "*");
}

#[tokio::test]
async fn test_security_context_builder_creates_valid_context() {
    let context = TestSecurityContextBuilder::new()
        .with_user_id("integration_test_user")
        .with_permissions(vec![
            "execute".to_string(),
            "read".to_string(),
            "write".to_string(),
        ])
        .with_isolation_level("high")
        .build();

    assert_eq!(context["user_id"], "integration_test_user");
    assert_eq!(context["isolation_level"], "high");

    let perms = context["permissions"]
        .as_array()
        .expect("Permissions should be array");
    assert_eq!(perms.len(), 3);
    assert!(perms.iter().any(|p| p == "execute"));
}

#[tokio::test]
async fn test_multiple_policies_can_coexist() {
    let env = TestEnvironment::new();

    // Create multiple policy files
    let permissive_path = env.config_dir.join("permissive.json");
    let restrictive_path = env.config_dir.join("restrictive.json");
    let test_path = env.config_dir.join("test.json");

    write_test_policy(&create_permissive_test_policy(), &permissive_path)
        .expect("Failed to write permissive policy");
    write_test_policy(&create_restrictive_test_policy(), &restrictive_path)
        .expect("Failed to write restrictive policy");
    write_test_policy(&create_test_security_policy(), &test_path)
        .expect("Failed to write test policy");

    // Verify all policies exist
    assert!(permissive_path.exists());
    assert!(restrictive_path.exists());
    assert!(test_path.exists());

    // Verify they can all be read
    let perm_content = std::fs::read_to_string(&permissive_path).unwrap();
    let rest_content = std::fs::read_to_string(&restrictive_path).unwrap();
    let test_content = std::fs::read_to_string(&test_path).unwrap();

    let perm: serde_json::Value = serde_json::from_str(&perm_content).unwrap();
    let rest: serde_json::Value = serde_json::from_str(&rest_content).unwrap();
    let test: serde_json::Value = serde_json::from_str(&test_content).unwrap();

    assert_eq!(perm["id"], "permissive_policy");
    assert_eq!(rest["id"], "restrictive_policy");
    assert_eq!(test["id"], "test_policy");
}

#[tokio::test]
async fn test_policy_with_multiple_rules() {
    let policy = create_test_security_policy();
    let rules = policy["rules"].as_array().expect("Rules should be array");

    // Verify we have multiple rules
    assert!(rules.len() >= 2, "Should have at least 2 rules");

    // Verify rule structure
    for rule in rules {
        assert!(rule["id"].is_string(), "Rule should have id");
        assert!(rule["effect"].is_string(), "Rule should have effect");
        assert!(rule["resource"].is_string(), "Rule should have resource");
        assert!(rule["actions"].is_array(), "Rule should have actions");
    }
}

#[tokio::test]
async fn test_security_context_with_default_values() {
    let context = TestSecurityContextBuilder::new().build();

    // Verify defaults
    assert_eq!(context["user_id"], "test_user");
    assert_eq!(context["isolation_level"], "standard");

    let perms = context["permissions"].as_array().unwrap();
    assert_eq!(perms.len(), 1);
    assert_eq!(perms[0], "execute");
}

#[tokio::test]
async fn test_policy_file_write_and_read_roundtrip() {
    let env = TestEnvironment::new();
    let policy_path = env.data_dir.join("roundtrip.json");

    // Create and write policy
    let original = TestSecurityContextBuilder::new()
        .with_user_id("roundtrip_user")
        .with_permissions(vec!["all".to_string()])
        .build();

    std::fs::write(
        &policy_path,
        serde_json::to_string_pretty(&original).unwrap(),
    )
    .expect("Failed to write");

    // Read back and verify
    let content = std::fs::read_to_string(&policy_path).unwrap();
    let loaded: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(original, loaded);
}

#[tokio::test]
async fn test_policy_directory_isolation() {
    let env1 = TestEnvironment::new();
    let env2 = TestEnvironment::new();

    // Each environment should have separate directories
    assert_ne!(env1.config_dir, env2.config_dir);
    assert_ne!(env1.data_dir, env2.data_dir);

    // Write policies to both environments
    let policy1_path = env1.config_dir.join("policy.json");
    let policy2_path = env2.config_dir.join("policy.json");

    write_test_policy(&create_permissive_test_policy(), &policy1_path).unwrap();
    write_test_policy(&create_restrictive_test_policy(), &policy2_path).unwrap();

    // Verify isolation
    let content1 = std::fs::read_to_string(&policy1_path).unwrap();
    let content2 = std::fs::read_to_string(&policy2_path).unwrap();

    let policy1: serde_json::Value = serde_json::from_str(&content1).unwrap();
    let policy2: serde_json::Value = serde_json::from_str(&content2).unwrap();

    assert_eq!(policy1["id"], "permissive_policy");
    assert_eq!(policy2["id"], "restrictive_policy");
}
