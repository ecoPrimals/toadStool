// SPDX-License-Identifier: AGPL-3.0-only
//! Workload submission implementation

use super::builders::{
    ContainerWorkloadBuilder, NativeWorkloadBuilder, PythonWorkloadBuilder, WasmWorkloadBuilder,
};

use super::types::WorkloadSubmission;

impl WorkloadSubmission {
    /// Create a native workload submission
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toadstool_client::WorkloadSubmission;
    /// use std::collections::HashMap;
    ///
    /// let workload = WorkloadSubmission::native()
    ///     .executable("/bin/echo")
    ///     .args(vec!["Hello, World!".to_string()])
    ///     .build()?;
    /// # Ok::<(), toadstool_client::ClientError>(())
    /// ```
    pub fn native() -> NativeWorkloadBuilder {
        NativeWorkloadBuilder::new()
    }

    /// Create a container workload submission
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toadstool_client::WorkloadSubmission;
    /// use std::collections::HashMap;
    ///
    /// let workload = WorkloadSubmission::container()
    ///     .image("ubuntu:latest")
    ///     .command(vec!["echo".to_string()])
    ///     .args(vec!["Hello from container!".to_string()])
    ///     .build();
    /// ```
    pub fn container() -> ContainerWorkloadBuilder {
        ContainerWorkloadBuilder::new()
    }

    /// Create a WASM workload submission
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toadstool_client::WorkloadSubmission;
    /// use std::collections::HashMap;
    ///
    /// # fn example() -> Result<(), std::io::Error> {
    /// let wasm_module = std::fs::read("hello.wasm")?;
    /// let workload = WorkloadSubmission::wasm()
    ///     .module_data(wasm_module)
    ///     .args(vec!["arg1".to_string(), "arg2".to_string()])
    ///     .build();
    /// # Ok(())
    /// # }
    /// ```
    pub fn wasm() -> WasmWorkloadBuilder {
        WasmWorkloadBuilder::new()
    }

    /// Create a Python workload submission
    ///
    /// # Examples
    ///
    /// ```rust
    /// use toadstool_client::WorkloadSubmission;
    /// use std::collections::HashMap;
    ///
    /// let workload = WorkloadSubmission::python()
    ///     .script("print('Hello from Python!')")
    ///     .requirements(vec!["requests>=2.28.0".to_string()])
    ///     .build();
    /// ```
    pub fn python() -> PythonWorkloadBuilder {
        PythonWorkloadBuilder::new()
    }
}
