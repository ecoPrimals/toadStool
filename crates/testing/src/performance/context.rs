// SPDX-License-Identifier: AGPL-3.0-only
// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Performance benchmark runtime context and resource monitoring

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Context for a running performance benchmark
#[derive(Debug)]
pub struct BenchmarkContext {
    pub test_name: String,
    pub start_time: Instant,
    pub iteration_times: Vec<Duration>,
    pub resource_monitor: ResourceMonitor,
    pub custom_metrics: HashMap<String, Vec<f64>>,
}

impl BenchmarkContext {
    /// Create a new benchmark context
    #[must_use]
    pub fn new(test_name: impl Into<String>) -> Self {
        Self {
            test_name: test_name.into(),
            start_time: Instant::now(),
            iteration_times: Vec::new(),
            resource_monitor: ResourceMonitor::new(),
            custom_metrics: HashMap::new(),
        }
    }

    /// Record a custom metric value
    pub fn record_metric(&mut self, name: &str, value: f64) {
        self.custom_metrics
            .entry(name.to_string())
            .or_default()
            .push(value);
    }
}

/// Resource monitor for tracking system usage during benchmarks
#[derive(Debug)]
pub struct ResourceMonitor {
    pub memory_samples: Vec<u32>,
    pub cpu_samples: Vec<f32>,
    pub disk_io_samples: Vec<u64>,
    pub network_io_samples: Vec<u64>,
    pub start_time: Instant,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory_samples: Vec::new(),
            cpu_samples: Vec::new(),
            disk_io_samples: Vec::new(),
            network_io_samples: Vec::new(),
            start_time: Instant::now(),
        }
    }

    /// Sample current resource usage
    pub fn sample_resources(&mut self) {
        // Test infrastructure: Using deterministic sample values for reproducible tests
        // Production code would use toadstool-sysmon (pure Rust /proc parsing)
        self.memory_samples.push(100); // MB - Test value
        self.cpu_samples.push(50.0); // Percent - Test value
        self.disk_io_samples.push(1024); // Bytes - Test value
        self.network_io_samples.push(2048); // Bytes - Test value
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}
