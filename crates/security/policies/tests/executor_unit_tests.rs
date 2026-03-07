// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::float_cmp)]
//! Unit tests for policy action executor
//! Target: crates/security/policies/src/executor.rs (2.31% coverage)

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use uuid::Uuid;

use toadstool::security::{Capability, IsolationLevel};
use toadstool_security_policies::executor::ActionExecutor;
use toadstool_security_policies::types::{
    PolicyAction, PolicyEvaluationContext, PolicyEvaluationResult, PolicyResult,
};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_empty_result() -> PolicyEvaluationResult {
    PolicyEvaluationResult {
        evaluation_id: Uuid::new_v4(),
        policy_id: "test_policy".to_string(),
        result: PolicyResult::Allow,
        applied_rules: vec![],
        security_modifications: vec![],
        resource_modifications: vec![],
        warnings: vec![],
        evaluation_duration: Duration::from_millis(10),
        timestamp: SystemTime::now(),
    }
}

fn create_test_context() -> PolicyEvaluationContext {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use toadstool::security::SecurityContext;
    use toadstool::workload::{ExecutableSource, WorkloadSpec};
    use toadstool_security_policies::types::SystemInfo;

    PolicyEvaluationContext {
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from("/bin/test"),
            },
            args: Some(vec![]),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        security_context: SecurityContext::default(),
        requested_capabilities: HashSet::new(),
        user_info: None,
        system_info: SystemInfo {
            hostname: "test-host".to_string(),
            os_type: "linux".to_string(),
            os_version: "6.16.3".to_string(),
            architecture: "x86_64".to_string(),
            cpu_count: 8,
            memory_total_mb: 16384,
            load_average: 0.5,
            uptime_seconds: 3600,
        },
        timestamp: SystemTime::now(),
        variables: HashMap::new(),
    }
}

// ============================================================================
// Test: ActionExecutor::new()
// ============================================================================

#[test]
fn test_executor_new() {
    let executor = ActionExecutor::new();
    // Should create successfully (basic smoke test)
    assert!(std::mem::size_of_val(&executor) == 0); // ZST
}

#[test]
fn test_executor_default() {
    let executor = ActionExecutor;
    // Should create successfully using Default trait
    assert!(std::mem::size_of_val(&executor) == 0); // ZST
}

// ============================================================================
// Test: Execute - Allow
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_allow() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::Allow;
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Allow));
    assert_eq!(result.warnings.len(), 0);
}

// ============================================================================
// Test: Execute - Deny
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_deny() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::Deny;
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Deny));
    assert_eq!(result.warnings.len(), 0);
}

// ============================================================================
// Test: Execute - AllowWithWarning
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_allow_with_warning() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::AllowWithWarning {
        message: "This operation is discouraged".to_string(),
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Allow));
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].level, "warning");
    assert_eq!(result.warnings[0].message, "This operation is discouraged");
}

// ============================================================================
// Test: Execute - DenyWithMessage
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_deny_with_message() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::DenyWithMessage {
        message: "Access denied due to policy violation".to_string(),
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Deny));
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].level, "error");
    assert_eq!(
        result.warnings[0].message,
        "Access denied due to policy violation"
    );
}

// ============================================================================
// Test: Execute - ModifySecurityContext - IsolationLevel
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_modify_security_context_isolation() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::ModifySecurityContext {
        isolation_level: Some(IsolationLevel::Maximum),
        add_capabilities: vec![],
        remove_capabilities: vec![],
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Modified));
    assert_eq!(result.security_modifications.len(), 1);
    assert_eq!(
        result.security_modifications[0].modification_type,
        "isolation_level"
    );
    assert_eq!(
        result.security_modifications[0].reason,
        "Policy enforcement"
    );
}

// ============================================================================
// Test: Execute - ModifySecurityContext - Add Capabilities
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_modify_security_context_add_capabilities() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::ModifySecurityContext {
        isolation_level: None,
        add_capabilities: vec![Capability::NetworkClient, Capability::Read],
        remove_capabilities: vec![],
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Modified));
    assert_eq!(result.security_modifications.len(), 2);
    assert_eq!(
        result.security_modifications[0].modification_type,
        "add_capability"
    );
    assert_eq!(
        result.security_modifications[0].reason,
        "Policy requirement"
    );
    assert_eq!(
        result.security_modifications[1].modification_type,
        "add_capability"
    );
}

// ============================================================================
// Test: Execute - ModifySecurityContext - Remove Capabilities
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_modify_security_context_remove_capabilities() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::ModifySecurityContext {
        isolation_level: None,
        add_capabilities: vec![],
        remove_capabilities: vec![Capability::Write, Capability::ProcessManagement],
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Modified));
    assert_eq!(result.security_modifications.len(), 2);
    assert_eq!(
        result.security_modifications[0].modification_type,
        "remove_capability"
    );
    assert_eq!(
        result.security_modifications[0].reason,
        "Policy restriction"
    );
    assert_eq!(
        result.security_modifications[1].modification_type,
        "remove_capability"
    );
}

// ============================================================================
// Test: Execute - ModifySecurityContext - Combined
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_modify_security_context_combined() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::ModifySecurityContext {
        isolation_level: Some(IsolationLevel::Maximum),
        add_capabilities: vec![Capability::Read],
        remove_capabilities: vec![Capability::Write],
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Modified));
    // 1 for isolation + 1 for add + 1 for remove
    assert_eq!(result.security_modifications.len(), 3);
}

