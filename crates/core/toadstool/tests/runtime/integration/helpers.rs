// SPDX-License-Identifier: AGPL-3.0-or-later
// ============================================================================
// Helper Functions
// ============================================================================

use std::collections::HashMap;
use std::time::Duration;

use toadstool::ExecutionRequest;
use uuid::Uuid;

pub(super) fn create_test_execution_request() -> ExecutionRequest {
    use toadstool::WorkloadSpec;

    // Create a container workload that doesn't require file validation
    let workload = WorkloadSpec::Container {
        image: "alpine:latest".to_string(),
        command: Some(vec!["echo".to_string()]),
        args: Some(vec!["hello".to_string()]),
        env_vars: HashMap::new(),
        working_dir: None,
        volumes: vec![],
        ports: vec![],
        registry_auth: None,
    };

    ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload,
        runtime_hint: None,
        resources: toadstool::resources::ResourceRequirements::default(),
        security_context: toadstool::SecurityContext::default(),
        timeout: Some(Duration::from_secs(300)),
        environment: HashMap::new(),
        input_data: toadstool::ExecutionInput::default(),
        callback_config: None,
        encryption_config: None,
    }
}
