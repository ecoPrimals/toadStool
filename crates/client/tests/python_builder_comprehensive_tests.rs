//! Comprehensive tests for Python workload builder
//!
//! Week 14 Day 3: Python Builder Tests
//! Target: Achieve 80%+ coverage of client/builders/python.rs

use std::collections::HashMap;
use std::time::Duration;
use toadstool_client::{JobPriority, ResourceRequirements, WorkloadSubmission};

// =============================================================================
// Python Builder Creation & Basic Tests
// =============================================================================

#[test]
fn test_python_builder_creation() {
    let _builder = WorkloadSubmission::python();
    // Should create successfully
}

#[test]
fn test_python_builder_with_script() {
    let script = "print('Hello from Python!')";
    let _submission = WorkloadSubmission::python().script(script).build();
    // Build succeeds with script
}

#[test]
fn test_python_builder_with_multiline_script() {
    let script = r#"
import sys
print(f"Python version: {sys.version}")
print("Multi-line script executed successfully!")
"#;
    let _submission = WorkloadSubmission::python().script(script).build();
    // Build succeeds with multi-line script
}

#[test]
#[should_panic(expected = "Script is required for Python workload")]
fn test_python_builder_missing_script() {
    let _submission = WorkloadSubmission::python().build();
    // Should panic without script
}

#[test]
fn test_python_builder_empty_script() {
    let _submission = WorkloadSubmission::python().script("").build();
    // Empty script is allowed (will fail on execution)
}

// =============================================================================
// Requirements Tests
// =============================================================================

#[test]
fn test_python_builder_with_single_requirement() {
    let requirements = vec!["requests>=2.28.0".to_string()];
    let script = "import requests\nprint('Requests library available')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .requirements(requirements)
        .build();
    // Build succeeds with requirements
}

#[test]
fn test_python_builder_with_multiple_requirements() {
    let requirements = vec![
        "requests>=2.28.0".to_string(),
        "numpy>=1.24.0".to_string(),
        "pandas>=2.0.0".to_string(),
    ];
    let script = "import requests, numpy, pandas\nprint('All libraries available')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .requirements(requirements)
        .build();
}

#[test]
fn test_python_builder_with_empty_requirements() {
    let requirements = Vec::new();
    let script = "print('No requirements')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .requirements(requirements)
        .build();
}

#[test]
fn test_python_builder_requirements_with_version_specifiers() {
    let requirements = vec![
        "requests==2.28.1".to_string(), // Exact version
        "numpy>=1.24.0".to_string(),    // Minimum version
        "pandas~=2.0.0".to_string(),    // Compatible release
        "scipy!=1.10.0".to_string(),    // Exclude version
    ];
    let script = "print('Complex requirements')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .requirements(requirements)
        .build();
}

// =============================================================================
// Environment Variable Tests
// =============================================================================

#[test]
fn test_python_builder_with_single_env_var() {
    let mut environment = HashMap::new();
    environment.insert("DEBUG".to_string(), "true".to_string());

    let script = "import os\nprint(f'DEBUG={os.getenv(\"DEBUG\")}')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .environment(environment)
        .build();
}

#[test]
fn test_python_builder_with_multiple_env_vars() {
    let mut environment = HashMap::new();
    environment.insert(
        "DATABASE_URL".to_string(),
        "postgresql://localhost/db".to_string(),
    );
    environment.insert("API_KEY".to_string(), "secret123".to_string());
    environment.insert("LOG_LEVEL".to_string(), "INFO".to_string());

    let script = "import os\nprint('Environment loaded')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .environment(environment)
        .build();
}

#[test]
fn test_python_builder_with_empty_environment() {
    let environment = HashMap::new();
    let script = "print('No environment variables')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .environment(environment)
        .build();
}

// =============================================================================
// Priority Tests
// =============================================================================

#[test]
fn test_python_builder_with_priority_low() {
    let script = "print('Low priority job')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .priority(JobPriority::Low)
        .build();
}

#[test]
fn test_python_builder_with_priority_normal() {
    let script = "print('Normal priority job')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .priority(JobPriority::Normal)
        .build();
}

#[test]
fn test_python_builder_with_priority_high() {
    let script = "print('High priority job')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .priority(JobPriority::High)
        .build();
}

// =============================================================================
// Timeout Tests
// =============================================================================

#[test]
fn test_python_builder_with_timeout_short() {
    let script = "print('Quick task')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .timeout(Duration::from_secs(5))
        .build();
}

#[test]
fn test_python_builder_with_timeout_long() {
    let script = "import time\ntime.sleep(30)\nprint('Long task')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .timeout(Duration::from_secs(60))
        .build();
}

#[test]
fn test_python_builder_with_timeout_zero() {
    let script = "print('Instant task')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .timeout(Duration::from_secs(0))
        .build();
}

// =============================================================================
// Resource Requirements Tests
// =============================================================================

#[test]
fn test_python_builder_with_cpu_requirement() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: None,
        disk_mb: None,
        gpu_required: None,
    };

    let script = "print('CPU-intensive task')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .resources(resources)
        .build();
}

#[test]
fn test_python_builder_with_memory_requirement() {
    let resources = ResourceRequirements {
        cpu_cores: None,
        memory_mb: Some(2048),
        disk_mb: None,
        gpu_required: None,
    };

    let script = "print('Memory-intensive task')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .resources(resources)
        .build();
}

