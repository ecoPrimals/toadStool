// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{ModuleSource, NV_BAR0_DOMAINS, SeederContainment, SeederDriver, WarmInitPlan};

/// A single trial in a driver comparison lab.
///
/// Each trial binds a different seeder to the same GPU and captures
/// a BAR0 snapshot after initialization. Comparing snapshots across
/// trials reveals exactly what each driver initializes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverTrial {
    /// Human-readable label for this trial (e.g., "nouveau-warm", "nvidia470-vm").
    pub label: String,

    /// The warm init plan that seeds this trial.
    pub plan: WarmInitPlan,

    /// BAR0 register ranges to scan after seeder init.
    /// Each tuple is (domain_name, start_offset, end_offset).
    /// If empty, a default set of NVIDIA domains is used.
    pub scan_ranges: Vec<(String, usize, usize)>,

    /// Whether to perform a full BAR0 scan (slow, ~4M reads on 16MB BAR).
    /// If false, only `scan_ranges` are probed.
    pub full_scan: bool,

    /// Whether a GPU power cycle is needed before this trial.
    /// Required when transitioning from a warm state to test cold behavior,
    /// or when the previous trial's seeder leaves state that would confound results.
    pub needs_power_cycle: bool,
}

/// A plan to compare multiple drivers/seeders on the same GPU.
///
/// The diesel engine executes each trial in sequence, capturing BAR0
/// snapshots after each seeder initializes the GPU. The resulting
/// snapshots can be diffed to understand what each driver does differently.
///
/// ```text
/// Trial 1: vfio-pci (cold)     → BAR0 snapshot A (baseline)
/// Trial 2: nouveau (warm)      → BAR0 snapshot B
/// Trial 3: nvidia-470 (VM)     → BAR0 snapshot C (via agentReagents)
///
/// Diff A→B: what nouveau initializes from cold
/// Diff A→C: what nvidia-470 initializes from cold
/// Diff B→C: what nvidia-470 adds beyond nouveau (FECS, SEC2, ACR)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverLabPlan {
    /// Target GPU's PCI BDF.
    pub bdf: String,

    /// GPU description (e.g., "Titan V (GV100)").
    pub gpu_description: String,

    /// Ordered list of trials to execute.
    pub trials: Vec<DriverTrial>,

    /// Where to persist snapshots (directory path).
    pub output_dir: String,
}

impl DriverLabPlan {
    /// Standard comparison plan: cold baseline vs nouveau vs nvidia-470.
    ///
    /// Trial 1 captures the cold/vfio-pci baseline (no seeder — GPU as
    /// VFIO leaves it after bind). Trial 2 seeds with nouveau for mesa/open
    /// comparison. Trial 3 uses nvidia-470 in an agentReagents VM for the
    /// full vendor init comparison.
    #[must_use]
    pub fn standard_titanv(bdf: &str, output_dir: &str) -> Self {
        let domains: Vec<(String, usize, usize)> = NV_BAR0_DOMAINS
            .iter()
            .map(|&(name, start, end)| (name.to_string(), start, end))
            .collect();

        Self {
            bdf: bdf.to_string(),
            gpu_description: "Titan V (GV100)".into(),
            trials: vec![
                DriverTrial {
                    label: "cold-vfio".into(),
                    plan: WarmInitPlan {
                        bdf: bdf.to_string(),
                        seeder: SeederDriver {
                            name: "vfio-pci".into(),
                            module: "vfio-pci".into(),
                            module_source: ModuleSource::System,
                            initializes: vec!["nothing — cold baseline".into()],
                            limitations: vec![
                                "PRI ring dead after FLR".into(),
                                "All PGRAPH/FECS registers return 0xbadf".into(),
                            ],
                        },
                        containment: SeederContainment::BareMetal,
                        seeder_settle: Duration::from_secs(2),
                        final_target: "vfio-pci".into(),
                    },
                    scan_ranges: domains.clone(),
                    full_scan: false,
                    needs_power_cycle: true,
                },
                DriverTrial {
                    label: "nouveau-warm".into(),
                    plan: WarmInitPlan::nouveau_titanv(bdf),
                    scan_ranges: domains.clone(),
                    full_scan: false,
                    needs_power_cycle: true,
                },
                DriverTrial {
                    label: "nvidia470-vm".into(),
                    plan: WarmInitPlan::nvidia470_titanv(bdf),
                    scan_ranges: domains,
                    full_scan: false,
                    needs_power_cycle: true,
                },
            ],
            output_dir: output_dir.to_string(),
        }
    }

