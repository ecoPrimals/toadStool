// SPDX-License-Identifier: AGPL-3.0-or-later
//! Key/value metadata maps for [`PlatformType`] (`Arc<str>` values = zero-copy clone).

use std::collections::HashMap;
use std::sync::Arc;
use toadstool_distributed::substrate_detection::PlatformType;

/// Build a metadata map describing the given platform.
pub(crate) fn from_platform(platform: &PlatformType) -> HashMap<String, Arc<str>> {
    let mut metadata = HashMap::new();

    match platform {
        PlatformType::Linux {
            distribution,
            architecture,
        } => {
            metadata.insert("type".to_string(), Arc::from("linux"));
            metadata.insert("distribution".to_string(), Arc::from(distribution.as_str()));
            metadata.insert("architecture".to_string(), Arc::from(architecture.as_str()));
        }
        PlatformType::MacOS {
            version,
            architecture,
        } => {
            metadata.insert("type".to_string(), Arc::from("macos"));
            metadata.insert("version".to_string(), Arc::from(version.as_str()));
            metadata.insert("architecture".to_string(), Arc::from(architecture.as_str()));
        }
        PlatformType::Windows {
            version,
            architecture,
        } => {
            metadata.insert("type".to_string(), Arc::from("windows"));
            metadata.insert("version".to_string(), Arc::from(version.as_str()));
            metadata.insert("architecture".to_string(), Arc::from(architecture.as_str()));
        }
        PlatformType::Docker => {
            metadata.insert("type".to_string(), Arc::from("container"));
            metadata.insert("runtime".to_string(), Arc::from("docker"));
        }
        PlatformType::Podman => {
            metadata.insert("type".to_string(), Arc::from("container"));
            metadata.insert("runtime".to_string(), Arc::from("podman"));
        }
        PlatformType::Containerd => {
            metadata.insert("type".to_string(), Arc::from("container"));
            metadata.insert("runtime".to_string(), Arc::from("containerd"));
        }
        PlatformType::WebAssembly { runtime } => {
            metadata.insert("type".to_string(), Arc::from("wasm"));
            metadata.insert("runtime".to_string(), Arc::from(runtime.as_str()));
        }
        PlatformType::Language { name, command } => {
            metadata.insert("type".to_string(), Arc::from("language"));
            metadata.insert("name".to_string(), Arc::from(name.as_str()));
            metadata.insert("command".to_string(), Arc::from(command.as_str()));
        }
        PlatformType::GPU { vendor, framework } => {
            metadata.insert("type".to_string(), Arc::from("gpu"));
            metadata.insert("vendor".to_string(), Arc::from(vendor.as_str()));
            metadata.insert("framework".to_string(), Arc::from(framework.as_str()));
        }
        PlatformType::Other { os, architecture } => {
            metadata.insert("type".to_string(), Arc::from("other"));
            metadata.insert("os".to_string(), Arc::from(os.as_str()));
            metadata.insert("architecture".to_string(), Arc::from(architecture.as_str()));
        }
        PlatformType::EdgeDevice {
            device_type,
            architecture,
        } => {
            metadata.insert("type".to_string(), Arc::from("edge_device"));
            metadata.insert("device_type".to_string(), Arc::from(device_type.as_str()));
            metadata.insert("architecture".to_string(), Arc::from(architecture.as_str()));
        }
        PlatformType::MCUDevelopment { platform, tool } => {
            metadata.insert("type".to_string(), Arc::from("mcu_development"));
            metadata.insert("platform".to_string(), Arc::from(platform.as_str()));
            metadata.insert("tool".to_string(), Arc::from(tool.as_str()));
        }
        PlatformType::BiologicalComputing {
            platform,
            simulation,
        } => {
            metadata.insert("type".to_string(), Arc::from("biological"));
            metadata.insert("platform".to_string(), Arc::from(platform.as_str()));
            metadata.insert(
                "simulation".to_string(),
                Arc::from(if *simulation { "true" } else { "false" }),
            );
        }
        PlatformType::Quantum {
            framework,
            simulator,
        } => {
            metadata.insert("type".to_string(), Arc::from("quantum"));
            metadata.insert("framework".to_string(), Arc::from(framework.as_str()));
            metadata.insert(
                "simulator".to_string(),
                Arc::from(if *simulator { "true" } else { "false" }),
            );
        }
        PlatformType::NeuromorphicComputing { platform, hardware } => {
            metadata.insert("type".to_string(), Arc::from("neuromorphic"));
            metadata.insert("platform".to_string(), Arc::from(platform.as_str()));
            metadata.insert(
                "hardware".to_string(),
                Arc::from(if *hardware { "true" } else { "false" }),
            );
        }
    }

    metadata
}
