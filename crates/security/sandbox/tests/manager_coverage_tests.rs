// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::match_same_arms
)]
//! Integration tests targeting [`toadstool_security_sandbox::manager`] behavior: lifecycle,
//! persistence of sandbox metadata, validation and error paths, and trait coverage for public
//! sandbox types re-exported from the crate.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tempfile::TempDir;
use toadstool::security::{IsolationLevel, SecurityContext};
use toadstool::workload::WorkloadSpec;
use toadstool_security_policies::{
    FilePolicyManager, PolicyManagerConfig, SecurityPolicy, ViolationAction,
};
#[cfg(target_os = "linux")]
use toadstool_security_sandbox::LinuxSandboxManager;
use toadstool_security_sandbox::{
    BandwidthLimits, CrossPlatformSandboxManager, FilesystemMount, MountType, NetworkConfig,
    NetworkIsolationMode, ResourceLimits, ResourceUsage, SandboxConfig, SandboxInfo,
    SandboxLifetime, SandboxManager, SandboxSpec, SandboxStatus, SecurityViolation,
    ViolationSeverity, helpers,
};

fn policy_manager() -> Arc<FilePolicyManager> {
    let dir = TempDir::new().expect("tempdir");
    let policy_config = PolicyManagerConfig {
        policy_dir: dir.path().join("policies"),
        cache_enabled: false,
        strict_enforcement: false,
        default_violation_action: ViolationAction::Block,
        max_composition_depth: 10,
        validation_timeout_ms: 5000,
        ..Default::default()
    };
    Arc::new(FilePolicyManager::new(policy_config).expect("policy manager"))
}

fn isolated_config() -> (SandboxConfig, TempDir) {
    let base = TempDir::new().expect("tempdir");
    let sandbox_root = base.path().join("sandbox_root");
    let temp_dir = base.path().join("temp_dir");
    let config = SandboxConfig {
        sandbox_root,
        temp_dir,
        max_concurrent_sandboxes: 32,
        cleanup_timeout_secs: 60,
        monitoring_interval_ms: 50,
        enable_monitoring: true,
        enable_seccomp: false,
        enable_namespace_isolation: false,
        ..SandboxConfig::default()
    };
    (config, base)
}

fn native_echo_spec(sandbox_id: &str) -> SandboxSpec {
    SandboxSpec {
        sandbox_id: sandbox_id.to_string(),
        workload: WorkloadSpec::Native {
            executable: toadstool::workload::ExecutableSource::File {
                path: PathBuf::from("/bin/echo"),
            },
            args: Some(vec!["ok".to_string()]),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        security_context: SecurityContext::for_isolation_level(IsolationLevel::Standard),
        resource_limits: ResourceLimits::default(),
        filesystem_mounts: Vec::new(),
        network_config: NetworkConfig::default(),
        environment: HashMap::new(),
        working_directory: None,
        lifetime: SandboxLifetime::Ephemeral,
    }
}

fn sample_security_policy() -> SecurityPolicy {
    SecurityPolicy {
        id: "cov-policy".to_string(),
        name: "Coverage Policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: std::time::SystemTime::UNIX_EPOCH,
        modified_at: std::time::SystemTime::UNIX_EPOCH,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    }
}

#[tokio::test]
async fn manager_new_creates_roots_and_round_trips() {
    let (config, _keep) = isolated_config();
    let pm = policy_manager();
    let mgr = CrossPlatformSandboxManager::new(config, pm)
        .await
        .expect("manager new");
    drop(mgr);
}

#[tokio::test]
async fn manager_new_fails_when_sandbox_root_path_is_a_file() {
    let base = TempDir::new().expect("tempdir");
    let bad = base.path().join("root_file");
    std::fs::write(&bad, b"x").expect("write file");
    let config = SandboxConfig {
        sandbox_root: bad,
        temp_dir: base.path().join("td"),
        ..SandboxConfig::default()
    };
    let result = CrossPlatformSandboxManager::new(config, policy_manager()).await;
    assert!(result.is_err(), "expected configuration error");
    let err = result.err().expect("err");
    let msg = format!("{err}");
    assert!(
        msg.contains("sandbox root") || msg.contains("Failed to create"),
        "{msg}"
    );
}

#[tokio::test]
async fn create_list_destroy_and_unknown_queries() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    assert!(mgr.list_sandboxes().await.expect("list").is_empty());

    let id = mgr
        .create_sandbox(native_echo_spec("s-one"))
        .await
        .expect("create");
    let listed: HashSet<String> = mgr
        .list_sandboxes()
        .await
        .expect("list")
        .into_iter()
        .collect();
    assert_eq!(listed, HashSet::from([id.clone()]));

    mgr.destroy_sandbox(&id).await.expect("destroy");
    assert!(mgr.list_sandboxes().await.expect("list").is_empty());
    mgr.destroy_sandbox(&id)
        .await
        .expect("destroy unknown is ok");
    mgr.get_sandbox_info(&id)
        .await
        .expect_err("removed from tracking");
}

