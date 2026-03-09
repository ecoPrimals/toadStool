// SPDX-License-Identifier: AGPL-3.0-only
//! Native workload builder for executing native binaries

use std::collections::HashMap;
use std::time::Duration;

use super::super::error::ClientError;
use super::super::types::{JobPriority, ResourceRequirements, WorkloadSubmission, WorkloadType};

/// Builder for native executable workloads
#[must_use]
pub struct NativeWorkloadBuilder {
    executable: Option<String>,
    args: Vec<String>,
    working_dir: Option<String>,
    environment: HashMap<String, String>,
    priority: Option<JobPriority>,
    timeout: Option<Duration>,
    resources: Option<ResourceRequirements>,
    metadata: HashMap<String, String>,
}

impl Default for NativeWorkloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeWorkloadBuilder {
    /// Create a new native workload builder
    pub fn new() -> Self {
        Self {
            executable: None,
            args: Vec::new(),
            working_dir: None,
            environment: HashMap::new(),
            priority: None,
            timeout: None,
            resources: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the executable path for the native workload
    ///
    /// # Arguments
    ///
    /// * `executable` - The path to the executable to run
    pub fn executable<S: Into<String>>(mut self, executable: S) -> Self {
        self.executable = Some(executable.into());
        self
    }

    /// Set command-line arguments for the executable
    ///
    /// # Arguments
    ///
    /// * `args` - Vector of command-line arguments to pass to the executable
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set the working directory for the execution
    ///
    /// # Arguments
    ///
    /// * `working_dir` - The directory path where the executable should run
    pub fn working_dir<S: Into<String>>(mut self, working_dir: S) -> Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    /// Set environment variables for the execution
    ///
    /// # Arguments
    ///
    /// * `environment` - HashMap of environment variable names to values
    pub fn environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    /// Set the execution priority for the workload
    ///
    /// # Arguments
    ///
    /// * `priority` - The job priority level (affects scheduling order)
    pub fn priority(mut self, priority: JobPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set the maximum execution timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum duration the workload is allowed to run before being terminated
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set resource requirements for the workload
    pub fn resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Set metadata for the workload
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Build the workload submission
    ///
    /// # Errors
    ///
    /// Returns an error if the workload configuration is invalid
    pub fn build(self) -> Result<WorkloadSubmission, ClientError> {
        let executable = self.executable.ok_or_else(|| {
            ClientError::Configuration(
                "Executable path is required for native workload. Use .executable(\"/path/to/binary\") to set it.".to_string()
            )
        })?;

        Ok(WorkloadSubmission {
            workload_type: WorkloadType::Native {
                executable,
                args: self.args,
                working_dir: self.working_dir,
            },
            runtime_hint: Some("native".to_string()),
            priority: self.priority,
            timeout: self.timeout,
            environment: self.environment,
            resources: self.resources,
            metadata: self.metadata,
        })
    }
}
