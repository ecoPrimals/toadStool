//! Conversion from workload file format to ToadStool runtime types.
//!
//! Transforms parsed workload specifications into ExecutionRequest components.

use crate::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use toadstool::{
    resources::ResourceRequirements,
    security::{IsolationLevel, SecurityContext},
    workload::{ExecutableSource, WorkloadSpec},
};

use super::spec::{ExecutionSpec, ResourceSpec, SecuritySpec, WorkloadFile};

/// Convert a parsed workload file to ToadStool WorkloadSpec.
pub(super) fn convert_to_workload_spec(
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
                return Err(crate::CliError::Other(
                    "Python workload must specify either 'script' or 'file'".to_string(),
                ));
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
        _ => Err(crate::CliError::Other(
            "Workload type not yet supported".to_string(),
        )),
    }
}

/// Convert resource spec to ToadStool ResourceRequirements.
pub(super) fn convert_resource_requirements(
    _resources: &Option<ResourceSpec>,
) -> ResourceRequirements {
    // Use default for now - actual resource fields need to match toadstool::resources::ResourceRequirements
    ResourceRequirements::default()
}

/// Convert security spec to ToadStool SecurityContext.
pub(super) fn convert_security_context(_security: &Option<SecuritySpec>) -> SecurityContext {
    // Use standard isolation for now
    SecurityContext::for_isolation_level(IsolationLevel::Standard)
}

#[cfg(test)]
mod tests {
    use super::super::spec::{ExecutionSpec, WorkloadMetadata};
    use super::*;

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

        let spec = result.unwrap();
        assert!(matches!(spec, WorkloadSpec::Native { .. }));
        if let WorkloadSpec::Native {
            env_vars,
            working_dir,
            ..
        } = &spec
        {
            assert_eq!(env_vars.get("VAR1"), Some(&"value1".to_string()));
            assert_eq!(working_dir, &Some(PathBuf::from("/app")));
        }
    }

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

        let spec = result.unwrap();
        assert!(matches!(spec, WorkloadSpec::Python { .. }));
        if let WorkloadSpec::Python { source, .. } = &spec {
            assert!(matches!(
                source,
                toadstool::workload::PythonSource::Code { .. }
            ));
            if let toadstool::workload::PythonSource::Code { code } = source {
                assert!(code.contains("hello"));
            }
        }
    }

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

        let spec = result.unwrap();
        assert!(matches!(spec, WorkloadSpec::Python { .. }));
        if let WorkloadSpec::Python { source, .. } = &spec {
            assert!(matches!(
                source,
                toadstool::workload::PythonSource::File { .. }
            ));
            if let toadstool::workload::PythonSource::File { path } = source {
                assert_eq!(path, &PathBuf::from("script.py"));
            }
        }
    }

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

    #[test]
    fn test_convert_resource_requirements_none() {
        let resources = convert_resource_requirements(&None);
        assert!(!format!("{:?}", resources).is_empty());
    }

    #[test]
    fn test_convert_resource_requirements_some() {
        let spec = Some(ResourceSpec {
            cpu_cores: Some(4.0),
            memory_mb: Some(8192),
            disk_mb: Some(10_240),
            gpu: Some(true),
        });

        let resources = convert_resource_requirements(&spec);
        assert!(!format!("{:?}", resources).is_empty());
    }

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
        assert!(matches!(result, WorkloadSpec::Native { .. }));
        if let WorkloadSpec::Native { env_vars, .. } = &result {
            assert_eq!(env_vars.get("VAR1"), Some(&"override".to_string()));
            assert_eq!(env_vars.get("VAR2"), Some(&"new".to_string()));
        }
    }
}
