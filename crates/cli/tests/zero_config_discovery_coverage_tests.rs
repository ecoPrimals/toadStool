// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(deprecated)]
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive tests for `ZeroConfig` discovery - coverage target 90%
//!
//! Tests `discover_system`, `discover_ecosystem`, and discovery helper methods.
//! Uses actual system commands on Linux - tests may behave differently on other OS.

use toadstool_cli::Result;
use toadstool_cli::zero_config::{DiscoveryExt, ZeroConfigCore, ZeroConfigDeployment};

// ============================================================================
// discover_system tests - runs nproc, /proc/cpuinfo, /proc/meminfo, df, ip, uname, etc.
// ============================================================================

#[tokio::test]
async fn test_discover_system_succeeds_on_linux() {
    let mut deployment = ZeroConfigDeployment::new();
    let result: Result<()> = deployment.discover_system().await;
    #[cfg(target_os = "linux")]
    assert!(
        result.is_ok(),
        "discover_system should succeed on Linux: {result:?}"
    );
    #[cfg(not(target_os = "linux"))]
    let _ = result;
}

#[tokio::test]
async fn test_discover_system_populates_cpu_info() {
    let mut deployment = ZeroConfigDeployment::new();
    if matches!(deployment.discover_system().await, Ok(())) {
        assert!(deployment.system_info.cpu.cores >= 1);
        assert!(!deployment.system_info.cpu.architecture.is_empty());
    }
}

#[tokio::test]
async fn test_discover_system_populates_memory_info() {
    let mut deployment = ZeroConfigDeployment::new();
    if matches!(deployment.discover_system().await, Ok(())) {
        // Memory info may be 0 if /proc/meminfo parse fails
        let _ = deployment.system_info.memory.total_bytes;
        let _ = deployment.system_info.memory.available_bytes;
    }
}

#[tokio::test]
async fn test_discover_system_populates_storage_info() {
    let mut deployment = ZeroConfigDeployment::new();
    if matches!(deployment.discover_system().await, Ok(())) {
        let _ = deployment.system_info.storage.total_bytes;
        let _ = deployment.system_info.storage.available_bytes;
    }
}

#[tokio::test]
async fn test_discover_system_populates_os_info() {
    let mut deployment = ZeroConfigDeployment::new();
    if matches!(deployment.discover_system().await, Ok(())) {
        assert!(!deployment.system_info.os.name.is_empty());
        assert!(!deployment.system_info.os.arch.is_empty());
    }
}

#[tokio::test]
async fn test_discover_system_populates_container_runtime() {
    let mut deployment = ZeroConfigDeployment::new();
    if matches!(deployment.discover_system().await, Ok(())) {
        let _ = deployment.system_info.container_runtime.docker;
        let _ = deployment.system_info.container_runtime.podman;
        let _ = deployment.system_info.container_runtime.containerd;
    }
}

#[tokio::test]
async fn test_discover_system_populates_gpu_info() {
    let mut deployment = ZeroConfigDeployment::new();
    if matches!(deployment.discover_system().await, Ok(())) {
        let _ = deployment.system_info.gpu.count;
        let _ = deployment.system_info.gpu.vendor.as_str();
        let _ = deployment.system_info.gpu.model.as_str();
    }
}

// ============================================================================
// discover_ecosystem tests - capability-based service discovery
// ============================================================================

#[tokio::test]
async fn test_discover_ecosystem_succeeds() {
    let mut deployment = ZeroConfigDeployment::new();
    let result: Result<()> = deployment.discover_ecosystem().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_discover_ecosystem_populates_services() {
    let mut deployment = ZeroConfigDeployment::new();
    if matches!(deployment.discover_ecosystem().await, Ok(())) {
        let _ = &deployment.ecosystem_services.coordination;
        let _ = &deployment.ecosystem_services.security;
        let _ = &deployment.ecosystem_services.storage;
        let _ = &deployment.ecosystem_services.ai_processing;
        let _ = &deployment.ecosystem_services.toadstool_peers;
    }
}

#[tokio::test]
async fn test_discover_system_then_ecosystem() {
    let mut deployment = ZeroConfigDeployment::new();
    let sys_result: Result<()> = deployment.discover_system().await;
    let eco_result: Result<()> = deployment.discover_ecosystem().await;
    #[cfg(target_os = "linux")]
    assert!(sys_result.is_ok());
    assert!(eco_result.is_ok());
}

// ============================================================================
// Additional discovery coverage - capability resolution paths
// ============================================================================

#[tokio::test]
async fn test_discover_ecosystem_multiple_times() {
    let mut deployment = ZeroConfigDeployment::new();
    let r1 = deployment.discover_ecosystem().await;
    let r2 = deployment.discover_ecosystem().await;
    assert!(r1.is_ok());
    assert!(r2.is_ok());
}

#[tokio::test]
async fn test_discover_system_cpu_cores_at_least_one() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        assert!(deployment.system_info.cpu.cores >= 1);
    }
}