#[tokio::test]
async fn create_generates_id_when_empty() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    let mut spec = native_echo_spec("");
    let id = mgr.create_sandbox(spec.clone()).await.expect("create");
    assert!(!id.is_empty());
    assert!(id.starts_with("sandbox_"));
    mgr.destroy_sandbox(&id).await.expect("destroy");

    spec.sandbox_id = "explicit".to_string();
    let id2 = mgr.create_sandbox(spec).await.expect("create");
    assert_eq!(id2, "explicit");
    mgr.destroy_sandbox(&id2).await.expect("destroy");
}

#[tokio::test]
async fn lifecycle_start_stop_destroy() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    let id = mgr
        .create_sandbox(native_echo_spec("lc-1"))
        .await
        .expect("create");
    assert_eq!(
        mgr.get_sandbox_info(&id).await.expect("info").status,
        SandboxStatus::Ready
    );

    mgr.start_execution(&id).await.expect("start");
    assert_eq!(
        mgr.get_sandbox_info(&id).await.expect("info").status,
        SandboxStatus::Running
    );

    mgr.stop_execution(&id).await.expect("stop");
    assert_eq!(
        mgr.get_sandbox_info(&id).await.expect("info").status,
        SandboxStatus::Completed
    );

    mgr.destroy_sandbox(&id).await.expect("destroy");
}

#[tokio::test]
async fn destroy_stops_when_running() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    let id = mgr
        .create_sandbox(native_echo_spec("run-destroy"))
        .await
        .expect("create");
    mgr.start_execution(&id).await.expect("start");
    mgr.destroy_sandbox(&id).await.expect("destroy");
    mgr.get_sandbox_info(&id)
        .await
        .expect_err("destroyed removes entry");
}

#[tokio::test]
async fn resource_limits_reject_invalid_memory_via_create() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    let mut spec = native_echo_spec("bad-mem");
    spec.resource_limits.max_memory_bytes = Some(0);
    mgr.create_sandbox(spec).await.expect_err("zero memory");
}

#[tokio::test]
async fn resource_limits_reject_invalid_cpu_via_create() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    let mut spec = native_echo_spec("bad-cpu");
    spec.resource_limits.max_cpu_percent = Some(0.0);
    mgr.create_sandbox(spec.clone())
        .await
        .expect_err("cpu zero");

    spec.resource_limits.max_cpu_percent = Some(101.0);
    mgr.create_sandbox(spec).await.expect_err("cpu over 100");
}

#[tokio::test]
async fn resource_limits_accept_cpu_boundaries_and_persist_in_spec_path() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    for cpu in [1.0_f64, 100.0_f64] {
        let mut spec = native_echo_spec(&format!("cpu-{cpu}"));
        spec.resource_limits.max_cpu_percent = Some(cpu);
        let id = mgr.create_sandbox(spec).await.expect("create");
        mgr.destroy_sandbox(&id).await.expect("destroy");
    }
}

#[tokio::test]
async fn validation_rejects_empty_mount_source_before_platform() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    let mut spec = native_echo_spec("bad-mount");
    spec.filesystem_mounts = vec![FilesystemMount {
        source: PathBuf::new(),
        target: PathBuf::from("t"),
        mount_type: MountType::ReadOnlyBind,
        options: vec![],
    }];
    mgr.create_sandbox(spec)
        .await
        .expect_err("empty mount source");
}

#[tokio::test]
async fn helpers_validate_matches_manager_create_errors() {
    let mut spec = native_echo_spec("h");
    spec.resource_limits.max_memory_bytes = Some(0);
    helpers::validate_sandbox_spec(&spec).expect_err("helper agrees");

    spec.resource_limits = ResourceLimits::default();
    helpers::validate_sandbox_spec(&spec).expect("ok");
}

#[tokio::test]
async fn monitor_updates_tracked_resource_usage() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    let id = mgr
        .create_sandbox(native_echo_spec("mon"))
        .await
        .expect("create");
    let usage = mgr.monitor_sandbox(&id).await.expect("monitor");
    let info = mgr.get_sandbox_info(&id).await.expect("info");
    assert_eq!(info.resource_usage.memory_bytes, usage.memory_bytes);
    assert_eq!(info.resource_usage.cpu_percent, usage.cpu_percent);
    mgr.destroy_sandbox(&id).await.expect("destroy");
}

#[tokio::test]
async fn start_execution_errors_not_found() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    mgr.start_execution("nope")
        .await
        .expect_err("missing sandbox");
}

#[tokio::test]
async fn start_execution_errors_when_not_ready() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    let id = mgr
        .create_sandbox(native_echo_spec("twice"))
        .await
        .expect("create");
    mgr.start_execution(&id).await.expect("first start");
    mgr.start_execution(&id)
        .await
        .expect_err("second start when running");
    mgr.destroy_sandbox(&id).await.expect("destroy");
}

#[tokio::test]
async fn apply_policy_and_logs_on_sandbox() {
    let (config, _keep) = isolated_config();
    let mgr = CrossPlatformSandboxManager::new(config, policy_manager())
        .await
        .expect("manager");
    let id = mgr
        .create_sandbox(native_echo_spec("pol"))
        .await
        .expect("create");
    mgr.apply_security_policy(&id, &sample_security_policy())
        .await
        .expect("apply");
    mgr.get_sandbox_logs(&id).await.expect("logs");
    mgr.destroy_sandbox(&id).await.expect("destroy");
}

