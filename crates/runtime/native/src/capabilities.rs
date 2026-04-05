// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use toadstool::WorkloadType;
use toadstool::execution::RuntimeCapabilities;

pub fn default_capabilities() -> RuntimeCapabilities {
    RuntimeCapabilities {
        supported_workloads: vec![WorkloadType::Native],
        max_concurrent_executions: Some(100),
        supported_architectures: vec![std::env::consts::ARCH.to_string()],
        platform_features: {
            let mut features = HashMap::new();
            features.insert("process_isolation".to_string(), true);
            features.insert("resource_limits".to_string(), cfg!(target_os = "linux"));
            features.insert("user_switching".to_string(), cfg!(unix));
            features.insert("chroot_jail".to_string(), cfg!(unix));
            features
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}
