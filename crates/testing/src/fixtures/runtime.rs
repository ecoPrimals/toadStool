// SPDX-License-Identifier: AGPL-3.0-only
//! Runtime test fixtures for integration testing

/// Test workload builder
pub struct TestWorkloadBuilder {
    workload_type: String,
    entry_point: Option<String>,
    timeout_seconds: u64,
    cpu_cores: f64,
    memory_mb: u64,
}

impl TestWorkloadBuilder {
    /// Create a new workload builder with the given type
    pub fn new(workload_type: impl Into<String>) -> Self {
        Self {
            workload_type: workload_type.into(),
            entry_point: None,
            timeout_seconds: 30,
            cpu_cores: 1.0,
            memory_mb: 256,
        }
    }

    /// Create a WASM workload builder
    pub fn wasm() -> Self {
        Self::new("Wasm")
    }

    /// Create a Native workload builder
    pub fn native() -> Self {
        Self::new("Native")
    }

    /// Create a Container workload builder
    pub fn container() -> Self {
        Self::new("Container")
    }

    /// Create a Python workload builder
    pub fn python() -> Self {
        Self::new("Python")
    }

    /// Set the entry point (e.g. "main" for WASM)
    pub fn with_entry_point(mut self, entry: impl Into<String>) -> Self {
        self.entry_point = Some(entry.into());
        self
    }

    /// Set the timeout in seconds
    pub const fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set CPU and memory resources
    pub const fn with_resources(mut self, cpu_cores: f64, memory_mb: u64) -> Self {
        self.cpu_cores = cpu_cores;
        self.memory_mb = memory_mb;
        self
    }

    /// Build the workload config as JSON
    pub fn build(self) -> serde_json::Value {
        let mut config = serde_json::json!({
            "workload_type": self.workload_type,
            "resources": {
                "cpu_cores": self.cpu_cores,
                "memory_mb": self.memory_mb,
            },
            "timeout_seconds": self.timeout_seconds,
        });

        if let Some(entry_point) = self.entry_point {
            config["entry_point"] = serde_json::Value::String(entry_point);
        }

        config
    }
}

/// Create a simple WASM test workload
pub fn create_wasm_test_workload() -> serde_json::Value {
    TestWorkloadBuilder::wasm().with_entry_point("main").build()
}

/// Create a simple Native test workload
pub fn create_native_test_workload() -> serde_json::Value {
    TestWorkloadBuilder::native()
        .with_entry_point("/bin/echo")
        .build()
}

/// Create a resource-intensive test workload
pub fn create_heavy_test_workload() -> serde_json::Value {
    TestWorkloadBuilder::wasm()
        .with_resources(4.0, 2048)
        .with_timeout(300)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_builder() {
        let workload = TestWorkloadBuilder::wasm()
            .with_entry_point("test_main")
            .with_timeout(60)
            .build();

        assert_eq!(workload["workload_type"], "Wasm");
        assert_eq!(workload["entry_point"], "test_main");
        assert_eq!(workload["timeout_seconds"], 60);
    }

    #[test]
    fn test_create_wasm_test_workload() {
        let workload = create_wasm_test_workload();
        assert_eq!(workload["workload_type"], "Wasm");
    }
}
