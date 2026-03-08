// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::runtime_defaults::{ConfigError, ConfigResult};
use crate::ToadStoolConfig;

pub(super) fn validate(config: &ToadStoolConfig) -> ConfigResult<()> {
    if config.runtime.resource_limits.max_cpu_usage <= 0.0
        || config.runtime.resource_limits.max_cpu_usage > 100.0
    {
        return Err(ConfigError::Invalid(
            "Max CPU usage must be between 0 and 100".to_string(),
        ));
    }

    if config.runtime.resource_limits.max_memory_usage <= 0.0
        || config.runtime.resource_limits.max_memory_usage > 100.0
    {
        return Err(ConfigError::Invalid(
            "Max memory usage must be between 0 and 100".to_string(),
        ));
    }

    if config.runtime.resource_limits.max_disk_usage <= 0.0
        || config.runtime.resource_limits.max_disk_usage > 100.0
    {
        return Err(ConfigError::Invalid(
            "Max disk usage must be between 0 and 100".to_string(),
        ));
    }

    Ok(())
}
