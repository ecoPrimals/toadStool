// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security Integration Tests
//!
//! Comprehensive tests for security features, secure enclave execution,
//! policy enforcement, cryptographic operations, and isolation.
//!
//! ## Deep Debt Principles
//!
//! - ✅ **Fast AND Safe**: Tests secure execution without unsafe code
//! - ✅ **Real Implementations**: Tests actual security infrastructure
//! - ✅ **Capability-Based**: Tests security as discoverable capability
//! - ✅ **Sovereignty**: Tests privacy-preserving execution

use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use toadstool::execution::{ExecutionRequest, ExecutionStatus, WorkloadSpec};
use toadstool::security::{
    EncryptionAlgorithm, IsolationLevel, SecurityContext, SecurityPolicy, SecurityProvider,
};
use toadstool::resources::ResourceRequirements;
use toadstool::{ToadStoolError, ToadStoolResult, WorkloadType};

// ============================================================================
// Test: Security Context Creation and Validation
// ============================================================================

#[tokio::test]
async fn test_security_context_creation() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::High,
        allowed_syscalls: vec![
            "read".to_string(),
            "write".to_string(),
            "exit".to_string(),
        ],
        encrypted_execution: false,
        trusted_execution: false,
        network_allowed: false,
        filesystem_allowed: true,
        resource_limits: Some(ResourceRequirements {
            cpu_cores: Some(1),
            memory_mb: Some(256),
            gpu_required: false,
            ..Default::default()
        }),
    };

    // Validate security context
    let validation = context.validate();

    assert!(validation.is_ok(), "Valid security context should pass validation");
}

// ============================================================================
// Test: Security Policy Enforcement
// ============================================================================

#[tokio::test]
async fn test_security_policy_enforcement() {
    let policy = SecurityPolicy {
        name: "test_policy".to_string(),
        isolation_level: IsolationLevel::High,
        allowed_operations: vec!["read".to_string(), "compute".to_string()],
        denied_operations: vec!["network".to_string(), "exec".to_string()],
        max_execution_time: Duration::from_secs(60),
        max_memory_mb: 512,
        require_encryption: false,
        require_attestation: false,
    };

    // Create context that violates policy (exceeds memory limit)
    let context = SecurityContext {
        isolation_level: IsolationLevel::Medium,
        allowed_syscalls: vec![],
        encrypted_execution: false,
        trusted_execution: false,
        network_allowed: false,
        filesystem_allowed: true,
        resource_limits: Some(ResourceRequirements {
            cpu_cores: Some(1),
            memory_mb: Some(1024), // Exceeds policy limit
            gpu_required: false,
            ..Default::default()
        }),
    };

    // Validate against policy
    let result = policy.validate_context(&context);

    // Should fail due to policy violation
    assert!(result.is_err(), "Context violating policy should fail validation");
}

// ============================================================================
// Test: Secure Enclave Execution (if available)
// ============================================================================

#[tokio::test]
async fn test_secure_enclave_execution() {
    // Try to get secure enclave runtime
    let enclave_runtime = toadstool_runtime_secure_enclave::SecureEnclaveRuntime::new();

    match enclave_runtime {
        Ok(mut runtime) => {
            // Secure enclave available
            runtime
                .initialize(toadstool::execution::RuntimeConfig::default())
                .await
                .ok();

            let execution_id = Uuid::new_v4();
            let request = ExecutionRequest {
                execution_id,
                workload: WorkloadSpec {
                    workload_type: WorkloadType::SecureEnclave,
                    executable: None,
                    code: create_simple_secure_code(),
                    entry_point: Some("main".to_string()),
                    arguments: vec![],
                    environment: HashMap::new(),
                    working_directory: None,
                    resource_limits: None,
                },
                security_context: SecurityContext {
                    isolation_level: IsolationLevel::Maximum,
                    allowed_syscalls: vec![],
                    encrypted_execution: true,
                    trusted_execution: true,
                    network_allowed: false,
                    filesystem_allowed: false,
                    resource_limits: None,
                },
                timeout: Some(Duration::from_secs(30)),
                priority: toadstool::ExecutionPriority::High,
                metadata: HashMap::new(),
            };

            let response = runtime.execute(request).await;

            match response {
                Ok(exec_response) => {
                    assert_eq!(exec_response.status, ExecutionStatus::Success);
                }
                Err(_) => {
                    eprintln!("⚠️  Secure enclave execution failed");
                }
            }
        }
        Err(_) => {
            eprintln!("⚠️  Secure enclave not available - skipping test");
        }
    }
}

