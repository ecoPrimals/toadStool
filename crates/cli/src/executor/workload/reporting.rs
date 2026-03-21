// SPDX-License-Identifier: AGPL-3.0-only
//! Execution result reporting and output formatting.
//!
//! Formats execution results for human-readable (text) or machine-readable (JSON) output.

use crate::Result;
use colored::Colorize;
use std::time::Duration;

use super::spec::WorkloadFile;

/// Print execution results in human-readable text format.
pub(super) fn print_text_output(
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
            format!("{exit_code}").bright_green()
        } else {
            format!("{exit_code}").bright_red()
        };
        println!("Exit Code:     {status_str}");
    }

    println!();

    if let Some(stdout) = &response.output.stdout {
        if !stdout.is_empty() {
            println!("{}", "Standard Output:".bright_blue());
            println!("{}", "─".repeat(60));
            println!("{stdout}");
            println!("{}", "─".repeat(60));
            println!();
        }
    }

    if let Some(stderr) = &response.output.stderr {
        if !stderr.is_empty() {
            println!("{}", "Standard Error:".bright_yellow());
            println!("{}", "─".repeat(60));
            println!("{stderr}");
            println!("{}", "─".repeat(60));
            println!();
        }
    }

    Ok(())
}

/// Print execution results in JSON format.
pub(super) fn print_json_output(
    response: &toadstool::ExecutionResponse,
    duration: Duration,
) -> Result<()> {
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
    use crate::executor::workload::spec::{ExecutionSpec, WorkloadMetadata};
    use std::time::Duration;
    use toadstool::execution::{ExecutionOutput, ExecutionStatus, RuntimeType};
    use uuid::Uuid;

    fn make_workload_file() -> WorkloadFile {
        WorkloadFile {
            metadata: WorkloadMetadata {
                name: "test-workload".to_string(),
                description: Some("Test workload for unit tests".to_string()),
                version: Some("1.0.0".to_string()),
            },
            execution: ExecutionSpec::Native {
                command: "/usr/bin/true".to_string(),
                args: None,
                working_dir: None,
                env: None,
            },
            resources: None,
            security: None,
        }
    }

    fn make_execution_response_success() -> toadstool::ExecutionResponse {
        toadstool::ExecutionResponse {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Success,
            output: ExecutionOutput {
                exit_code: Some(0),
                stdout: Some("Hello from stdout".to_string()),
                stderr: Some("Hello from stderr".to_string()),
                ..Default::default()
            },
            duration: Duration::from_secs(1),
            runtime_used: RuntimeType::Native,
            ..Default::default()
        }
    }

    fn make_execution_response_failed() -> toadstool::ExecutionResponse {
        toadstool::ExecutionResponse {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Failed {
                error: std::borrow::Cow::Borrowed("Test error"),
            },
            output: ExecutionOutput {
                exit_code: Some(1),
                stdout: None,
                stderr: Some("Error output".to_string()),
                ..Default::default()
            },
            duration: Duration::from_millis(500),
            runtime_used: RuntimeType::Native,
            ..Default::default()
        }
    }

    fn make_execution_response_minimal() -> toadstool::ExecutionResponse {
        toadstool::ExecutionResponse {
            execution_id: Uuid::new_v4(),
            status: ExecutionStatus::Success,
            output: ExecutionOutput::default(),
            duration: Duration::ZERO,
            runtime_used: RuntimeType::Native,
            ..Default::default()
        }
    }

    #[test]
    fn test_print_text_output_success() {
        let workload = make_workload_file();
        let response = make_execution_response_success();
        let duration = Duration::from_secs_f64(1.234);
        let result = print_text_output(&workload, &response, duration);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_text_output_failed() {
        let workload = make_workload_file();
        let response = make_execution_response_failed();
        let duration = Duration::from_secs_f64(0.5);
        let result = print_text_output(&workload, &response, duration);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_text_output_minimal() {
        let workload = make_workload_file();
        let response = make_execution_response_minimal();
        let duration = Duration::from_secs_f64(0.001);
        let result = print_text_output(&workload, &response, duration);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_text_output_empty_stdout_stderr() {
        let mut response = make_execution_response_success();
        response.output.stdout = Some(String::new());
        response.output.stderr = Some(String::new());
        // Empty stdout/stderr are skipped by print_text_output (no output blocks)
        let workload = make_workload_file();
        let result = print_text_output(&workload, &response, Duration::from_secs(1));
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_json_output_success() {
        let response = make_execution_response_success();
        let duration = Duration::from_secs_f64(1.5);
        let result = print_json_output(&response, duration);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_json_output_failed() {
        let response = make_execution_response_failed();
        let duration = Duration::from_millis(100);
        let result = print_json_output(&response, duration);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_json_output_minimal() {
        let response = make_execution_response_minimal();
        let duration = Duration::ZERO;
        let result = print_json_output(&response, duration);
        assert!(result.is_ok());
    }
}
