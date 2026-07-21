// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use std::time::Duration;

#[test]
fn test_client_config_default() {
    let config = ClientConfig::default();
    assert_eq!(
        config.base_url,
        format!(
            "http://{}:{}",
            toadstool_config::defaults::network::LOCALHOST,
            toadstool_config::defaults::network::API_PORT
        )
    );
    assert_eq!(
        config.request_timeout,
        Duration::from_millis(toadstool_config::defaults::timeouts::REQUEST_MS)
    );
    assert_eq!(
        config.max_retries,
        toadstool_config::defaults::retries::MAX_ATTEMPTS
    );
}

#[test]
fn test_native_workload_builder() {
    let workload = WorkloadSubmission::native()
        .executable("/bin/echo")
        .args(vec!["Hello".to_string(), "World".to_string()])
        .priority(JobPriority::High)
        .build()
        .unwrap();

    match workload.workload_type {
        WorkloadType::Native {
            executable, args, ..
        } => {
            assert_eq!(executable, "/bin/echo");
            assert_eq!(args, vec!["Hello", "World"]);
        }
        _ => unreachable!(
            "expected native workload type, got: {:?}",
            workload.workload_type
        ),
    }

    assert_eq!(workload.priority, Some(JobPriority::High));
    assert_eq!(workload.runtime_hint, Some("native".to_string()));
}

#[test]
fn test_container_workload_builder() {
    let workload = WorkloadSubmission::container()
        .image("alpine:latest")
        .command(vec!["echo".to_string()])
        .args(vec!["Hello from container".to_string()])
        .build()
        .expect("Failed to build workload");

    assert!(matches!(
        workload.workload_type,
        WorkloadType::Container { .. }
    ));
    if let WorkloadType::Container {
        image,
        command,
        args,
        ..
    } = &workload.workload_type
    {
        assert_eq!(image, "alpine:latest");
        assert_eq!(command, &Some(vec!["echo".to_string()]));
        assert_eq!(args, &Some(vec!["Hello from container".to_string()]));
    }

    assert_eq!(workload.runtime_hint, Some("container".to_string()));
}

#[test]
fn test_python_workload_builder() {
    let workload = WorkloadSubmission::python()
        .script("print('Hello, Python!')")
        .requirements(vec!["requests==2.28.0".to_string()])
        .build()
        .expect("Failed to build workload");

    assert!(matches!(
        workload.workload_type,
        WorkloadType::Python { .. }
    ));
    if let WorkloadType::Python {
        script,
        requirements,
    } = &workload.workload_type
    {
        assert_eq!(script, "print('Hello, Python!')");
        assert_eq!(requirements, &vec!["requests==2.28.0".to_string()]);
    }

    assert_eq!(workload.runtime_hint, Some("python".to_string()));
}

#[test]
fn test_execution_status_serialization() {
    let status = ExecutionStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, r#""Running""#);

    let deserialized: ExecutionStatus = serde_json::from_str(&json).unwrap();
    matches!(deserialized, ExecutionStatus::Running);
}

#[test]
fn test_wasm_workload_builder() {
    let module_data = vec![0, 97, 115, 109]; // WASM magic number
    let workload = WorkloadSubmission::wasm()
        .module_data(module_data.clone())
        .args(vec!["arg1".to_string(), "arg2".to_string()])
        .priority(JobPriority::Normal)
        .build()
        .expect("Failed to build workload");

    assert!(matches!(workload.workload_type, WorkloadType::Wasm { .. }));
    if let WorkloadType::Wasm {
        module_data: data,
        args,
    } = &workload.workload_type
    {
        assert_eq!(data, &module_data);
        assert_eq!(args, &vec!["arg1".to_string(), "arg2".to_string()]);
    }

    assert_eq!(workload.priority, Some(JobPriority::Normal));
    assert_eq!(workload.runtime_hint, Some("wasm".to_string()));
}

#[test]
fn test_job_priority_ordering() {
    // Lower number = higher priority in the new ordering
    assert!(JobPriority::Emergency < JobPriority::Critical);
    assert!(JobPriority::Critical < JobPriority::High);
    assert!(JobPriority::High < JobPriority::Normal);
    assert!(JobPriority::Normal < JobPriority::Low);
    assert!(JobPriority::Low < JobPriority::Background);
}