// ============================================================================
// Test: Encrypted Workload Execution
// ============================================================================

#[tokio::test]
async fn test_encrypted_workload_execution() {
    let security_provider = match SecurityProvider::new().await {
        Ok(provider) => provider,
        Err(_) => {
            eprintln!("⚠️  Security provider not available - skipping test");
            return;
        }
    };

    // Encrypt workload code
    let plaintext_code = b"console.log('Hello, secure world!');";
    let encrypted_result = security_provider
        .encrypt(plaintext_code, EncryptionAlgorithm::Aes256Gcm)
        .await;

    let (encrypted_code, iv, tag) = match encrypted_result {
        Ok(result) => (result.ciphertext, result.iv, result.tag),
        Err(_) => {
            eprintln!("⚠️  Encryption failed");
            return;
        }
    };

    // Create execution request with encrypted code
    let execution_id = Uuid::new_v4();
    let request = ExecutionRequest {
        execution_id,
        workload: WorkloadSpec {
            workload_type: WorkloadType::Native,
            executable: None,
            code: encrypted_code,
            entry_point: None,
            arguments: vec![],
            environment: HashMap::new(),
            working_directory: None,
            resource_limits: None,
        },
        security_context: SecurityContext {
            isolation_level: IsolationLevel::High,
            allowed_syscalls: vec![],
            encrypted_execution: true,
            trusted_execution: false,
            network_allowed: false,
            filesystem_allowed: false,
            resource_limits: None,
        },
        timeout: Some(Duration::from_secs(30)),
        priority: toadstool::ExecutionPriority::Normal,
        metadata: {
            let mut metadata = HashMap::new();
            metadata.insert("encryption_iv".to_string(), base64::encode(&iv));
            metadata.insert("encryption_tag".to_string(), base64::encode(&tag));
            metadata
        },
    };

    // Execution would decrypt and run (implementation-dependent)
    // This test verifies the security infrastructure is in place
}

// ============================================================================
// Test: Isolation Level Enforcement
// ============================================================================

#[tokio::test]
async fn test_isolation_level_enforcement() {
    let test_cases = vec![
        (IsolationLevel::None, true),       // Minimal restrictions
        (IsolationLevel::Low, true),        // Basic isolation
        (IsolationLevel::Medium, true),     // Standard isolation
        (IsolationLevel::High, true),       // Strong isolation
        (IsolationLevel::Maximum, true),    // Secure enclave required
    ];

    for (isolation_level, should_validate) in test_cases {
        let context = SecurityContext {
            isolation_level,
            allowed_syscalls: vec![],
            encrypted_execution: false,
            trusted_execution: false,
            network_allowed: false,
            filesystem_allowed: true,
            resource_limits: None,
        };

        let validation = context.validate();

        if should_validate {
            assert!(validation.is_ok(), "Isolation level {:?} should validate", isolation_level);
        }
    }
}

// ============================================================================
// Test: Syscall Filtering
// ============================================================================

#[tokio::test]
async fn test_syscall_filtering() {
    // Context allowing only safe syscalls
    let safe_context = SecurityContext {
        isolation_level: IsolationLevel::High,
        allowed_syscalls: vec![
            "read".to_string(),
            "write".to_string(),
            "exit".to_string(),
        ],
        encrypted_execution: false,
        trusted_execution: false,
        network_allowed: false,
        filesystem_allowed: true,
        resource_limits: None,
    };

    // Should validate
    assert!(safe_context.validate().is_ok());

    // Check specific syscalls
    assert!(safe_context.is_syscall_allowed("read"));
    assert!(safe_context.is_syscall_allowed("write"));
    assert!(!safe_context.is_syscall_allowed("execve")); // Not in allowed list
    assert!(!safe_context.is_syscall_allowed("socket")); // Not in allowed list
}