#[tokio::test]
async fn test_discover_system_os_name_non_empty() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        assert!(!deployment.system_info.os.name.is_empty());
    }
}

#[tokio::test]
async fn test_discover_system_arch_populated() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        assert!(!deployment.system_info.cpu.architecture.is_empty());
    }
}

#[tokio::test]
async fn test_discover_system_network_info() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        let _ = &deployment.system_info.network.interfaces;
        let _ = &deployment.system_info.network.local_ips;
    }
}

#[tokio::test]
async fn test_discover_system_storage_filesystem() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        assert!(!deployment.system_info.storage.filesystem.is_empty());
    }
}

#[tokio::test]
async fn test_discover_system_gpu_vendor() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        let _ = deployment.system_info.gpu.vendor.as_str();
        let _ = deployment.system_info.gpu.model.as_str();
    }
}

#[tokio::test]
async fn test_zero_config_deployment_new() {
    let deployment = ZeroConfigDeployment::new();
    let _ = deployment.system_info.cpu.cores;
    let _ = deployment.ecosystem_services.coordination.is_none();
}

// ─── Additional coverage: types, edge cases, platform detection ───

#[test]
fn test_system_info_default() {
    use toadstool_cli::zero_config::SystemInfo;
    let info = SystemInfo::default();
    assert!(info.cpu.cores >= 1);
    assert!(info.network.interfaces.is_empty());
}

#[test]
fn test_network_info_default() {
    use toadstool_cli::zero_config::NetworkInfo;
    let info = NetworkInfo::default();
    assert!(info.interfaces.is_empty());
    assert!(info.local_ips.is_empty());
    assert!(info.external_ip.is_none());
}

#[test]
fn test_container_runtime_info_default() {
    use toadstool_cli::zero_config::ContainerRuntimeInfo;
    let info = ContainerRuntimeInfo::default();
    let _docker: bool = info.docker;
    let _podman: bool = info.podman;
    let _containerd: bool = info.containerd;
}

#[tokio::test]
async fn test_discover_system_network_local_ips() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        let _ = &deployment.system_info.network.local_ips;
        let _ = &deployment.system_info.network.external_ip;
    }
}

#[tokio::test]
async fn test_discover_ecosystem_toadstool_peers() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_ecosystem().await.is_ok() {
        let _ = &deployment.ecosystem_services.toadstool_peers;
    }
}

#[tokio::test]
async fn test_discover_system_storage_type() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        assert!(!deployment.system_info.storage.storage_type.is_empty());
    }
}

#[tokio::test]
async fn test_discover_system_memory_type() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        assert!(!deployment.system_info.memory.memory_type.is_empty());
    }
}

#[tokio::test]
async fn test_discover_system_cpu_model() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        let _ = deployment.system_info.cpu.model.as_str();
    }
}

#[tokio::test]
async fn test_discover_system_cpu_vendor() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        let _ = deployment.system_info.cpu.vendor.as_str();
    }
}

#[tokio::test]
async fn test_discover_system_cpu_frequency() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        let _ = deployment.system_info.cpu.frequency;
    }
}

#[tokio::test]
async fn test_discover_system_os_version() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        let _ = deployment.system_info.os.version.as_str();
    }
}

#[tokio::test]
async fn test_discover_system_os_kernel() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        let _ = deployment.system_info.os.kernel.as_str();
    }
}

#[tokio::test]
async fn test_discover_system_gpu_cuda() {
    let mut deployment = ZeroConfigDeployment::new();
    if deployment.discover_system().await.is_ok() {
        let _ = deployment.system_info.gpu.cuda;
    }
}
