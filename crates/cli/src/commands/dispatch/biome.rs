// SPDX-License-Identifier: AGPL-3.0-only
//! Biome lifecycle command handlers
//!
//! Run, Up, Down, Ps, Logs - foreground/background execution and monitoring.

use std::path::PathBuf;

use tracing::info;

use crate::{CliContext, Result};

use crate::executor::{BiomeExecutor, RunBiomeOptions, UpBiomeOptions};

/// Execute `run` - foreground biome
pub async fn execute_run(ctx: &CliContext, opts: RunBiomeOptions) -> Result<()> {
    info!("🚀 Starting biome in foreground mode");
    let executor = BiomeExecutor::new().await?;
    executor.run_biome(ctx, opts).await
}

/// Execute `up` - background biome
pub async fn execute_up(
    ctx: &CliContext,
    manifest: PathBuf,
    detach: bool,
    name: Option<String>,
    env: Vec<String>,
    restart: bool,
    health_interval: u64,
) -> Result<()> {
    info!("🚀 Starting biome in background mode");
    let executor = BiomeExecutor::new().await?;
    let opts = UpBiomeOptions {
        manifest_path: manifest,
        detach,
        name,
        env,
        restart,
        health_interval,
    };
    executor.up_biome(ctx, opts).await
}

/// Execute `down` - stop biome
pub async fn execute_down(biome: &str, force: bool, timeout: u64, purge: bool) -> Result<()> {
    info!("🛑 Stopping biome: {}", biome);
    let executor = BiomeExecutor::new().await?;
    executor.down_biome(biome, force, timeout, purge).await
}

/// Execute `ps` - list biomes
pub async fn execute_ps(
    all: bool,
    format: &str,
    resources: bool,
    status: Option<&str>,
) -> Result<()> {
    info!("📋 Listing biomes");
    let executor = BiomeExecutor::new().await?;
    executor.list_biomes(all, format, resources, status).await
}

/// Execute `logs` - show logs
pub async fn execute_logs(
    target: &str,
    follow: bool,
    lines: usize,
    timestamps: bool,
    level: Option<&str>,
    grep: Option<&str>,
) -> Result<()> {
    info!("📜 Showing logs for: {}", target);
    let executor = BiomeExecutor::new().await?;
    executor
        .show_logs(target, follow, lines, timestamps, level, grep)
        .await
}