// ============================================================================
// Test: Execute - ApplyResourceLimits - CPU
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_apply_resource_limits_cpu() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::ApplyResourceLimits {
        cpu_percent: Some(50.0),
        memory_mb: None,
        network_mbps: None,
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Modified));
    assert_eq!(result.resource_modifications.len(), 1);
    assert_eq!(
        result.resource_modifications[0].resource_type,
        "cpu_percent"
    );
    assert_eq!(result.resource_modifications[0].new_limit, 50.0);
    assert_eq!(
        result.resource_modifications[0].reason,
        "Policy enforcement"
    );
}

// ============================================================================
// Test: Execute - ApplyResourceLimits - Memory
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_apply_resource_limits_memory() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::ApplyResourceLimits {
        cpu_percent: None,
        memory_mb: Some(2048),
        network_mbps: None,
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Modified));
    assert_eq!(result.resource_modifications.len(), 1);
    assert_eq!(result.resource_modifications[0].resource_type, "memory_mb");
    assert_eq!(result.resource_modifications[0].new_limit, 2048.0);
}

// ============================================================================
// Test: Execute - ApplyResourceLimits - Network
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_apply_resource_limits_network() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::ApplyResourceLimits {
        cpu_percent: None,
        memory_mb: None,
        network_mbps: Some(100.0),
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Modified));
    assert_eq!(result.resource_modifications.len(), 1);
    assert_eq!(
        result.resource_modifications[0].resource_type,
        "network_mbps"
    );
    assert_eq!(result.resource_modifications[0].new_limit, 100.0);
}

// ============================================================================
// Test: Execute - ApplyResourceLimits - All
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_apply_resource_limits_all() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::ApplyResourceLimits {
        cpu_percent: Some(75.5),
        memory_mb: Some(4096),
        network_mbps: Some(50.25),
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Modified));
    assert_eq!(result.resource_modifications.len(), 3);

    // Check CPU
    assert_eq!(
        result.resource_modifications[0].resource_type,
        "cpu_percent"
    );
    assert_eq!(result.resource_modifications[0].new_limit, 75.5);

    // Check Memory
    assert_eq!(result.resource_modifications[1].resource_type, "memory_mb");
    assert_eq!(result.resource_modifications[1].new_limit, 4096.0);

    // Check Network
    assert_eq!(
        result.resource_modifications[2].resource_type,
        "network_mbps"
    );
    assert_eq!(result.resource_modifications[2].new_limit, 50.25);
}

// ============================================================================
// Test: Execute - RequireAuthentication
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_require_authentication() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::RequireAuthentication {
        method: "2FA".to_string(),
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::RequiresAuth));
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].level, "info");
    assert!(result.warnings[0]
        .message
        .contains("Additional authentication required"));
    assert!(result.warnings[0].message.contains("2FA"));
}

// ============================================================================
// Test: Execute - LogAndContinue
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_log_and_continue() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::LogAndContinue {
        level: "debug".to_string(),
        message: "Operation logged for audit".to_string(),
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    // Result should remain unchanged from initial Allow
    assert!(matches!(result.result, PolicyResult::Allow));
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].level, "debug");
    assert_eq!(result.warnings[0].message, "Operation logged for audit");
}

// ============================================================================
// Test: Execute - Custom Action
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_custom_action() {
    let executor = ActionExecutor::new();
    let mut params = HashMap::new();
    params.insert(
        "custom_param".to_string(),
        serde_json::json!("custom_value"),
    );

    let action = PolicyAction::Custom {
        action: "custom_security_check".to_string(),
        parameters: params,
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].level, "info");
    assert!(result.warnings[0]
        .message
        .contains("Custom action executed: custom_security_check"));
}

// ============================================================================
// Test: Execute - Multiple Actions in Sequence
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_multiple_actions_sequence() {
    let executor = ActionExecutor::new();
    let context = create_test_context();
    let mut result = create_empty_result();

    // Action 1: Log
    let action1 = PolicyAction::LogAndContinue {
        level: "info".to_string(),
        message: "Starting operation".to_string(),
    };
    executor
        .execute_action(&action1, &mut result, &context)
        .unwrap();
    assert_eq!(result.warnings.len(), 1);

    // Action 2: Modify security
    let action2 = PolicyAction::ModifySecurityContext {
        isolation_level: Some(IsolationLevel::Maximum),
        add_capabilities: vec![Capability::Read],
        remove_capabilities: vec![],
    };
    executor
        .execute_action(&action2, &mut result, &context)
        .unwrap();
    assert_eq!(result.security_modifications.len(), 2);
    assert!(matches!(result.result, PolicyResult::Modified));

    // Action 3: Apply limits
    let action3 = PolicyAction::ApplyResourceLimits {
        cpu_percent: Some(50.0),
        memory_mb: Some(1024),
        network_mbps: None,
    };
    executor
        .execute_action(&action3, &mut result, &context)
        .unwrap();
    assert_eq!(result.resource_modifications.len(), 2);

    // Check final state
    assert!(matches!(result.result, PolicyResult::Modified));
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.security_modifications.len(), 2);
    assert_eq!(result.resource_modifications.len(), 2);
}

// ============================================================================
// Test: Execute - Edge Cases
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_empty_capabilities_lists() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::ModifySecurityContext {
        isolation_level: None,
        add_capabilities: vec![],
        remove_capabilities: vec![],
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Modified));
    assert_eq!(result.security_modifications.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_no_resource_limits() {
    let executor = ActionExecutor::new();
    let action = PolicyAction::ApplyResourceLimits {
        cpu_percent: None,
        memory_mb: None,
        network_mbps: None,
    };
    let mut result = create_empty_result();
    let context = create_test_context();

    let exec_result = executor.execute_action(&action, &mut result, &context);
    assert!(exec_result.is_ok());
    assert!(matches!(result.result, PolicyResult::Modified));
    assert_eq!(result.resource_modifications.len(), 0);
}
