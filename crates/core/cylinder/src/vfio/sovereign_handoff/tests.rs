// SPDX-License-Identifier: AGPL-3.0-or-later

use super::lock::HandoffGuard;
use super::rollback::halt_result;
use super::types::{HandoffConfig, HandoffResult, HandoffStep, ModuleSourceConfig};
use crate::vfio::sovereign_tiers::TierEvidence;
use std::time::Instant;

/// Parse `modprobe --show-depends` output into a list of dependency `.ko` paths,
/// excluding the target module itself. Kept for test coverage of the fallback
/// parser in `kmod::resolve_from_modprobe`.
fn parse_modprobe_deps(output: &str, target_module: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let ko_path = line.strip_prefix("insmod ")?.trim();
            if ko_path.contains(&format!("/{target_module}.ko")) {
                None
            } else {
                Some(ko_path.to_string())
            }
        })
        .collect()
}

#[test]
fn config_from_strategy_resolves_known() {
    assert!(HandoffConfig::from_strategy("nouveau_titanv", "0000:02:00.0").is_some());
    assert!(HandoffConfig::from_strategy("nouveau_k80", "0000:49:00.0").is_some());
    assert!(HandoffConfig::from_strategy("nvidia_titanv", "0000:02:00.0").is_some());
    assert!(HandoffConfig::from_strategy("nvidia_patched_titanv", "0000:02:00.0").is_some());
    assert!(HandoffConfig::from_strategy("unknown", "0000:02:00.0").is_none());
}

#[test]
fn titanv_config_uses_patched_source() {
    let cfg = HandoffConfig::nouveau_titanv("0000:02:00.0");
    assert!(matches!(
        cfg.module_source,
        ModuleSourceConfig::Patched { .. }
    ));
    assert_eq!(cfg.seeder_driver, "nouveau");
    assert_eq!(cfg.final_driver, "vfio-pci");
}

#[test]
fn k80_config_uses_system_source() {
    let cfg = HandoffConfig::nouveau_k80("0000:49:00.0");
    assert!(matches!(cfg.module_source, ModuleSourceConfig::System));
}

#[test]
fn nvidia_titanv_config_uses_system_nvidia() {
    let cfg = HandoffConfig::nvidia_titanv("0000:02:00.0");
    assert!(matches!(cfg.module_source, ModuleSourceConfig::System));
    assert_eq!(cfg.seeder_driver, "nvidia");
    assert_eq!(cfg.module_name, "nvidia");
    assert_eq!(cfg.final_driver, "vfio-pci");
    assert_eq!(cfg.settle.as_secs(), 10);
}

#[test]
fn nvidia_patched_titanv_uses_renamed_module() {
    let cfg = HandoffConfig::nvidia_patched_titanv("0000:02:00.0");
    assert!(matches!(
        cfg.module_source,
        ModuleSourceConfig::DkmsPatched { .. }
    ));
    assert_eq!(cfg.seeder_driver, "nvsov");
    assert_eq!(cfg.module_name, "nvsov");
    if let ModuleSourceConfig::DkmsPatched {
        dkms_module,
        dkms_version,
        patch_set,
    } = &cfg.module_source
    {
        assert_eq!(dkms_module, "nvidia");
        assert_eq!(dkms_version, "470.256.02");
        assert_eq!(patch_set, "nvidia_warm_handoff");
    }
}

#[test]
fn nvidia_catalyst_titanv_uses_catalyst_patch_set() {
    let cfg = HandoffConfig::nvidia_catalyst_titanv("0000:49:00.0");
    assert!(matches!(
        cfg.module_source,
        ModuleSourceConfig::DkmsPatched { .. }
    ));
    assert_eq!(cfg.seeder_driver, "nvsov");
    assert_eq!(cfg.module_name, "nvsov");
    assert_eq!(cfg.settle.as_secs(), 60);
    if let ModuleSourceConfig::DkmsPatched {
        dkms_module,
        dkms_version,
        patch_set,
    } = &cfg.module_source
    {
        assert_eq!(dkms_module, "nvidia");
        assert_eq!(dkms_version, "470.256.02");
        assert_eq!(patch_set, "nvidia_catalyst_handoff");
    }
}