#[test]
fn test_workload_type_equality() {
    let workload1 = WorkloadSubmission::native()
        .executable("/bin/echo")
        .build()
        .unwrap();

    let workload2 = WorkloadSubmission::native()
        .executable("/bin/echo")
        .build()
        .unwrap();

    assert!(matches!(
        &workload1.workload_type,
        WorkloadType::Native { .. }
    ));
    assert!(matches!(
        &workload2.workload_type,
        WorkloadType::Native { .. }
    ));
    if let (
        WorkloadType::Native {
            executable: exec1, ..
        },
        WorkloadType::Native {
            executable: exec2, ..
        },
    ) = (&workload1.workload_type, &workload2.workload_type)
    {
        assert_eq!(exec1, exec2);
    }
}

#[test]
fn test_resource_requirements_default() {
    let requirements = ResourceRequirements::default();

    assert!(requirements.cpu_cores.is_none());
    assert!(requirements.memory_mb.is_none());
    assert!(requirements.disk_mb.is_none());
    assert!(requirements.gpu_required.is_none());
}

#[test]
fn test_execution_status_variants() {
    let statuses = vec![
        ExecutionStatus::Pending,
        ExecutionStatus::Queued,
        ExecutionStatus::Running,
        ExecutionStatus::Completed,
        ExecutionStatus::Failed,
        ExecutionStatus::Cancelled,
        ExecutionStatus::Timeout,
    ];

    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ExecutionStatus = serde_json::from_str(&json).unwrap();
        // Compare discriminants
        assert_eq!(
            std::mem::discriminant(&status),
            std::mem::discriminant(&deserialized)
        );
    }
}

#[test]
fn test_workload_submission_metadata() {
    use std::collections::HashMap;

    let mut metadata = HashMap::new();
    metadata.insert("key1".to_string(), "value1".to_string());
    metadata.insert("key2".to_string(), "value2".to_string());

    let workload = WorkloadSubmission::native()
        .executable("/bin/true")
        .metadata(metadata.clone())
        .build()
        .unwrap();

    assert_eq!(workload.metadata, metadata);
    assert_eq!(workload.metadata.get("key1"), Some(&"value1".to_string()));
    assert_eq!(workload.metadata.get("key2"), Some(&"value2".to_string()));
}

#[test]
fn test_workload_submission_environment() {
    use std::collections::HashMap;

    let mut env = HashMap::new();
    env.insert("VAR1".to_string(), "value1".to_string());
    env.insert("VAR2".to_string(), "value2".to_string());

    let workload = WorkloadSubmission::native()
        .executable("/bin/true")
        .environment(env.clone())
        .build()
        .unwrap();

    assert_eq!(workload.environment, env);
    assert_eq!(
        workload.environment.get("VAR1"),
        Some(&"value1".to_string())
    );
}

#[test]
fn test_native_workload_working_directory() {
    let workload = WorkloadSubmission::native()
        .executable("/bin/pwd")
        .working_dir("/tmp")
        .build()
        .unwrap();

    assert!(matches!(
        workload.workload_type,
        WorkloadType::Native { .. }
    ));
    if let WorkloadType::Native { working_dir, .. } = &workload.workload_type {
        assert_eq!(working_dir, &Some("/tmp".to_string()));
    }
}

#[test]
fn test_container_workload_environment() {
    use std::collections::HashMap;

    let mut env = HashMap::new();
    env.insert("CONTAINER_VAR".to_string(), "container_value".to_string());

    let workload = WorkloadSubmission::container()
        .image("alpine:latest")
        .environment(env.clone())
        .build()
        .expect("Failed to build workload");

    assert_eq!(workload.environment, env);
}

#[test]
fn test_python_workload_requirements() {
    let requirements = vec![
        "numpy==1.21.0".to_string(),
        "pandas>=1.3.0".to_string(),
        "requests~=2.26.0".to_string(),
    ];

    let workload = WorkloadSubmission::python()
        .script("import numpy")
        .requirements(requirements.clone())
        .build()
        .expect("Failed to build workload");

    assert!(matches!(
        workload.workload_type,
        WorkloadType::Python { .. }
    ));
    if let WorkloadType::Python {
        requirements: reqs, ..
    } = &workload.workload_type
    {
        assert_eq!(reqs, &requirements);
        assert_eq!(reqs.len(), 3);
    }
}

