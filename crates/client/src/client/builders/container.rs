// SPDX-License-Identifier: AGPL-3.0-only
//! Container workload builder for running containerized applications

use std::collections::HashMap;
use std::time::Duration;

use super::super::types::{JobPriority, ResourceRequirements, WorkloadSubmission, WorkloadType};

/// Builder for container workloads
#[must_use]
pub struct ContainerWorkloadBuilder {
    image: Option<String>,
    command: Option<Vec<String>>,
    args: Option<Vec<String>>,
    working_dir: Option<String>,
    environment: HashMap<String, String>,
    priority: Option<JobPriority>,
    timeout: Option<Duration>,
    resources: Option<ResourceRequirements>,
    metadata: HashMap<String, String>,
}

impl Default for ContainerWorkloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerWorkloadBuilder {
    /// Create a new container workload builder
    pub fn new() -> Self {
        Self {
            image: None,
            command: None,
            args: None,
            working_dir: None,
            environment: HashMap::new(),
            priority: None,
            timeout: None,
            resources: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the container image
    ///
    /// # Examples
    /// ```
    /// use toadstool_client::WorkloadSubmission;
    /// let workload = WorkloadSubmission::container()
    ///     .image("alpine:latest")
    ///     .build();
    /// ```
    pub fn image<S: Into<String>>(mut self, image: S) -> Self {
        self.image = Some(image.into());
        self
    }

    /// Set the container command
    pub fn command(mut self, command: Vec<String>) -> Self {
        self.command = Some(command);
        self
    }

    /// Set command-line arguments for the executable
    ///
    /// # Arguments
    ///
    /// * `args` - Vector of command-line arguments to pass to the executable
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
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
    /// Returns an error if image is not set
    pub fn build(self) -> Result<WorkloadSubmission, String> {
        let image = self
            .image
            .ok_or_else(|| "Image is required for container workload".to_string())?;

        Ok(WorkloadSubmission {
            workload_type: WorkloadType::Container {
                image,
                command: self.command,
                args: self.args,
                working_dir: self.working_dir,
            },
            runtime_hint: Some("container".to_string()),
            priority: self.priority,
            timeout: self.timeout,
            environment: self.environment,
            resources: self.resources,
            metadata: self.metadata,
        })
    }
}