    /// Describe the lab plan as a human-readable sequence.
    #[must_use]
    pub fn describe(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(format!(
            "Driver Lab: {} ({}) — {} trials",
            self.gpu_description,
            self.bdf,
            self.trials.len(),
        ));

        for (i, trial) in self.trials.iter().enumerate() {
            let containment = if trial.plan.requires_containment() {
                " [contained/VM]"
            } else {
                " [bare-metal]"
            };
            let power = if trial.needs_power_cycle {
                " (power cycle first)"
            } else {
                ""
            };
            let scan = if trial.full_scan {
                "full BAR0"
            } else {
                &format!("{} ranges", trial.scan_ranges.len())
            };
            out.push(format!(
                "  Trial {}: {} via {}{}{} — scan {}",
                i + 1,
                trial.label,
                trial.plan.seeder.name,
                containment,
                power,
                scan,
            ));
        }

        out.push(format!("Output: {}", self.output_dir));

        let n = self.trials.len();
        if n >= 2 {
            out.push(format!(
                "Diffs to generate: {} pairwise comparisons",
                n * (n - 1) / 2
            ));
        }

        out
    }

    /// List all pairwise diff combinations that should be generated.
    #[must_use]
    pub fn diff_pairs(&self) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        for i in 0..self.trials.len() {
            for j in (i + 1)..self.trials.len() {
                pairs.push((i, j));
            }
        }
        pairs
    }
}

impl std::fmt::Display for DriverLabPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DriverLab({}, {} trials: {})",
            self.bdf,
            self.trials.len(),
            self.trials
                .iter()
                .map(|t| t.label.as_str())
                .collect::<Vec<_>>()
                .join(" → "),
        )
    }
}

/// Result of a single trial execution within a `DriverLabPlan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialExecutionResult {
    /// The trial label (mirrors `DriverTrial::label`).
    pub label: String,
    /// Whether this trial completed successfully.
    pub success: bool,
    /// Human-readable detail.
    pub detail: String,
    /// Path to the persisted BAR0 snapshot JSON, if saved.
    pub snapshot_path: Option<String>,
    /// Duration of this trial in milliseconds.
    pub duration_ms: u64,
    /// Whether a power cycle was needed (and presumably performed).
    pub power_cycle_performed: bool,
}

/// Result of executing a full `DriverLabPlan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabExecutionResult {
    /// The target BDF.
    pub bdf: String,
    /// GPU description.
    pub gpu_description: String,
    /// Per-trial outcomes.
    pub trials: Vec<TrialExecutionResult>,
    /// Pairwise diff summary (trial_a, trial_b, changed_registers).
    pub diffs: Vec<DiffSummary>,
    /// Total wall-clock time in milliseconds.
    pub total_ms: u64,
}

/// Summary of a pairwise diff between two trials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Label of the first trial.
    pub trial_a: String,
    /// Label of the second trial.
    pub trial_b: String,
    /// Number of registers that changed between the two snapshots.
    pub changed_registers: usize,
    /// Path to the persisted diff JSON, if saved.
    pub diff_path: Option<String>,
}

/// Executor for `DriverLabPlan` — orchestrates trial runs.
///
/// The executor does NOT perform driver swaps or power cycles itself.
/// Instead it accepts callbacks for each step, making it testable and
/// composable with any swap mechanism (bare-metal glowplug swap,
/// agentReagents VM, manual operator).
///
/// # Lifecycle per trial
///
/// ```text
/// 1. power_cycle_fn (if trial.needs_power_cycle)
/// 2. swap_fn (bind seeder driver)
/// 3. settle (wait seeder_settle duration)
/// 4. capture_fn (BAR0 snapshot via WarmStateCapture/TrialResult)
/// 5. persist snapshot to output_dir
/// ```
///
/// After all trials, pairwise diffs are computed and persisted.
pub struct DriverLabExecutor {
    plan: DriverLabPlan,
}

impl DriverLabExecutor {
    /// Create an executor for the given plan.
    pub fn new(plan: DriverLabPlan) -> Self {
        Self { plan }
    }

    /// The underlying plan.
    pub fn plan(&self) -> &DriverLabPlan {
        &self.plan
    }