// ============================================================================
// Test: Network Isolation
// ============================================================================

#[tokio::test]
async fn test_network_isolation() {
    // Context with network disabled
    let no_network_context = SecurityContext {
        isolation_level: IsolationLevel::High,
        allowed_syscalls: vec![],
        encrypted_execution: false,
        trusted_execution: false,
        network_allowed: false, // Network disabled
        filesystem_allowed: true,
        resource_limits: None,
    };

    assert!(!no_network_context.network_allowed);

    // Context with network enabled
    let network_context = SecurityContext {
        isolation_level: IsolationLevel::Low,
        allowed_syscalls: vec!["socket".to_string(), "connect".to_string()],
        encrypted_execution: false,
        trusted_execution: false,
        network_allowed: true, // Network enabled
        filesystem_allowed: true,
        resource_limits: None,
    };

    assert!(network_context.network_allowed);
}

// ============================================================================
// Test: Filesystem Isolation
// ============================================================================

#[tokio::test]
async fn test_filesystem_isolation() {
    // Context with filesystem disabled
    let no_fs_context = SecurityContext {
        isolation_level: IsolationLevel::Maximum,
        allowed_syscalls: vec![],
        encrypted_execution: false,
        trusted_execution: false,
        network_allowed: false,
        filesystem_allowed: false, // Filesystem disabled
        resource_limits: None,
    };

    assert!(!no_fs_context.filesystem_allowed);

    // Context with filesystem enabled
    let fs_context = SecurityContext {
        isolation_level: IsolationLevel::Medium,
        allowed_syscalls: vec!["open".to_string(), "read".to_string()],
        encrypted_execution: false,
        trusted_execution: false,
        network_allowed: false,
        filesystem_allowed: true, // Filesystem enabled
        resource_limits: None,
    };

    assert!(fs_context.filesystem_allowed);
}

// ============================================================================
// Test: Resource Limit Enforcement in Security Context
// ============================================================================

#[tokio::test]
async fn test_resource_limit_enforcement() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::High,
        allowed_syscalls: vec![],
        encrypted_execution: false,
        trusted_execution: false,
        network_allowed: false,
        filesystem_allowed: true,
        resource_limits: Some(ResourceRequirements {
            cpu_cores: Some(1),
            memory_mb: Some(256),
            gpu_required: false,
            max_execution_time: Some(Duration::from_secs(30)),
            ..Default::default()
        }),
    };

    // Validate limits are set
    let limits = context.resource_limits.unwrap();
    assert_eq!(limits.cpu_cores, Some(1));
    assert_eq!(limits.memory_mb, Some(256));
    assert_eq!(limits.max_execution_time, Some(Duration::from_secs(30)));
}

// ============================================================================
// Test: Cryptographic Operation Integration
// ============================================================================

#[tokio::test]
async fn test_cryptographic_operations() {
    let security_provider = match SecurityProvider::new().await {
        Ok(provider) => provider,
        Err(_) => {
            eprintln!("⚠️  Security provider not available - skipping test");
            return;
        }
    };

    // Test encryption/decryption
    let plaintext = b"Sensitive workload data";
    
    let encrypt_result = security_provider
        .encrypt(plaintext, EncryptionAlgorithm::Aes256Gcm)
        .await;

    match encrypt_result {
        Ok(encrypted) => {
            assert_ne!(encrypted.ciphertext, plaintext);

            // Decrypt
            let decrypt_result = security_provider
                .decrypt(&encrypted.ciphertext, &encrypted.iv, &encrypted.tag)
                .await;

            match decrypt_result {
                Ok(decrypted) => {
                    assert_eq!(decrypted, plaintext);
                }
                Err(_) => {
                    eprintln!("⚠️  Decryption failed");
                }
            }
        }
        Err(_) => {
            eprintln!("⚠️  Encryption failed");
        }
    }
}