#[test]
fn from_strategy_resolves_catalyst() {
    assert!(HandoffConfig::from_strategy("nvidia_catalyst_titanv", "0000:02:00.0").is_some());
}

#[test]
fn handoff_result_display_success() {
    let r = HandoffResult {
        bdf: "0000:02:00.0".into(),
        success: true,
        halted_at: None,
        steps: vec![],
        patch_result: None,
        tier: Some(TierEvidence {
            tier: crate::vfio::sovereign_tiers::SovereignTier::WarmInfrastructure,
            pmc_enable: 0xFFFF_FFFF,
            pmc_popcount: 32,
            pramin_accessible: true,
            fecs_pc: Some(0),
            gpc_enables: None,
            ce_status: None,
            gr_status: None,
            pbdma_intr: None,
            ce_runlist: None,
            tpc_status: None,
            tpc_alive: false,
        }),
        module_loaded: true,
        module_unloaded: true,
        catalyst_snapshot_path: None,
        catalyst_alive_count: None,
        catalyst_tier: None,
        rm_channel_evidence: None,
        boot_service_evidence: None,
        pri_ring_anchor: None,
        total_ms: 6000,
    };
    let s = r.to_string();
    assert!(s.contains("HANDOFF OK"));
    assert!(s.contains("Tier 1"));
}

#[test]
fn handoff_result_display_halted() {
    let r = HandoffResult {
        bdf: "0000:02:00.0".into(),
        success: false,
        halted_at: Some("seeder_bind".into()),
        steps: vec![],
        patch_result: None,
        tier: None,
        module_loaded: false,
        module_unloaded: false,
        catalyst_snapshot_path: None,
        catalyst_alive_count: None,
        catalyst_tier: None,
        rm_channel_evidence: None,
        boot_service_evidence: None,
        pri_ring_anchor: None,
        total_ms: 100,
    };
    let s = r.to_string();
    assert!(s.contains("HALTED@seeder_bind"));
}

#[test]
fn handoff_result_serde_roundtrip() {
    let r = HandoffResult {
        bdf: "0000:02:00.0".into(),
        success: true,
        halted_at: None,
        steps: vec![HandoffStep {
            name: "module_prep".into(),
            ok: true,
            detail: Some("patched module loaded".into()),
            duration_ms: 200,
        }],
        patch_result: None,
        tier: None,
        module_loaded: true,
        module_unloaded: true,
        catalyst_snapshot_path: None,
        catalyst_alive_count: None,
        catalyst_tier: None,
        rm_channel_evidence: None,
        boot_service_evidence: None,
        pri_ring_anchor: None,
        total_ms: 5000,
    };
    let json = serde_json::to_string(&r).unwrap();
    let back: HandoffResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.bdf, "0000:02:00.0");
    assert!(back.success);
    assert_eq!(back.steps.len(), 1);
}

#[test]
fn handoff_guard_acquire_release() {
    // Use unique BDFs to avoid interference with parallel tests
    let bdf = "test:aa:00.0";
    let guard = HandoffGuard::acquire(bdf).unwrap();

    // Double-acquire should fail
    let second = HandoffGuard::acquire(bdf);
    assert!(second.is_err());

    // Drop the guard
    drop(guard);

    // Re-acquire should succeed after drop
    let guard2 = HandoffGuard::acquire(bdf).unwrap();
    drop(guard2);
}

#[test]
fn handoff_guard_raii_drop() {
    let bdf = "test:bb:00.0";
    {
        let _guard = HandoffGuard::acquire(bdf).unwrap();
        // guard drops at end of scope
    }
    // Should be re-acquirable
    let _guard = HandoffGuard::acquire(bdf).unwrap();
}