    /// Execute all trials using the provided callbacks.
    ///
    /// - `power_cycle_fn`: called when a trial requires a power cycle.
    ///   Returns `Ok(())` on success, `Err(reason)` to abort.
    /// - `swap_fn`: binds the seeder driver. Receives `(bdf, seeder_name)`.
    ///   Returns `Ok(detail_string)` on success.
    /// - `capture_fn`: captures BAR0 state. Receives `(bdf, trial_label, scan_ranges)`.
    ///   Returns `Ok(snapshot_json_bytes)` with the serialized snapshot.
    pub fn execute<F1, F2, F3>(
        &self,
        mut power_cycle_fn: F1,
        mut swap_fn: F2,
        mut capture_fn: F3,
    ) -> LabExecutionResult
    where
        F1: FnMut(&str) -> Result<(), String>,
        F2: FnMut(&str, &str) -> Result<String, String>,
        F3: FnMut(&str, &str, &[(String, usize, usize)]) -> Result<Vec<u8>, String>,
    {
        let lab_start = std::time::Instant::now();
        let mut trial_results = Vec::new();
        let mut snapshots: Vec<(String, Vec<u8>)> = Vec::new();

        for trial in &self.plan.trials {
            let trial_start = std::time::Instant::now();
            let mut power_cycle_performed = false;

            if trial.needs_power_cycle {
                match power_cycle_fn(&self.plan.bdf) {
                    Ok(()) => {
                        power_cycle_performed = true;
                    }
                    Err(reason) => {
                        trial_results.push(TrialExecutionResult {
                            label: trial.label.clone(),
                            success: false,
                            detail: format!("power cycle failed: {reason}"),
                            snapshot_path: None,
                            duration_ms: trial_start.elapsed().as_millis() as u64,
                            power_cycle_performed: false,
                        });
                        continue;
                    }
                }
            }

            match swap_fn(&self.plan.bdf, &trial.plan.seeder.name) {
                Ok(swap_detail) => {
                    tracing::info!(
                        label = trial.label.as_str(),
                        seeder = trial.plan.seeder.name.as_str(),
                        detail = swap_detail.as_str(),
                        "trial swap complete"
                    );
                }
                Err(reason) => {
                    trial_results.push(TrialExecutionResult {
                        label: trial.label.clone(),
                        success: false,
                        detail: format!("swap failed: {reason}"),
                        snapshot_path: None,
                        duration_ms: trial_start.elapsed().as_millis() as u64,
                        power_cycle_performed,
                    });
                    continue;
                }
            }

            std::thread::sleep(trial.plan.seeder_settle);

            match capture_fn(&self.plan.bdf, &trial.label, &trial.scan_ranges) {
                Ok(snapshot_bytes) => {
                    let snapshot_path = format!("{}/{}.json", self.plan.output_dir, trial.label);
                    let saved = std::fs::create_dir_all(&self.plan.output_dir)
                        .and_then(|()| std::fs::write(&snapshot_path, &snapshot_bytes));

                    let path = match saved {
                        Ok(()) => Some(snapshot_path),
                        Err(e) => {
                            tracing::warn!(
                                label = trial.label.as_str(),
                                error = %e,
                                "failed to persist snapshot"
                            );
                            None
                        }
                    };

                    snapshots.push((trial.label.clone(), snapshot_bytes));

                    trial_results.push(TrialExecutionResult {
                        label: trial.label.clone(),
                        success: true,
                        detail: "capture complete".into(),
                        snapshot_path: path,
                        duration_ms: trial_start.elapsed().as_millis() as u64,
                        power_cycle_performed,
                    });
                }
                Err(reason) => {
                    trial_results.push(TrialExecutionResult {
                        label: trial.label.clone(),
                        success: false,
                        detail: format!("capture failed: {reason}"),
                        snapshot_path: None,
                        duration_ms: trial_start.elapsed().as_millis() as u64,
                        power_cycle_performed,
                    });
                }
            }
        }

        // Compute pairwise diffs for successful captures
        let diff_pairs = self.plan.diff_pairs();
        let mut diffs = Vec::new();

        for (i, j) in diff_pairs {
            let label_a = &self.plan.trials[i].label;
            let label_b = &self.plan.trials[j].label;

            let snap_a = snapshots.iter().find(|(l, _)| l == label_a);
            let snap_b = snapshots.iter().find(|(l, _)| l == label_b);

            if let (Some((_, bytes_a)), Some((_, bytes_b))) = (snap_a, snap_b) {
                let changed = count_json_diffs(bytes_a, bytes_b);
                let diff_path =
                    format!("{}/diff_{}_{}.json", self.plan.output_dir, label_a, label_b);

                let _ = std::fs::write(
                    &diff_path,
                    serde_json::json!({
                        "trial_a": label_a,
                        "trial_b": label_b,
                        "changed_registers": changed,
                    })
                    .to_string(),
                );

                diffs.push(DiffSummary {
                    trial_a: label_a.clone(),
                    trial_b: label_b.clone(),
                    changed_registers: changed,
                    diff_path: Some(diff_path),
                });
            }
        }

        LabExecutionResult {
            bdf: self.plan.bdf.clone(),
            gpu_description: self.plan.gpu_description.clone(),
            trials: trial_results,
            diffs,
            total_ms: lab_start.elapsed().as_millis() as u64,
        }
    }
}

/// Count the number of differing bytes between two snapshot blobs.
/// Used as a rough "changed register count" heuristic for diff summaries.
fn count_json_diffs(a: &[u8], b: &[u8]) -> usize {
    if a.len() != b.len() {
        return a.len().max(b.len());
    }
    a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
}

impl std::fmt::Display for LabExecutionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let succeeded = self.trials.iter().filter(|t| t.success).count();
        write!(
            f,
            "DriverLab({}, {}/{} trials ok, {} diffs, {}ms)",
            self.bdf,
            succeeded,
            self.trials.len(),
            self.diffs.len(),
            self.total_ms,
        )
    }
}

#[cfg(test)]
mod tests {
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
}
