// SPDX-License-Identifier: AGPL-3.0-or-later
//! Container platforms
//!
//! Support for containerization technologies including Docker, Podman,
//! WebAssembly runtimes, unikernels, serverless, and orchestration platforms.

use serde::{Deserialize, Serialize};

/// Container platforms
///
/// Represents various containerization and isolation technologies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContainerPlatform {
    // Container runtimes
    /// Docker container runtime
    Docker {
        version: String,
        features: Vec<String>,
    },

    /// Podman container runtime
    Podman { version: String, rootless: bool },

    /// Containerd runtime
    Containerd {
        version: String,
        snapshotter: String,
    },

    /// CRI-O runtime
    CriO { version: String, runtime: String },

    // VM-based containers
    /// Firecracker microVM
    Firecracker { version: String, jailer: bool },

    /// Kata Containers
    Kata { version: String, hypervisor: String },

    /// gVisor sandboxed container runtime
    #[serde(rename = "gVisor")]
    GVisor { version: String, platform: String },

    // WebAssembly runtimes
    /// Wasmtime WebAssembly runtime
    Wasmtime {
        version: String,
        features: Vec<String>,
    },

    /// Wasmer WebAssembly runtime
    Wasmer {
        version: String,
        backends: Vec<String>,
    },

    /// WasmEdge runtime
    WasmEdge {
        version: String,
        extensions: Vec<String>,
    },

    // Unikernel platforms
    /// Unikernel platforms
    Unikernel { platform: String, language: String },

    // Serverless platforms
    /// AWS Lambda
    Lambda { runtime: String, memory_mb: u32 },

    /// Google Cloud Run
    CloudRun {
        runtime: String,
        cpu_allocation: String,
    },

    /// Azure Functions
    AzureFunctions {
        runtime: String,
        trigger_type: String,
    },

    // Orchestration platforms
    /// Kubernetes container orchestration
    Kubernetes {
        version: String,
        distribution: String,
    },

    /// Docker Swarm
    DockerSwarm {
        version: String,
        features: Vec<String>,
    },

    /// HashiCorp Nomad
    Nomad { version: String, driver: String },
}

