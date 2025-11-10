//! WebAssembly workload builder for executing WASM modules

use std::collections::HashMap;
use std::time::Duration;

use super::super::types::{JobPriority, ResourceRequirements, WorkloadSubmission, WorkloadType};

/// Builder for WASM workloads
#[must_use]
pub struct WasmWorkloadBuilder {
    module_data: Option<Vec<u8>>,
    args: Vec<String>,
    environment: HashMap<String, String>,
    priority: Option<JobPriority>,
    timeout: Option<Duration>,
    resources: Option<ResourceRequirements>,
    metadata: HashMap<String, String>,
}

impl Default for WasmWorkloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmWorkloadBuilder {
    /// Create a new WASM workload builder
    pub fn new() -> Self {
        Self {
            module_data: None,
            args: Vec::new(),
            environment: HashMap::new(),
            priority: None,
            timeout: None,
            resources: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the WASM module data
    pub fn module_data(mut self, module_data: Vec<u8>) -> Self {
        self.module_data = Some(module_data);
        self
    }

    /// Set command line arguments
    ///
    /// # Arguments
    ///
    /// * `args` - Vector of command-line arguments to pass to the executable
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
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
    /// # Panics
    /// Panics if the module data is not set, as it is required for WASM workloads
    #[must_use]
    pub fn build(self) -> WorkloadSubmission {
        WorkloadSubmission {
            workload_type: WorkloadType::Wasm {
                module_data: self
                    .module_data
                    .expect("Module data is required for WASM workload"),
                args: self.args,
            },
            runtime_hint: Some("wasm".to_string()),
            priority: self.priority,
            timeout: self.timeout,
            environment: self.environment,
            resources: self.resources,
            metadata: self.metadata,
        }
    }
}
