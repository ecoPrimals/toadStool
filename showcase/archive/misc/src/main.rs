// ToadStool Showcase - REAL Execution using ToadStool Core
// This demonstrates ToadStool's actual runtime execution WITHOUT biome.yaml

use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use toadstool::{
    execution::{ExecutionRequest, RuntimeType},
    init,
    resources::ResourceRequirements,
    runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy},
    security::{IsolationLevel, SecurityContext},
    workload::{ExecutableSource, WorkloadSpec},
};
use toadstool_runtime_native::NativeRuntimeEngine;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize ToadStool
    init()?;

    println!();
    println!(
        "{}",
        "╔════════════════════════════════════════════════════════════╗".bright_cyan()
    );
    println!(
        "{}",
        "║    🍄 ToadStool Showcase - REAL Runtime Execution        ║".bright_cyan()
    );
    println!(
        "{}",
        "╚════════════════════════════════════════════════════════════╝".bright_cyan()
    );
    println!();

    println!(
        "{}",
        "This uses ToadStool's ACTUAL runtime engine.".bright_blue()
    );
    println!(
        "{}",
        "No simulation. No mock. Real execution.".bright_blue()
    );
    println!();

    // Create runtime orchestrator
    let orchestrator = RuntimeOrchestrator::new(RuntimeSelectionStrategy::FirstAvailable);

    // Register native runtime engine
    let native_engine = NativeRuntimeEngine::new();
    orchestrator
        .register_engine(RuntimeType::Native, Box::new(native_engine))
        .await?;

    println!("{}", "✅ Native runtime engine registered".bright_green());
    println!();

    // Demo 1: Hello World on Native
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_yellow()
    );
    println!(
        "{}",
        "Demo 1: Hello World on Native Substrate".bright_yellow()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_yellow()
    );
    println!();

    // Create workload spec for echo command
    let workload = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: "/bin/bash".into(),
        },
        args: Some(vec![
            "-c".to_string(),
            r#"
echo "╔════════════════════════════════════════════════════════════╗"
echo "║        🍄 ToadStool Universal Hello (NATIVE)             ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Execution Context:"
echo "  Substrate:    NATIVE"
echo "  Hostname:     $(hostname)"
echo "  Platform:     $(uname -s) $(uname -m)"
echo "  Process ID:   $$"
echo "  Timestamp:    $(date -Iseconds)"
echo ""
echo "✅ Hello World executed by ToadStool's REAL runtime engine"
echo ""
"#
            .to_string(),
        ]),
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };

    // Create execution request
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload,
        runtime_hint: Some(RuntimeType::Native),
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::for_isolation_level(IsolationLevel::Standard),
        timeout: Some(Duration::from_secs(30)),
        environment: HashMap::new(),
        input_data: Default::default(),
        callback_config: None,
        encryption_config: None,
    };

    println!("{}", "Executing workload...".bright_cyan());
    println!();

    // EXECUTE!
    let response = orchestrator.execute(request).await?;

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_green()
    );
    println!("{}", "✅ Execution Complete!".bright_green());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_green()
    );
    println!();
    println!("Execution ID:  {}", response.execution_id);
    println!("Status:        {:?}", response.status);
    println!("Duration:      {:?}", response.duration);
    println!("Exit Code:     {:?}", response.output.exit_code);
    println!();

    if let Some(stdout) = &response.output.stdout {
        println!("{}", "Standard Output:".bright_blue());
        println!("{}", stdout);
    }

    if let Some(stderr) = &response.output.stderr {
        if !stderr.is_empty() {
            println!("{}", "Standard Error:".bright_red());
            println!("{}", stderr);
        }
    }

    println!();
    println!(
        "{}",
        "🎉 This was ToadStool's REAL runtime in action!"
            .bright_magenta()
            .bold()
    );
    println!(
        "{}",
        "   • RuntimeOrchestrator selected the engine".bright_white()
    );
    println!(
        "{}",
        "   • NativeRuntimeEngine executed the workload".bright_white()
    );
    println!("{}", "   • No biome.yaml required!".bright_white());
    println!();

    Ok(())
}
