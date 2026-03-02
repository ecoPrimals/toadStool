//! Resource limits and execution configuration overrides.

use super::super::ConfigResult;
use super::parse;
use crate::ToadStoolConfig;
use std::time::Duration;

pub(super) fn apply(config: &mut ToadStoolConfig) -> ConfigResult<()> {
    if let Ok(max_cpu) = std::env::var("TOADSTOOL_MAX_CPU") {
        config.runtime.resource_limits.max_cpu_usage = parse::parse_f64(&max_cpu, "max CPU")?;
    }

    if let Ok(max_memory) = std::env::var("TOADSTOOL_MAX_MEMORY") {
        config.runtime.resource_limits.max_memory_usage =
            parse::parse_f64(&max_memory, "max memory")?;
    }

    if let Ok(max_concurrent) = std::env::var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS") {
        config.runtime.max_concurrent_executions =
            parse::parse_u32(&max_concurrent, "max concurrent executions")?;
    }

    if let Ok(timeout) = std::env::var("TOADSTOOL_EXECUTION_TIMEOUT") {
        let timeout_secs = parse::parse_u64(&timeout, "execution timeout")?;
        config.runtime.execution_timeout = Duration::from_secs(timeout_secs);
    }

    Ok(())
}
