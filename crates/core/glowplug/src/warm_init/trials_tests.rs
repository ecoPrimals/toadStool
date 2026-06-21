// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

#[test]
fn standard_titanv_lab_has_three_trials() {
    let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
    assert_eq!(lab.trials.len(), 3);
    assert_eq!(lab.trials[0].label, "cold-vfio");
    assert_eq!(lab.trials[1].label, "nouveau-warm");
    assert_eq!(lab.trials[2].label, "nvidia470-vm");
}

#[test]
fn standard_lab_cold_is_bare_metal() {
    let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
    assert!(lab.trials[0].plan.is_bare_metal());
}

#[test]
fn standard_lab_nvidia470_is_contained() {
    let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
    assert!(lab.trials[2].plan.requires_containment());
}

#[test]
fn lab_diff_pairs_three_trials() {
    let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
    let pairs = lab.diff_pairs();
    assert_eq!(pairs, vec![(0, 1), (0, 2), (1, 2)]);
}

#[test]
fn lab_describe_lists_all_trials() {
    let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
    let desc = lab.describe();
    assert!(desc[0].contains("3 trials"));
    assert!(desc.iter().any(|s| s.contains("cold-vfio")));
    assert!(desc.iter().any(|s| s.contains("nouveau-warm")));
    assert!(desc.iter().any(|s| s.contains("nvidia470-vm")));
    assert!(desc.iter().any(|s| s.contains("[contained/VM]")));
    assert!(desc.iter().any(|s| s.contains("[bare-metal]")));
}

#[test]
fn lab_display_format() {
    let lab = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab");
    let s = lab.to_string();
    assert!(s.contains("cold-vfio → nouveau-warm → nvidia470-vm"));
}

#[test]
fn driver_trial_serde_roundtrip() {
    let trial = DriverTrial {
        label: "nouveau-test".into(),
        plan: WarmInitPlan::nouveau_titanv("0000:02:00.0"),
        scan_ranges: vec![("PMC".into(), 0, 0x1000)],
        full_scan: false,
        needs_power_cycle: false,
    };
    let json = serde_json::to_string(&trial).unwrap();
    let back: DriverTrial = serde_json::from_str(&json).unwrap();
    assert_eq!(back.label, "nouveau-test");
    assert!(!back.full_scan);
}

#[test]
fn executor_new_from_plan() {
    let plan = DriverLabPlan::standard_titanv("0000:02:00.0", "/tmp/lab-test");
    let executor = DriverLabExecutor::new(plan);
    assert_eq!(executor.plan().trials.len(), 3);
}

fn fast_test_plan(bdf: &str) -> WarmInitPlan {
    WarmInitPlan {
        seeder_settle: Duration::from_millis(1),
        ..WarmInitPlan::nouveau_titanv(bdf)
    }
}

#[test]
fn executor_runs_with_mock_callbacks() {
    let plan = DriverLabPlan {
        bdf: "0000:02:00.0".into(),
        gpu_description: "Test GPU".into(),
        trials: vec![
            DriverTrial {
                label: "trial-a".into(),
                plan: fast_test_plan("0000:02:00.0"),
                scan_ranges: vec![("PMC".into(), 0, 0x100)],
                full_scan: false,
                needs_power_cycle: false,
            },
            DriverTrial {
                label: "trial-b".into(),
                plan: fast_test_plan("0000:02:00.0"),
                scan_ranges: vec![("PMC".into(), 0, 0x100)],
                full_scan: false,
                needs_power_cycle: false,
            },
        ],
        output_dir: "/tmp/glowplug-lab-test-nonexistent".into(),
    };

    let executor = DriverLabExecutor::new(plan);
    let result = executor.execute(
        |_bdf| Ok(()),
        |_bdf, _seeder| Ok("mock swap".into()),
        |_bdf, label, _ranges| Ok(format!("{{\"label\": \"{label}\"}}").into_bytes()),
    );

    assert_eq!(result.trials.len(), 2);
    assert!(result.trials.iter().all(|t| t.success));
}

#[test]
fn executor_handles_swap_failure() {
    let plan = DriverLabPlan {
        bdf: "0000:02:00.0".into(),
        gpu_description: "Test GPU".into(),
        trials: vec![DriverTrial {
            label: "fail-trial".into(),
            plan: fast_test_plan("0000:02:00.0"),
            scan_ranges: vec![],
            full_scan: false,
            needs_power_cycle: false,
        }],
        output_dir: "/tmp/glowplug-lab-test-fail".into(),
    };

    let executor = DriverLabExecutor::new(plan);
    let result = executor.execute(
        |_| Ok(()),
        |_, _| Err("swap refused".into()),
        |_, _, _| Ok(vec![]),
    );

    assert_eq!(result.trials.len(), 1);
    assert!(!result.trials[0].success);
    assert!(result.trials[0].detail.contains("swap failed"));
}

#[test]
fn executor_handles_power_cycle_failure() {
    let plan = DriverLabPlan {
        bdf: "0000:02:00.0".into(),
        gpu_description: "Test GPU".into(),
        trials: vec![DriverTrial {
            label: "power-fail".into(),
            plan: fast_test_plan("0000:02:00.0"),
            scan_ranges: vec![],
            full_scan: false,
            needs_power_cycle: true,
        }],
        output_dir: "/tmp/glowplug-lab-test-power".into(),
    };

    let executor = DriverLabExecutor::new(plan);
    let result = executor.execute(
        |_| Err("power cycle unavailable".into()),
        |_, _| Ok("ok".into()),
        |_, _, _| Ok(vec![]),
    );

    assert_eq!(result.trials.len(), 1);
    assert!(!result.trials[0].success);
    assert!(result.trials[0].detail.contains("power cycle failed"));
}

#[test]
fn lab_execution_result_display() {
    let result = LabExecutionResult {
        bdf: "0000:02:00.0".into(),
        gpu_description: "Test GPU".into(),
        trials: vec![TrialExecutionResult {
            label: "test".into(),
            success: true,
            detail: "ok".into(),
            snapshot_path: None,
            duration_ms: 42,
            power_cycle_performed: false,
        }],
        diffs: vec![],
        total_ms: 100,
    };
    let s = result.to_string();
    assert!(s.contains("1/1 trials ok"));
}

#[test]
fn trial_execution_result_serde_roundtrip() {
    let result = TrialExecutionResult {
        label: "test".into(),
        success: true,
        detail: "ok".into(),
        snapshot_path: Some("/tmp/test.json".into()),
        duration_ms: 42,
        power_cycle_performed: true,
    };
    let json = serde_json::to_string(&result).unwrap();
    let back: TrialExecutionResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.label, "test");
    assert!(back.success);
    assert!(back.power_cycle_performed);
}

#[test]
fn diff_summary_serde_roundtrip() {
    let diff = DiffSummary {
        trial_a: "a".into(),
        trial_b: "b".into(),
        changed_registers: 42,
        diff_path: Some("/tmp/diff.json".into()),
    };
    let json = serde_json::to_string(&diff).unwrap();
    let back: DiffSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(back.changed_registers, 42);
}

#[test]
fn count_json_diffs_identical() {
    let a = b"hello world";
    let b = b"hello world";
    assert_eq!(count_json_diffs(a, b), 0);
}

#[test]
fn count_json_diffs_different() {
    let a = b"hello";
    let b = b"world";
    assert!(count_json_diffs(a, b) > 0);
}