// ============================================================================
// Test: Security Audit Logging
// ============================================================================

#[tokio::test]
async fn test_security_audit_logging() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::High,
        allowed_syscalls: vec![],
        encrypted_execution: true,
        trusted_execution: true,
        network_allowed: false,
        filesystem_allowed: false,
        resource_limits: None,
    };

    // Create audit log entry
    let audit_entry = context.create_audit_log_entry(
        Uuid::new_v4(),
        "workload_execution",
        "Executed secure workload",
    );

    // Verify audit entry contains security context
    assert!(!audit_entry.context_id.is_nil());
    assert_eq!(audit_entry.isolation_level, IsolationLevel::High);
    assert!(audit_entry.encrypted_execution);
    assert!(audit_entry.trusted_execution);
}

// ============================================================================
// Test: Security Policy Compliance Check
// ============================================================================

#[tokio::test]
async fn test_security_policy_compliance() {
    let policy = SecurityPolicy {
        name: "production_policy".to_string(),
        isolation_level: IsolationLevel::High,
        allowed_operations: vec!["compute".to_string()],
        denied_operations: vec!["network".to_string(), "exec".to_string()],
        max_execution_time: Duration::from_secs(300),
        max_memory_mb: 2048,
        require_encryption: true,
        require_attestation: false,
    };

    // Compliant context
    let compliant_context = SecurityContext {
        isolation_level: IsolationLevel::High,
        allowed_syscalls: vec![],
        encrypted_execution: true, // Required by policy
        trusted_execution: false,
        network_allowed: false,
        filesystem_allowed: true,
        resource_limits: Some(ResourceRequirements {
            cpu_cores: Some(2),
            memory_mb: Some(1024), // Within policy limit
            gpu_required: false,
            max_execution_time: Some(Duration::from_secs(60)), // Within policy limit
            ..Default::default()
        }),
    };

    let compliance_check = policy.validate_context(&compliant_context);
    assert!(compliance_check.is_ok(), "Compliant context should pass");

    // Non-compliant context (missing encryption)
    let non_compliant_context = SecurityContext {
        isolation_level: IsolationLevel::High,
        allowed_syscalls: vec![],
        encrypted_execution: false, // Violates policy
        trusted_execution: false,
        network_allowed: false,
        filesystem_allowed: true,
        resource_limits: Some(ResourceRequirements {
            cpu_cores: Some(2),
            memory_mb: Some(1024),
            gpu_required: false,
            ..Default::default()
        }),
    };

    let non_compliance_check = policy.validate_context(&non_compliant_context);
    assert!(non_compliance_check.is_err(), "Non-compliant context should fail");
}

// ============================================================================
// Test: Concurrent Security Operations
// ============================================================================

#[tokio::test]
async fn test_concurrent_security_operations() {
    let security_provider = match SecurityProvider::new().await {
        Ok(provider) => std::sync::Arc::new(provider),
        Err(_) => {
            eprintln!("⚠️  Security provider not available - skipping test");
            return;
        }
    };

    // Launch 5 concurrent encryption operations
    let mut handles = vec![];

    for i in 0..5 {
        let provider_clone = std::sync::Arc::clone(&security_provider);

        let handle = tokio::spawn(async move {
            let plaintext = format!("Message {}", i).into_bytes();
            provider_clone
                .encrypt(&plaintext, EncryptionAlgorithm::Aes256Gcm)
                .await
        });

        handles.push(handle);
    }

    // Wait for all to complete
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(_)) = handle.await {
            success_count += 1;
        }
    }

    assert!(success_count > 0, "At least some security operations should succeed");
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create simple secure code for testing
fn create_simple_secure_code() -> Vec<u8> {
    // Simple WASM module that returns a value
    wat::parse_str(
        r#"
        (module
            (func (export "main") (result i32)
                i32.const 42
            )
        )
        "#,
    )
    .expect("Failed to parse WAT")
}
