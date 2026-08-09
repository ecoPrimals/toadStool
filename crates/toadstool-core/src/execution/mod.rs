// SPDX-License-Identifier: AGPL-3.0-or-later
//! Execution data types — requests, responses, status, config.
//! Pure data structures (no async runtime) suitable for WASM targets.

mod types;

pub use types::{
    CallbackConfig, CallbackEvent, ExecutionInput, ExecutionOutput, ExecutionRequest,
    ExecutionResponse, ExecutionStatus, LoggingConfig, RuntimeCapabilities, RuntimeConfig,
    RuntimeType,
};