#[test]
fn test_workload_timeout() {
    let timeout = Duration::from_mins(5); // 5 minutes

    let workload = WorkloadSubmission::native()
        .executable("/bin/sleep")
        .args(vec!["100".to_string()])
        .timeout(timeout)
        .build()
        .unwrap();

    assert_eq!(workload.timeout, Some(timeout));
}

#[test]
fn test_workload_resources() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(8192),
        disk_mb: Some(10240),
        gpu_required: Some(true),
    };

    let workload = WorkloadSubmission::native()
        .executable("/bin/true")
        .resources(resources.clone())
        .build()
        .unwrap();

    assert_eq!(workload.resources, Some(resources));
}

#[test]
fn test_native_workload_builder_missing_executable() {
    let result = WorkloadSubmission::native().build();

    assert!(result.is_err());
    assert!(matches!(&result, Err(ClientError::Configuration(_))));
    if let Err(ClientError::Configuration(msg)) = result {
        assert!(msg.contains("Executable path is required"));
    }
}

#[test]
fn test_client_config_api_url() {
    // `TEST_TOADSTOOL_ENDPOINT` overrides; fallback matches `coordination_loopback_bootstrap_url` (port from `capability_fallback::COORDINATION`).
    let test_endpoint = std::env::var("TEST_TOADSTOOL_ENDPOINT").unwrap_or_else(|_| {
        toadstool_config::defaults::endpoints::coordination_loopback_bootstrap_url()
    });

    let config = ClientConfig {
        base_url: test_endpoint.clone(),
        ..Default::default()
    };

    let api_url = config.api_url("test");
    assert_eq!(api_url, format!("{test_endpoint}/api/v1/test"));
}

#[test]
fn test_client_config_default_values() {
    let config = ClientConfig::default();

    assert_eq!(
        config.base_url,
        format!(
            "http://{}:{}",
            toadstool_config::defaults::network::LOCALHOST,
            toadstool_config::defaults::network::API_PORT
        )
    );
    assert_eq!(
        config.max_retries,
        toadstool_config::defaults::retries::MAX_ATTEMPTS
    );
    assert_eq!(
        config.request_timeout,
        Duration::from_millis(toadstool_config::defaults::timeouts::REQUEST_MS)
    );
    assert_eq!(
        config.retry_backoff,
        Duration::from_millis(toadstool_config::defaults::retries::BACKOFF_MS)
    );
    assert!(config.auth.is_none());
    assert!(config.custom_headers.is_empty());
}

#[test]
fn test_wasm_workload_empty_args() {
    let workload = WorkloadSubmission::wasm()
        .module_data(vec![0, 97, 115, 109])
        .args(Vec::new())
        .build()
        .expect("Failed to build workload");

    assert!(matches!(workload.workload_type, WorkloadType::Wasm { .. }));
    if let WorkloadType::Wasm { args, .. } = &workload.workload_type {
        assert!(args.is_empty());
    }
}

#[test]
fn test_native_workload_absolute_path() {
    let workload = WorkloadSubmission::native()
        .executable("/usr/local/bin/custom-script")
        .build()
        .unwrap();

    assert!(matches!(
        workload.workload_type,
        WorkloadType::Native { .. }
    ));
    if let WorkloadType::Native { executable, .. } = &workload.workload_type {
        assert!(executable.starts_with('/'));
        assert_eq!(executable, "/usr/local/bin/custom-script");
    }
}

#[test]
fn test_container_workload_image_with_tag() {
    let workload = WorkloadSubmission::container()
        .image("myregistry.io/myapp:v1.2.3")
        .build()
        .expect("Failed to build workload");

    assert!(matches!(
        workload.workload_type,
        WorkloadType::Container { .. }
    ));
    if let WorkloadType::Container { image, .. } = &workload.workload_type {
        assert!(image.contains(':'));
        assert!(image.contains("v1.2.3"));
    }
}
