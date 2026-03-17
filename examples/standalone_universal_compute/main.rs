// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::nursery,
    clippy::cast_precision_loss,
    clippy::struct_excessive_bools,
    clippy::unused_async,
    dead_code,
    unused_variables
)]
//! Standalone Universal Compute Demonstration
//! ToadStool's Open-First Strategy: Isolate → Abstract → Incentivize

mod ecosystem;
mod scheduling;
mod types;
mod workload_demos;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🍄 ToadStool Universal Compute Demo: Breaking the CUDA Monopoly");
    println!("===============================================================");
    println!("Strategy: Isolate → Abstract → Incentivize");
    println!();

    let ecosystem = ecosystem::create_diverse_ecosystem();
    ecosystem::print_ecosystem_overview(&ecosystem);

    workload_demos::demonstrate_ai_workloads(&ecosystem);
    workload_demos::demonstrate_general_compute(&ecosystem);
    workload_demos::demonstrate_cuda_isolation(&ecosystem);

    workload_demos::show_strategic_metrics(&ecosystem);

    println!("\n🎯 Mission Accomplished!");
    println!("✅ Open standards prioritized over proprietary lock-in");
    println!("✅ Cross-platform compatibility maximized");
    println!("✅ Community frameworks championed");
    println!("✅ NVIDIA incentivized to join the open ecosystem");
    println!("\n💡 The message to NVIDIA: 'Your hardware is amazing. Your drivers could power the world.'");

    Ok(())
}