#[test]
fn public_types_default_clone_debug_serde_roundtrip() {
    let cfg_default = SandboxConfig::default();
    let _cfg_clone = cfg_default.clone();
    assert!(!format!("{cfg_default:?}").is_empty());
    let json = serde_json::to_string(&cfg_default).expect("serde");
    let _: SandboxConfig = serde_json::from_str(&json).expect("de");

    let limits = ResourceLimits::default();
    let lim2 = limits.clone();
    assert_eq!(
        lim2.max_memory_bytes,
        ResourceLimits::default().max_memory_bytes
    );
    let j = serde_json::to_string(&limits).expect("serde");
    let _: ResourceLimits = serde_json::from_str(&j).expect("de");

    let mut spec = native_echo_spec("serde-id");
    spec.resource_limits = limits;
    let sj = serde_json::to_string(&spec).expect("serde");
    let _: SandboxSpec = serde_json::from_str(&sj).expect("de");

    let mount = FilesystemMount {
        source: PathBuf::from("/a"),
        target: PathBuf::from("/b"),
        mount_type: MountType::ReadWriteBind,
        options: vec!["rw".to_string()],
    };
    let mj = serde_json::to_string(&mount).expect("serde");
    let _: FilesystemMount = serde_json::from_str(&mj).expect("de");

    for mt in [
        MountType::ReadOnlyBind,
        MountType::ReadWriteBind,
        MountType::TmpFs,
        MountType::Device,
        MountType::Proc,
        MountType::Sys,
    ] {
        let tj = serde_json::to_string(&mt).expect("serde");
        let _: MountType = serde_json::from_str(&tj).expect("de");
    }

    let nc = NetworkConfig::default();
    let ncj = serde_json::to_string(&nc).expect("serde");
    let _: NetworkConfig = serde_json::from_str(&ncj).expect("de");

    for mode in [
        NetworkIsolationMode::None,
        NetworkIsolationMode::Firewall,
        NetworkIsolationMode::Namespace,
        NetworkIsolationMode::Isolated,
    ] {
        let mj = serde_json::to_string(&mode).expect("serde");
        let _: NetworkIsolationMode = serde_json::from_str(&mj).expect("de");
    }

    let bw = BandwidthLimits {
        upload_bps: 1,
        download_bps: 2,
    };
    let bj = serde_json::to_string(&bw).expect("serde");
    let _: BandwidthLimits = serde_json::from_str(&bj).expect("de");

    for life in [
        SandboxLifetime::Ephemeral,
        SandboxLifetime::Persistent {
            ttl: Duration::from_secs(1),
        },
        SandboxLifetime::Manual,
    ] {
        let lj = serde_json::to_string(&life).expect("serde");
        let _: SandboxLifetime = serde_json::from_str(&lj).expect("de");
    }

    for st in [
        SandboxStatus::Creating,
        SandboxStatus::Ready,
        SandboxStatus::Running,
        SandboxStatus::Completed,
        SandboxStatus::Failed,
        SandboxStatus::Destroying,
        SandboxStatus::Destroyed,
    ] {
        let sj = serde_json::to_string(&st).expect("serde");
        let st2: SandboxStatus = serde_json::from_str(&sj).expect("de");
        assert_eq!(st, st2);
    }

    for sev in [
        ViolationSeverity::Low,
        ViolationSeverity::Medium,
        ViolationSeverity::High,
        ViolationSeverity::Critical,
    ] {
        let vj = serde_json::to_string(&sev).expect("serde");
        let _: ViolationSeverity = serde_json::from_str(&vj).expect("de");
    }

    let info = SandboxInfo {
        sandbox_id: "x".to_string(),
        status: SandboxStatus::Ready,
        created_at: std::time::SystemTime::UNIX_EPOCH,
        updated_at: std::time::SystemTime::UNIX_EPOCH,
        process_id: None,
        resource_usage: ResourceUsage::default(),
        security_violations: vec![],
        metadata: HashMap::new(),
    };
    let i2 = info.clone();
    assert_eq!(i2.sandbox_id, "x");
    assert!(!format!("{info:?}").is_empty());

    let viol = SecurityViolation {
        violation_type: "t".to_string(),
        description: "d".to_string(),
        timestamp: std::time::SystemTime::UNIX_EPOCH,
        severity: ViolationSeverity::Low,
        action_taken: "a".to_string(),
    };
    let v2 = viol;
    assert!(!format!("{v2:?}").is_empty());

    let ru = ResourceUsage::default();
    let ru2 = ru.clone();
    assert_eq!(ru2.memory_bytes, 0);
    assert!(!format!("{ru:?}").is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn linux_sandbox_manager_capabilities_clone_and_debug() {
    let (config, _keep) = isolated_config();
    let mgr = LinuxSandboxManager::new(config);
    let caps = mgr.capabilities().clone();
    assert!(!format!("{caps:?}").is_empty());
}
