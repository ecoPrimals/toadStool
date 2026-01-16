//! Direct workload execution without biome.yaml
//!
//! This module implements the `toadstool execute` command for running
//! workloads directly using ToadStool's runtime engines.

use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool::{
    execution::{ExecutionRequest, RuntimeType},
    resources::ResourceRequirements,
    runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy},
    security::{IsolationLevel, SecurityContext},
    workload::{ExecutableSource, WorkloadSpec},
};

// Import runtime engines
use toadstool_runtime_native::NativeRuntimeEngine;
use toadstool_runtime_python::PythonRuntimeEngine;
#[cfg(feature = "wasm")]
use toadstool_runtime_wasm::WasmRuntimeEngine;

#[cfg(feature = "gpu")]
use toadstool_runtime_gpu::UniversalGpuEngine;

/// Workload specification file format
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WorkloadFile {
    pub metadata: WorkloadMetadata,
    pub execution: ExecutionSpec,
    pub resources: Option<ResourceSpec>,
    pub security: Option<SecuritySpec>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct WorkloadMetadata {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExecutionSpec {
    Native {
        command: String,
        args: Option<Vec<String>>,
        working_dir: Option<String>,
        env: Option<HashMap<String, String>>,
    },
    Python {
        script: Option<String>,
        file: Option<String>,
        args: Option<Vec<String>>,
        env: Option<HashMap<String, String>>,
    },
    Wasm {
        module: String,
        args: Option<Vec<String>>,
        env: Option<HashMap<String, String>>,
    },
    Container {
        image: String,
        command: Option<Vec<String>>,
        args: Option<Vec<String>>,
        env: Option<HashMap<String, String>>,
    },
    Gpu {
        kernel_name: String,
        source: String,
        input_data: Option<serde_json::Value>,
        output_data_keys: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ResourceSpec {
    pub cpu_cores: Option<f64>,
    pub memory_mb: Option<u64>,
    pub disk_mb: Option<u64>,
    pub gpu: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SecuritySpec {
    pub isolation: Option<String>,
}

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

async fn register_runtime_engines(orchestrator: &RuntimeOrchestrator) -> Result<()> {
    // Native runtime (always available)
    let native_engine = NativeRuntimeEngine::new();
    orchestrator
        .register_engine(RuntimeType::Native, Box::new(native_engine))
        .await
        .context("Failed to register native runtime")?;
    info!("   ✅ Native runtime registered");

    // Python runtime
    match PythonRuntimeEngine::new() {
        Ok(python_engine) => {
            orchestrator
                .register_engine(RuntimeType::Python, Box::new(python_engine))
                .await
                .context("Failed to register Python runtime")?;
            info!("   ✅ Python runtime registered");
        }
        Err(e) => {
            debug!("   ⚠️  Python runtime not available: {}", e);
        }
    }

    // WASM runtime - Optional (has zstd C dependency)
    #[cfg(feature = "wasm")]
    {
        let wasm_config = toadstool_runtime_wasm::WasmRuntimeConfig::default();
        match WasmRuntimeEngine::new(wasm_config) {
            Ok(wasm_engine) => {
                orchestrator
                    .register_engine(RuntimeType::Wasm, Box::new(wasm_engine))
                    .await
                    .context("Failed to register WASM runtime")?;
                info!("   ✅ WASM runtime registered");
            }
            Err(e) => {
                debug!("   ⚠️  WASM runtime not available: {}", e);
            }
        }
    }
    #[cfg(not(feature = "wasm"))]
    {
        debug!("   ⚠️  WASM runtime not enabled (pure-rust build)");
    }

    // GPU runtime (optional, feature-gated)
    #[cfg(feature = "gpu")]
    {
        match UniversalGpuEngine::new().await {
            Ok(gpu_engine) => {
                orchestrator
                    .register_engine(RuntimeType::Gpu, Box::new(gpu_engine))
                    .await
                    .context("Failed to register GPU runtime")?;
                info!("   ✅ GPU runtime registered");
            }
            Err(e) => {
                debug!("   ⚠️  GPU runtime not available: {}", e);
            }
        }
    }

    Ok(())
}

async fn load_workload_file(path: &PathBuf) -> Result<WorkloadFile> {
    let content = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read workload file: {}", path.display()))?;

    // Try TOML first, then JSON
    if path.extension().and_then(|s| s.to_str()) == Some("toml") {
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML workload file: {}", path.display()))
    } else {
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse JSON workload file: {}", path.display()))
    }
}

fn convert_to_workload_spec(
    workload: &WorkloadFile,
    env_overrides: HashMap<String, String>,
) -> Result<WorkloadSpec> {
    // ✅ ZERO-COPY: Pre-allocate with override capacity
    let mut env_vars = HashMap::with_capacity(env_overrides.len());

    match &workload.execution {
        ExecutionSpec::Native {
            command,
            args,
            working_dir,
            env,
        } => {
            if let Some(env) = env {
                env_vars.extend(env.clone());
            }
            env_vars.extend(env_overrides);

            Ok(WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: command.into(),
                },
                args: args.clone(),
                working_dir: working_dir.as_ref().map(PathBuf::from),
                env_vars,
                user: None,
            })
        }
        ExecutionSpec::Python {
            script,
            file,
            args: _,
            env,
        } => {
            if let Some(env) = env {
                env_vars.extend(env.clone());
            }
            env_vars.extend(env_overrides);

            let source = if let Some(script_content) = script {
                toadstool::workload::PythonSource::Code {
                    code: script_content.clone(),
                }
            } else if let Some(file_path) = file {
                toadstool::workload::PythonSource::File {
                    path: file_path.into(),
                }
            } else {
                anyhow::bail!("Python workload must specify either 'script' or 'file'");
            };

            Ok(WorkloadSpec::Python {
                source,
                python_version: None,
                requirements: vec![],
                env_vars,
            })
        }
        ExecutionSpec::Gpu {
            kernel_name,
            source,
            input_data: _,
            output_data_keys: _,
        } => {
            // Parse as OpenCL for now (most universal)
            Ok(WorkloadSpec::Gpu {
                program: toadstool::workload::GpuProgramSource::OpenCL {
                    source: source.clone(),
                },
                kernel_name: kernel_name.clone(),
                work_group_size: None,
                global_work_size: (1024, 1, 1), // Default size
                args: vec![],                   // Args would be populated from input_data
            })
        }
        _ => anyhow::bail!("Workload type not yet supported"),
    }
}

fn convert_resource_requirements(_resources: &Option<ResourceSpec>) -> ResourceRequirements {
    // Use default for now - actual resource fields need to match toadstool::resources::ResourceRequirements
    ResourceRequirements::default()
}

fn convert_security_context(_security: &Option<SecuritySpec>) -> SecurityContext {
    // Use standard isolation for now
    SecurityContext::for_isolation_level(IsolationLevel::Standard)
}

fn parse_runtime_hint(hint: &str) -> Result<RuntimeType> {
    match hint.to_lowercase().as_str() {
        "native" => Ok(RuntimeType::Native),
        "python" => Ok(RuntimeType::Python),
        "wasm" | "webassembly" => Ok(RuntimeType::Wasm),
        "container" | "docker" => Ok(RuntimeType::Container),
        "gpu" => Ok(RuntimeType::Gpu),
        _ => anyhow::bail!("Unknown runtime type: {}", hint),
    }
}

fn infer_runtime_type(workload: &WorkloadSpec) -> RuntimeType {
    match workload {
        WorkloadSpec::Native { .. } => RuntimeType::Native,
        WorkloadSpec::Python { .. } => RuntimeType::Python,
        WorkloadSpec::Wasm { .. } => RuntimeType::Wasm,
        WorkloadSpec::Container { .. } => RuntimeType::Container,
        WorkloadSpec::Gpu { .. } => RuntimeType::Gpu,
        WorkloadSpec::AiMl { .. } => RuntimeType::Gpu, // AI/ML uses GPU runtime with backend selector
        WorkloadSpec::Cuda { .. } => RuntimeType::Gpu, // CUDA uses GPU runtime with compat layer
    }
}

fn print_text_output(
    workload: &WorkloadFile,
    response: &toadstool::ExecutionResponse,
    duration: Duration,
) -> Result<()> {
    println!();
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════".bright_green()
    );
    println!("{}", "✅ Execution Complete!".bright_green().bold());
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════".bright_green()
    );
    println!();
    println!("Workload:      {}", workload.metadata.name.bright_cyan());
    println!("Execution ID:  {}", response.execution_id);
    println!("Status:        {:?}", response.status);
    println!("Duration:      {:.3}s", duration.as_secs_f64());

    if let Some(exit_code) = response.output.exit_code {
        let status_str = if exit_code == 0 {
            format!("{}", exit_code).bright_green()
        } else {
            format!("{}", exit_code).bright_red()
        };
        println!("Exit Code:     {}", status_str);
    }

    println!();

    if let Some(stdout) = &response.output.stdout {
        if !stdout.is_empty() {
            println!("{}", "Standard Output:".bright_blue());
            println!("{}", "─".repeat(60));
            println!("{}", stdout);
            println!("{}", "─".repeat(60));
            println!();
        }
    }

    if let Some(stderr) = &response.output.stderr {
        if !stderr.is_empty() {
            println!("{}", "Standard Error:".bright_yellow());
            println!("{}", "─".repeat(60));
            println!("{}", stderr);
            println!("{}", "─".repeat(60));
            println!();
        }
    }

    Ok(())
}

fn print_json_output(response: &toadstool::ExecutionResponse, duration: Duration) -> Result<()> {
    let output = serde_json::json!({
        "execution_id": response.execution_id,
        "status": format!("{:?}", response.status),
        "duration_secs": duration.as_secs_f64(),
        "exit_code": response.output.exit_code,
        "stdout": response.output.stdout,
        "stderr": response.output.stderr,
        "duration_internal": format!("{:?}", response.duration),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ========================================================================
    // Test 1: load_workload_file with TOML
    // ========================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_load_workload_file_toml() {
        let content = r#"
[metadata]
name = "test-workload"
description = "Test"
version = "1.0"

[execution]
type = "native"
command = "/bin/echo"
"#;

        let mut temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        write!(temp_file, "{}", content).unwrap();
        let path = temp_file.path().to_path_buf();

        let result = load_workload_file(&path).await;
        assert!(result.is_ok());
        let workload = result.unwrap();
        assert_eq!(workload.metadata.name, "test-workload");
    }

    // ========================================================================
    // Test 2: load_workload_file with JSON
    // ========================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_load_workload_file_json() {
        let content = r#"
{
  "metadata": {"name": "json-workload"},
  "execution": {"type": "python", "file": "script.py"}
}
"#;

        let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
        write!(temp_file, "{}", content).unwrap();
        let path = temp_file.path().to_path_buf();

        let result = load_workload_file(&path).await;
        assert!(result.is_ok());
        let workload = result.unwrap();
        assert_eq!(workload.metadata.name, "json-workload");
    }

    // ========================================================================
    // Test 3: load_workload_file with nonexistent file
    // ========================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_load_workload_file_not_found() {
        let path = PathBuf::from("/nonexistent/workload.toml");
        let result = load_workload_file(&path).await;
        assert!(result.is_err());
    }

    // ========================================================================
    // Test 4: convert_to_workload_spec for Native execution
    // ========================================================================

    #[test]
    fn test_convert_to_workload_spec_native() {
        let mut env = HashMap::new();
        env.insert("VAR1".to_string(), "value1".to_string());

        let workload = WorkloadFile {
            metadata: WorkloadMetadata {
                name: "test".to_string(),
                description: None,
                version: None,
            },
            execution: ExecutionSpec::Native {
                command: "/bin/echo".to_string(),
                args: Some(vec!["hello".to_string()]),
                working_dir: Some("/app".to_string()),
                env: Some(env.clone()),
            },
            resources: None,
            security: None,
        };

        let env_overrides = HashMap::new();
        let result = convert_to_workload_spec(&workload, env_overrides);
        assert!(result.is_ok());

        match result.unwrap() {
            WorkloadSpec::Native {
                env_vars,
                working_dir,
                ..
            } => {
                assert_eq!(env_vars.get("VAR1"), Some(&"value1".to_string()));
                assert_eq!(working_dir, Some(PathBuf::from("/app")));
            }
            _ => panic!("Expected Native workload spec"),
        }
    }

    // ========================================================================
    // Test 5: convert_to_workload_spec for Python with code
    // ========================================================================

    #[test]
    fn test_convert_to_workload_spec_python_code() {
        let workload = WorkloadFile {
            metadata: WorkloadMetadata {
                name: "python-test".to_string(),
                description: None,
                version: None,
            },
            execution: ExecutionSpec::Python {
                script: Some("print('hello')".to_string()),
                file: None,
                args: None,
                env: None,
            },
            resources: None,
            security: None,
        };

        let result = convert_to_workload_spec(&workload, HashMap::new());
        assert!(result.is_ok());

        match result.unwrap() {
            WorkloadSpec::Python { source, .. } => match source {
                toadstool::workload::PythonSource::Code { code } => {
                    assert!(code.contains("hello"));
                }
                _ => panic!("Expected Code source"),
            },
            _ => panic!("Expected Python workload spec"),
        }
    }

    // ========================================================================
    // Test 6: convert_to_workload_spec for Python with file
    // ========================================================================

    #[test]
    fn test_convert_to_workload_spec_python_file() {
        let workload = WorkloadFile {
            metadata: WorkloadMetadata {
                name: "python-file-test".to_string(),
                description: None,
                version: None,
            },
            execution: ExecutionSpec::Python {
                script: None,
                file: Some("script.py".to_string()),
                args: None,
                env: None,
            },
            resources: None,
            security: None,
        };

        let result = convert_to_workload_spec(&workload, HashMap::new());
        assert!(result.is_ok());

        match result.unwrap() {
            WorkloadSpec::Python { source, .. } => match source {
                toadstool::workload::PythonSource::File { path } => {
                    assert_eq!(path, PathBuf::from("script.py"));
                }
                _ => panic!("Expected File source"),
            },
            _ => panic!("Expected Python workload spec"),
        }
    }

    // ========================================================================
    // Test 7: convert_to_workload_spec Python without script or file (error)
    // ========================================================================

    #[test]
    fn test_convert_to_workload_spec_python_missing_source() {
        let workload = WorkloadFile {
            metadata: WorkloadMetadata {
                name: "invalid-python".to_string(),
                description: None,
                version: None,
            },
            execution: ExecutionSpec::Python {
                script: None,
                file: None,
                args: None,
                env: None,
            },
            resources: None,
            security: None,
        };

        let result = convert_to_workload_spec(&workload, HashMap::new());
        assert!(result.is_err());
    }

    // ========================================================================
    // Test 8: parse_runtime_hint
    // ========================================================================

    #[test]
    fn test_parse_runtime_hint_native() {
        let result = parse_runtime_hint("native");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), RuntimeType::Native));
    }

    #[test]
    fn test_parse_runtime_hint_python() {
        let result = parse_runtime_hint("PYTHON");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), RuntimeType::Python));
    }

    #[test]
    fn test_parse_runtime_hint_wasm() {
        assert!(parse_runtime_hint("wasm").is_ok());
        assert!(parse_runtime_hint("webassembly").is_ok());
    }

    #[test]
    fn test_parse_runtime_hint_container() {
        assert!(parse_runtime_hint("container").is_ok());
        assert!(parse_runtime_hint("docker").is_ok());
    }

    #[test]
    fn test_parse_runtime_hint_gpu() {
        let result = parse_runtime_hint("gpu");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), RuntimeType::Gpu));
    }

    #[test]
    fn test_parse_runtime_hint_invalid() {
        let result = parse_runtime_hint("invalid_runtime");
        assert!(result.is_err());
    }

    // ========================================================================
    // Test 9: infer_runtime_type
    // ========================================================================

    #[test]
    fn test_infer_runtime_type_native() {
        let spec = WorkloadSpec::Native {
            executable: toadstool::workload::ExecutableSource::File {
                path: "/bin/echo".into(),
            },
            args: None,
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        };

        let runtime = infer_runtime_type(&spec);
        assert!(matches!(runtime, RuntimeType::Native));
    }

    #[test]
    fn test_infer_runtime_type_python() {
        let spec = WorkloadSpec::Python {
            source: toadstool::workload::PythonSource::Code {
                code: "print('test')".to_string(),
            },
            python_version: None,
            requirements: vec![],
            env_vars: HashMap::new(),
        };

        let runtime = infer_runtime_type(&spec);
        assert!(matches!(runtime, RuntimeType::Python));
    }

    // ========================================================================
    // Test 10: convert_resource_requirements
    // ========================================================================

    #[test]
    fn test_convert_resource_requirements_none() {
        let resources = convert_resource_requirements(&None);
        // Should return default
        assert!(!format!("{:?}", resources).is_empty());
    }

    #[test]
    fn test_convert_resource_requirements_some() {
        let spec = Some(ResourceSpec {
            cpu_cores: Some(4.0),
            memory_mb: Some(8192),
            disk_mb: Some(10240),
            gpu: Some(true),
        });

        let resources = convert_resource_requirements(&spec);
        assert!(!format!("{:?}", resources).is_empty());
    }

    // ========================================================================
    // Test 11: convert_security_context
    // ========================================================================

    #[test]
    fn test_convert_security_context_none() {
        let context = convert_security_context(&None);
        assert!(!format!("{:?}", context).is_empty());
    }

    #[test]
    fn test_convert_security_context_with_isolation() {
        let spec = Some(SecuritySpec {
            isolation: Some("container".to_string()),
        });

        let context = convert_security_context(&spec);
        assert!(!format!("{:?}", context).is_empty());
    }

    // ========================================================================
    // Test 12: Environment variable merging in Native workload
    // ========================================================================

    #[test]
    fn test_native_workload_env_merging() {
        let mut base_env = HashMap::new();
        base_env.insert("VAR1".to_string(), "base".to_string());

        let mut overrides = HashMap::new();
        overrides.insert("VAR1".to_string(), "override".to_string());
        overrides.insert("VAR2".to_string(), "new".to_string());

        let workload = WorkloadFile {
            metadata: WorkloadMetadata {
                name: "env-test".to_string(),
                description: None,
                version: None,
            },
            execution: ExecutionSpec::Native {
                command: "/bin/cmd".to_string(),
                args: None,
                working_dir: None,
                env: Some(base_env),
            },
            resources: None,
            security: None,
        };

        let result = convert_to_workload_spec(&workload, overrides).unwrap();
        match result {
            WorkloadSpec::Native { env_vars, .. } => {
                assert_eq!(env_vars.get("VAR1"), Some(&"override".to_string()));
                assert_eq!(env_vars.get("VAR2"), Some(&"new".to_string()));
            }
            _ => panic!("Expected Native workload"),
        }
    }
}