#[test]
fn halt_result_rollback_with_needs_device_rollback() {
    // Even with module_loaded=false and empty sibling_state,
    // needs_device_rollback=true should trigger rollback step
    let steps = vec![HandoffStep {
        name: "test".into(),
        ok: true,
        detail: None,
        duration_ms: 0,
    }];
    let result = halt_result(
        "ffff:ff:ff.f",
        "test_halt",
        steps,
        None,
        false,
        false,
        Instant::now(),
        &[],
        "nouveau",
        true, // needs_device_rollback
    );
    assert!(!result.success);
    assert_eq!(result.halted_at.as_deref(), Some("test_halt"));
    // Should have 2 steps: original + rollback
    assert_eq!(result.steps.len(), 2);
    assert_eq!(result.steps[1].name, "rollback");
    let detail = result.steps[1].detail.as_ref().unwrap();
    assert!(detail.contains("device=true"));
}

#[test]
fn halt_result_no_rollback_when_nothing_needed() {
    let steps = vec![HandoffStep {
        name: "test".into(),
        ok: false,
        detail: None,
        duration_ms: 0,
    }];
    let result = halt_result(
        "ffff:ff:ff.f",
        "preflight",
        steps,
        None,
        false,
        false,
        Instant::now(),
        &[],
        "nouveau",
        false,
    );
    // Only 1 step — no rollback triggered
    assert_eq!(result.steps.len(), 1);
}

#[test]
fn halt_result_rollback_with_module_loaded() {
    let steps = vec![];
    let result = halt_result(
        "ffff:ff:ff.f",
        "warm_swap",
        steps,
        None,
        true, // module_loaded
        false,
        Instant::now(),
        &[],
        "nouveau",
        false,
    );
    assert_eq!(result.steps.len(), 1);
    assert_eq!(result.steps[0].name, "rollback");
    let detail = result.steps[0].detail.as_ref().unwrap();
    assert!(detail.contains("module=true"));
}

#[test]
fn halt_result_rollback_with_siblings() {
    let siblings = vec![(
        "0000:02:00.1".to_string(),
        Some("snd_hda_intel".to_string()),
    )];
    let steps = vec![];
    let result = halt_result(
        "ffff:ff:ff.f",
        "warm_swap",
        steps,
        None,
        false,
        false,
        Instant::now(),
        &siblings,
        "nouveau",
        false,
    );
    assert_eq!(result.steps.len(), 1);
    assert_eq!(result.steps[0].name, "rollback");
    let detail = result.steps[0].detail.as_ref().unwrap();
    assert!(detail.contains("siblings=1"));
}

#[test]
fn parse_modprobe_deps_extracts_paths() {
    let output = "\
insmod /lib/modules/6.17.9/kernel/drivers/gpu/drm/drm.ko
insmod /lib/modules/6.17.9/kernel/drivers/gpu/drm/drm_gpuvm.ko
insmod /lib/modules/6.17.9/kernel/drivers/gpu/drm/scheduler/gpu-sched.ko
insmod /lib/modules/6.17.9/kernel/drivers/gpu/drm/nouveau/nouveau.ko
";
    let deps = parse_modprobe_deps(output, "nouveau");
    assert_eq!(deps.len(), 3);
    assert!(deps[0].ends_with("drm.ko"));
    assert!(deps[1].ends_with("drm_gpuvm.ko"));
    assert!(deps[2].ends_with("gpu-sched.ko"));
}

#[test]
fn parse_modprobe_deps_handles_install_lines() {
    let output = "\
install /sbin/modprobe --ignore-install some-mod
insmod /lib/modules/6.17.9/dep.ko
insmod /lib/modules/6.17.9/nouveau.ko
";
    let deps = parse_modprobe_deps(output, "nouveau");
    assert_eq!(deps.len(), 1);
    assert!(deps[0].ends_with("dep.ko"));
}

#[test]
fn parse_modprobe_deps_empty_output() {
    let deps = parse_modprobe_deps("", "nouveau");
    assert!(deps.is_empty());
}

#[test]
fn parse_modprobe_deps_only_target() {
    let output = "insmod /lib/modules/6.17.9/nouveau.ko\n";
    let deps = parse_modprobe_deps(output, "nouveau");
    assert!(deps.is_empty());
}