impl ContainerPlatform {
    /// Get the platform type name
    pub const fn platform_type(&self) -> &'static str {
        match self {
            Self::Docker { .. } => "Docker",
            Self::Podman { .. } => "Podman",
            Self::Containerd { .. } => "Containerd",
            Self::CriO { .. } => "CRI-O",
            Self::Firecracker { .. } => "Firecracker",
            Self::Kata { .. } => "Kata Containers",
            Self::GVisor { .. } => "gVisor",
            Self::Wasmtime { .. } => "Wasmtime",
            Self::Wasmer { .. } => "Wasmer",
            Self::WasmEdge { .. } => "WasmEdge",
            Self::Unikernel { .. } => "Unikernel",
            Self::Lambda { .. } => "AWS Lambda",
            Self::CloudRun { .. } => "Google Cloud Run",
            Self::AzureFunctions { .. } => "Azure Functions",
            Self::Kubernetes { .. } => "Kubernetes",
            Self::DockerSwarm { .. } => "Docker Swarm",
            Self::Nomad { .. } => "Nomad",
        }
    }

    /// Check if platform is a traditional container runtime
    pub const fn is_traditional_container(&self) -> bool {
        matches!(
            self,
            Self::Docker { .. } | Self::Podman { .. } | Self::Containerd { .. } | Self::CriO { .. }
        )
    }

    /// Check if platform is a WebAssembly runtime
    pub const fn is_wasm_runtime(&self) -> bool {
        matches!(
            self,
            Self::Wasmtime { .. } | Self::Wasmer { .. } | Self::WasmEdge { .. }
        )
    }

    /// Check if platform is VM-based for enhanced isolation
    pub const fn is_vm_based(&self) -> bool {
        matches!(
            self,
            Self::Firecracker { .. } | Self::Kata { .. } | Self::GVisor { .. }
        )
    }

    /// Check if platform is serverless
    pub const fn is_serverless(&self) -> bool {
        matches!(
            self,
            Self::Lambda { .. } | Self::CloudRun { .. } | Self::AzureFunctions { .. }
        )
    }

    /// Check if platform is an orchestrator
    pub const fn is_orchestrator(&self) -> bool {
        matches!(
            self,
            Self::Kubernetes { .. } | Self::DockerSwarm { .. } | Self::Nomad { .. }
        )
    }

    /// Check if platform supports rootless operation
    pub const fn supports_rootless(&self) -> bool {
        match self {
            Self::Podman { rootless, .. } => *rootless,
            _ => false,
        }
    }

    /// Get version string
    pub fn version(&self) -> &str {
        match self {
            Self::Docker { version, .. }
            | Self::Podman { version, .. }
            | Self::Containerd { version, .. }
            | Self::CriO { version, .. }
            | Self::Firecracker { version, .. }
            | Self::Kata { version, .. }
            | Self::GVisor { version, .. }
            | Self::Wasmtime { version, .. }
            | Self::Wasmer { version, .. }
            | Self::WasmEdge { version, .. }
            | Self::Kubernetes { version, .. }
            | Self::DockerSwarm { version, .. }
            | Self::Nomad { version, .. } => version,
            Self::Unikernel { platform, .. } => platform,
            Self::Lambda { runtime, .. }
            | Self::CloudRun { runtime, .. }
            | Self::AzureFunctions { runtime, .. } => runtime,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traditional_container() {
        let docker = ContainerPlatform::Docker {
            version: "24.0.0".to_string(),
            features: vec!["buildkit".to_string()],
        };

        assert_eq!(docker.platform_type(), "Docker");
        assert!(docker.is_traditional_container());
        assert!(!docker.is_wasm_runtime());
    }

    #[test]
    fn test_wasm_runtime() {
        let wasmtime = ContainerPlatform::Wasmtime {
            version: "14.0.0".to_string(),
            features: vec!["async".to_string(), "component-model".to_string()],
        };

        assert!(wasmtime.is_wasm_runtime());
        assert!(!wasmtime.is_traditional_container());
    }

    #[test]
    fn test_vm_based() {
        let kata = ContainerPlatform::Kata {
            version: "3.0.0".to_string(),
            hypervisor: "QEMU".to_string(),
        };

        assert!(kata.is_vm_based());
        assert!(!kata.is_wasm_runtime());
    }

    #[test]
    fn test_serverless() {
        let lambda = ContainerPlatform::Lambda {
            runtime: "nodejs18.x".to_string(),
            memory_mb: 512,
        };

        assert!(lambda.is_serverless());
        assert!(!lambda.is_orchestrator());
    }

    #[test]
    fn test_orchestrator() {
        let k8s = ContainerPlatform::Kubernetes {
            version: "1.28.0".to_string(),
            distribution: "vanilla".to_string(),
        };

        assert!(k8s.is_orchestrator());
        assert!(!k8s.is_serverless());
    }

    #[test]
    fn test_rootless_support() {
        let podman_rootless = ContainerPlatform::Podman {
            version: "4.7.0".to_string(),
            rootless: true,
        };

        let podman_root = ContainerPlatform::Podman {
            version: "4.7.0".to_string(),
            rootless: false,
        };

        assert!(podman_rootless.supports_rootless());
        assert!(!podman_root.supports_rootless());
    }

    #[test]
    fn test_serialization() {
        let platform = ContainerPlatform::Wasmtime {
            version: "14.0.0".to_string(),
            features: vec!["async".to_string()],
        };

        let json = serde_json::to_string(&platform).unwrap();
        let deserialized: ContainerPlatform = serde_json::from_str(&json).unwrap();

        assert_eq!(platform, deserialized);
    }
}
