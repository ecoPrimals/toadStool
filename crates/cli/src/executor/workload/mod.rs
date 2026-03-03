// SPDX-License-Identifier: AGPL-3.0-or-later
//! Direct workload execution without biome.yaml
//!
//! This module implements the `toadstool execute` command for running
//! workloads directly using ToadStool's runtime engines.
//!
//! ## Module Structure (Refactored by Domain)
//!
//! - `spec`: Workload file format types (WorkloadFile, ExecutionSpec, etc.)
//! - `loading`: Loading and parsing workload files from disk
//! - `conversion`: Converting to ToadStool WorkloadSpec
//! - `runtime`: Runtime type selection and engine registration
//! - `reporting`: Output formatting (text/JSON)

mod conversion;
mod loading;
mod reporting;
mod runtime;
mod spec;

use crate::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool::{
    execution::ExecutionRequest,
    runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy},
};

// Re-export public types for external consumers
pub use spec::{ExecutionSpec, ResourceSpec, SecuritySpec, WorkloadFile, WorkloadMetadata};

use conversion::{
    convert_resource_requirements, convert_security_context, convert_to_workload_spec,
};
use loading::load_workload_file;
use reporting::{print_json_output, print_text_output};
use runtime::{infer_runtime_type, parse_runtime_hint, register_runtime_engines};

/// Execute a workload from a specification file
pub async fn execute_workload(
    workload_path: &PathBuf,
    runtime_hint: Option<String>,
    env_overrides: Vec<String>,
    timeout_secs: u64,
    output_format: String,
) -> Result<()> {
    info!(
        "📖 Loading workload specification: {}",
        workload_path.display()
    );

    // Load workload file
    let workload_file = load_workload_file(workload_path).await?;

    info!("✅ Loaded workload: {}", workload_file.metadata.name);
    if let Some(desc) = &workload_file.metadata.description {
        info!("   Description: {}", desc);
    }

    // Parse environment overrides (zero-copy: only allocate when inserting into HashMap)
    let env_map: HashMap<String, String> = env_overrides
        .iter()
        .filter_map(|env_pair| {
            env_pair
                .split_once('=')
                .map(|(key, value)| (key.to_string(), value.to_string()))
        })
        .collect();

    // Create runtime orchestrator
    info!("🔧 Initializing runtime orchestrator");
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    // Register available runtime engines
    register_runtime_engines(&orchestrator).await?;

    // Convert workload spec to ToadStool WorkloadSpec
    let workload_spec = convert_to_workload_spec(&workload_file, env_map)?;

    // Determine runtime type
    let runtime_type = if let Some(hint) = runtime_hint {
        parse_runtime_hint(&hint)?
    } else {
        infer_runtime_type(&workload_spec)
    };

    // Create execution request
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: workload_spec,
        runtime_hint: Some(runtime_type),
        resources: convert_resource_requirements(&workload_file.resources),
        security_context: convert_security_context(&workload_file.security),
        timeout: Some(Duration::from_secs(timeout_secs)),
        environment: HashMap::new(),
        input_data: Default::default(),
        callback_config: None,
        encryption_config: None,
    };

    info!("🚀 Executing workload: {}", workload_file.metadata.name);
    debug!("   Execution ID: {}", request.execution_id);
    debug!("   Runtime: {:?}", request.runtime_hint);

    // Execute!
    let start_time = std::time::Instant::now();
    let response = orchestrator.execute(request).await?;
    let duration = start_time.elapsed();

    // Display results
    match output_format.as_str() {
        "json" => print_json_output(&response, duration)?,
        _ => print_text_output(&workload_file, &response, duration)?,
    }

    Ok(())
}
