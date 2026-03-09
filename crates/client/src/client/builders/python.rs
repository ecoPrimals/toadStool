// SPDX-License-Identifier: AGPL-3.0-only
//! Python workload builder for executing Python scripts

use std::collections::HashMap;
use std::time::Duration;

use super::super::types::{JobPriority, ResourceRequirements, WorkloadSubmission, WorkloadType};

/// Builder for Python workloads
#[must_use]
pub struct PythonWorkloadBuilder {
    script: Option<String>,
    requirements: Vec<String>,
    environment: HashMap<String, String>,
    priority: Option<JobPriority>,
    timeout: Option<Duration>,
    resources: Option<ResourceRequirements>,
    metadata: HashMap<String, String>,
}

impl Default for PythonWorkloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonWorkloadBuilder {
    /// Create a new Python workload builder
    pub fn new() -> Self {
        Self {
            script: None,
            requirements: Vec::new(),
            environment: HashMap::new(),
            priority: None,
            timeout: None,
            resources: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the Python script
    ///
    /// # Arguments
    ///
    /// * `script` - The Python script to execute.
    pub fn script<S: Into<String>>(mut self, script: S) -> Self {
        self.script = Some(script.into());
        self
    }

    /// Set Python requirements
    ///
    /// # Arguments
    ///
    /// * `requirements` - Vector of Python package requirements (e.g., "requests>=2.28.0").
    pub fn requirements(mut self, requirements: Vec<String>) -> Self {
        self.requirements = requirements;
        self
    }

    /// Set environment variables
    ///
    /// # Arguments
    ///
    /// * `environment` - HashMap of environment variable names to values
    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    /// Set job priority
    ///
    /// # Arguments
    ///
    /// * `priority` - The job priority level (affects scheduling order)
    pub fn priority(mut self, priority: JobPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set execution timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum duration the workload is allowed to run before being terminated
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set resource requirements
    ///
    /// # Arguments
    ///
    /// * `resources` - Resource requirements for the workload
    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Set metadata
    ///
    /// # Arguments
    ///
    /// * `metadata` - HashMap of metadata key-value pairs
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Build the workload submission
    ///
    /// # Errors
    /// Returns an error if script is not set
    pub fn build(self) -> Result<WorkloadSubmission, String> {
        let script = self
            .script
            .ok_or_else(|| "Script is required for Python workload".to_string())?;

        Ok(WorkloadSubmission {
            workload_type: WorkloadType::Python {
                script,
                requirements: self.requirements,
            },
            runtime_hint: Some("python".to_string()),
            priority: self.priority,
            timeout: self.timeout,
            environment: self.environment,
            resources: self.resources,
            metadata: self.metadata,
        })
    }
}