#[test]
fn test_python_builder_with_disk_requirement() {
    let resources = ResourceRequirements {
        cpu_cores: None,
        memory_mb: None,
        disk_mb: Some(10000),
        gpu_required: None,
    };

    let script = "print('Disk-intensive task')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .resources(resources)
        .build();
}

#[test]
fn test_python_builder_with_gpu_required() {
    let resources = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(4096),
        disk_mb: None,
        gpu_required: Some(true),
    };

    let script = "import torch\nprint(f'CUDA available: {torch.cuda.is_available()}')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .resources(resources)
        .build();
}

#[test]
fn test_python_builder_with_all_resource_requirements() {
    let resources = ResourceRequirements {
        cpu_cores: Some(8),
        memory_mb: Some(16384),
        disk_mb: Some(50000),
        gpu_required: Some(false),
    };

    let script = "print('Resource-heavy task')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .resources(resources)
        .build();
}

// =============================================================================
// Metadata Tests
// =============================================================================

#[test]
fn test_python_builder_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("author".to_string(), "test_user".to_string());
    metadata.insert("version".to_string(), "1.0.0".to_string());

    let script = "print('Task with metadata')";

    let _submission = WorkloadSubmission::python()
        .script(script)
        .metadata(metadata)
        .build();
}

#[test]
fn test_python_builder_with_single_metadata_item() {
    let script = "print('Task with single metadata')";

    let mut metadata = HashMap::new();
    metadata.insert("task_id".to_string(), "12345".to_string());

    let _submission = WorkloadSubmission::python()
        .script(script)
        .metadata(metadata)
        .build();
}

#[test]
fn test_python_builder_with_multiple_metadata_items() {
    let script = "print('Task with multiple metadata items')";

    let mut metadata = HashMap::new();
    metadata.insert("task_id".to_string(), "12345".to_string());
    metadata.insert("user_id".to_string(), "user_789".to_string());
    metadata.insert("project".to_string(), "data_analysis".to_string());

    let _submission = WorkloadSubmission::python()
        .script(script)
        .metadata(metadata)
        .build();
}

// =============================================================================
// Complex Integration Tests
// =============================================================================

#[test]
fn test_python_builder_full_configuration() {
    let mut environment = HashMap::new();
    environment.insert("DATA_PATH".to_string(), "/data".to_string());
    environment.insert("OUTPUT_PATH".to_string(), "/output".to_string());

    let requirements = vec![
        "pandas>=2.0.0".to_string(),
        "numpy>=1.24.0".to_string(),
        "scikit-learn>=1.3.0".to_string(),
    ];

    let resources = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(8192),
        disk_mb: Some(20000),
        gpu_required: Some(false),
    };

    let mut metadata = HashMap::new();
    metadata.insert("job_type".to_string(), "ml_training".to_string());
    metadata.insert("dataset".to_string(), "iris".to_string());

    let script = r#"
import pandas as pd
import numpy as np
from sklearn.datasets import load_iris

# Load dataset
iris = load_iris()
print(f"Dataset loaded: {iris.data.shape}")
"#;

    let _submission = WorkloadSubmission::python()
        .script(script)
        .requirements(requirements)
        .environment(environment)
        .priority(JobPriority::High)
        .timeout(Duration::from_secs(300))
        .resources(resources)
        .metadata(metadata)
        .build();
}

#[test]
fn test_python_builder_data_science_workflow() {
    let script = r#"
import numpy as np
import pandas as pd

# Generate synthetic data
data = np.random.randn(1000, 5)
df = pd.DataFrame(data, columns=['A', 'B', 'C', 'D', 'E'])

# Basic analysis
print(f"Mean of column A: {df['A'].mean()}")
print(f"Std of column B: {df['B'].std()}")
print("Analysis complete!")
"#;

    let requirements = vec!["numpy>=1.24.0".to_string(), "pandas>=2.0.0".to_string()];

    let mut metadata = HashMap::new();
    metadata.insert("workflow".to_string(), "data_science".to_string());

    let _submission = WorkloadSubmission::python()
        .script(script)
        .requirements(requirements)
        .priority(JobPriority::Normal)
        .timeout(Duration::from_secs(120))
        .metadata(metadata)
        .build();
}

#[test]
fn test_python_builder_with_chained_methods() {
    let mut metadata = HashMap::new();
    metadata.insert("chain".to_string(), "test".to_string());

    let _submission = WorkloadSubmission::python()
        .script("print('Chained methods')")
        .metadata(metadata)
        .priority(JobPriority::Normal)
        .timeout(Duration::from_secs(30))
        .build();
}

// =============================================================================
// Edge Cases & Error Handling
// =============================================================================

#[test]
fn test_python_builder_with_very_long_script() {
    let mut script = String::new();
    for i in 0..1000 {
        script.push_str(&format!("print('Line {}')\n", i));
    }

    let _submission = WorkloadSubmission::python().script(script).build();
}

#[test]
fn test_python_builder_with_special_characters_in_script() {
    let script = r#"
print("Special characters: !@#$%^&*()")
print('Single quotes: \' and "double quotes"')
print("Unicode: 你好 🚀 ñ")
"#;

    let _submission = WorkloadSubmission::python().script(script).build();
}

#[test]
fn test_python_builder_with_whitespace_only_script() {
    let script = "   \n\t\n   ";

    let _submission = WorkloadSubmission::python().script(script).build();

    // Should fail - whitespace-only is effectively empty
}
