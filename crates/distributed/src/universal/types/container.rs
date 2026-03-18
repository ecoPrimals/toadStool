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
    /// Docker container runtime.
    Docker {
        /// Docker version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Podman container runtime.
    Podman {
        /// Podman version.
        version: String,
        /// Rootless mode.
        rootless: bool,
    },
    /// Containerd runtime.
    Containerd {
        /// Containerd version.
        version: String,
        /// Snapshotter (overlayfs, etc.).
        snapshotter: String,
    },
    /// CRI-O runtime.
    CriO {
        /// CRI-O version.
        version: String,
        /// Runtime (runc, crun, etc.).
        runtime: String,
    },
    /// Firecracker microVM.
    Firecracker {
        /// Firecracker version.
        version: String,
        /// Jailer enabled.
        jailer: bool,
    },
    /// Kata Containers.
    Kata {
        /// Kata version.
        version: String,
        /// Hypervisor (QEMU, Cloud Hypervisor, etc.).
        hypervisor: String,
    },
    /// gVisor sandboxed container runtime.
    #[serde(rename = "gVisor")]
    GVisor {
        /// gVisor version.
        version: String,
        /// Platform (linux/amd64, etc.).
        platform: String,
    },
    /// Wasmtime WebAssembly runtime.
    Wasmtime {
        /// Wasmtime version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Wasmer WebAssembly runtime.
    Wasmer {
        /// Wasmer version.
        version: String,
        /// Backends (cranelift, llvm, etc.).
        backends: Vec<String>,
    },
    /// WasmEdge runtime.
    WasmEdge {
        /// WasmEdge version.
        version: String,
        /// Extensions.
        extensions: Vec<String>,
    },
    /// Unikernel platforms.
    Unikernel {
        /// Platform name.
        platform: String,
        /// Language (ocaml, rust, etc.).
        language: String,
    },
    /// AWS Lambda.
    Lambda {
        /// Runtime (nodejs18.x, python3.11, etc.).
        runtime: String,
        /// Memory in MB.
        memory_mb: u32,
    },
    /// Google Cloud Run.
    CloudRun {
        /// Runtime.
        runtime: String,
        /// CPU allocation (request, limit).
        cpu_allocation: String,
    },
    /// Azure Functions.
    AzureFunctions {
        /// Runtime.
        runtime: String,
        /// Trigger type (http, timer, etc.).
        trigger_type: String,
    },
    /// Kubernetes container orchestration.
    Kubernetes {
        /// Kubernetes version.
        version: String,
        /// Distribution (vanilla, openshift, etc.).
        distribution: String,
    },
    /// Docker Swarm.
    DockerSwarm {
        /// Swarm version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// HashiCorp Nomad.
    Nomad {
        /// Nomad version.
        version: String,
        /// Driver (docker, exec, etc.).
        driver: String,
    },
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
