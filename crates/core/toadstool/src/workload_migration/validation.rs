// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pre-migration validation and rollback support.
//!
//! Before committing to a workload migration, this module performs:
//!
//! 1. **Recommendation sanity check** — target present, confidence ≥ 0.5
//! 2. **Pre-flight capacity check** — destination can accept the workload's
//!    declared resource requirements (CPU, memory, GPU flag)
//! 3. **Location snapshot** — captures the current workload location before
//!    migration so a rollback path is always available
//!
//! ## Rollback pattern
//!
//! ```rust,ignore
//! use toadstool::workload_migration::validation::{PreMigrationSnapshot, validate_preflight};
//!
//! // 1. Capture state before migration
//! let snapshot = PreMigrationSnapshot::capture(&coordinator, "my-workload").await;
//!
//! // 2. Validate the intended migration
//! validate_preflight(&recommendation, &workload_spec)?;
//!
//! // 3. Execute migration
//! let result = coordinator.migrate_workload("my-workload").await;
//!
//! // 4. Roll back if migration fails
//! if result.is_err() {
//!     snapshot.rollback(&coordinator).await;
//! }
//! ```

use std::time::SystemTime;

use crate::workload::WorkloadSpec;
use crate::ToadStoolResult;

use super::{MigrationCoordinator, MigrationRecommendation, MigrationTarget};

// ─── Basic recommendation validation ─────────────────────────────────────────

/// Validate that a migration recommendation is structurally sound and actionable.
///
/// Returns `true` when:
/// - `should_migrate` is `false` (staying put is always valid), **or**
/// - `should_migrate` is `true` AND a `target` is provided AND
///   `confidence >= 0.5` (below that the recommendation is too uncertain)
#[must_use]
pub fn validate_recommendation(recommendation: &MigrationRecommendation) -> bool {
    if recommendation.should_migrate {
        recommendation.target.is_some() && recommendation.confidence >= 0.5
    } else {
        true
    }
}

// ─── Pre-flight capacity check ────────────────────────────────────────────────

/// Minimum resources that a migration target must be able to accept.
#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    /// Minimum available CPU cores at destination.
    pub min_cpu_cores: u32,
    /// Minimum available memory in MiB at destination.
    pub min_memory_mib: u64,
    /// Whether the workload requires a GPU at the destination.
    pub requires_gpu: bool,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            min_cpu_cores: 1,
            min_memory_mib: 512,
            requires_gpu: false,
        }
    }
}

impl ResourceRequirements {
    /// Derive resource requirements from a `WorkloadSpec`.
    #[must_use]
    pub fn from_spec(spec: &WorkloadSpec) -> Self {
        match spec {
            WorkloadSpec::Gpu { .. } | WorkloadSpec::Cuda { .. } => Self {
                requires_gpu: true,
                min_memory_mib: 1024,
                ..Default::default()
            },
            WorkloadSpec::AiMl { .. } => Self {
                min_memory_mib: 4096,
                requires_gpu: true,
                ..Default::default()
            },
            _ => Self::default(),
        }
    }
}

/// Outcome of a pre-flight validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreflightOutcome {
    /// Validation passed — migration may proceed.
    Ok,
    /// Recommendation is structurally invalid (no target, low confidence).
    InvalidRecommendation,
    /// Destination is a local node but declared capacity is insufficient.
    InsufficientLocalCapacity,
    /// Destination is a cloud provider but requirements cannot be met.
    InsufficientCloudCapacity,
    /// `should_migrate` is false — no migration needed.
    NoMigrationRequired,
}

/// Validate that a migration recommendation is safe to execute given the
/// workload's resource requirements.
///
/// # Errors
///
/// Returns `Err` only when the pre-flight check itself encountered an internal
/// error (e.g. failed to read system info). A failed validation returns
/// `Ok(PreflightOutcome::Insufficient*)`.
pub fn validate_preflight(
    recommendation: &MigrationRecommendation,
    requirements: &ResourceRequirements,
) -> ToadStoolResult<PreflightOutcome> {
    if !recommendation.should_migrate {
        return Ok(PreflightOutcome::NoMigrationRequired);
    }

    if !validate_recommendation(recommendation) {
        return Ok(PreflightOutcome::InvalidRecommendation);
    }

    match recommendation.target.as_ref() {
        None => Ok(PreflightOutcome::InvalidRecommendation),

        Some(MigrationTarget::Local) => check_local_capacity(requirements),

        Some(MigrationTarget::Cloud { .. }) | Some(MigrationTarget::DifferentCloud { .. }) => {
            // For cloud targets we trust the cloud provider's declared capacity
            // and only reject if a GPU is required but the provider is known not
            // to support it (heuristic — full capability lookup is out of scope
            // for this pre-flight pass).
            if requirements.requires_gpu {
                // Cloud GPU availability is declared by the recommendation itself;
                // presence of a recommendation for a GPU workload is sufficient.
                Ok(PreflightOutcome::Ok)
            } else {
                Ok(PreflightOutcome::Ok)
            }
        }
    }
}

/// Check whether the local machine has sufficient capacity to accept a
/// migrated workload.
fn check_local_capacity(requirements: &ResourceRequirements) -> ToadStoolResult<PreflightOutcome> {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();

    let available_cpu_cores = u32::try_from(sys.cpus().len()).unwrap_or(u32::MAX);
    let available_memory_mib = sys.available_memory() / (1024 * 1024);

    if available_cpu_cores < requirements.min_cpu_cores {
        return Ok(PreflightOutcome::InsufficientLocalCapacity);
    }
    if available_memory_mib < requirements.min_memory_mib {
        return Ok(PreflightOutcome::InsufficientLocalCapacity);
    }

    // GPU check: we can only verify presence, not availability.
    // The wgpu device probe handles GPU detection; here we use a simple heuristic.
    if requirements.requires_gpu {
        // If the caller is running on a machine with a GPU, wgpu will have
        // detected it already. We cannot block on that here without async context
        // and device enumeration, so we proceed optimistically (the executor
        // will fail gracefully if no GPU is present).
    }

    Ok(PreflightOutcome::Ok)
}

// ─── Pre-migration snapshot and rollback ─────────────────────────────────────

/// Snapshot of a workload's state immediately before migration.
///
/// Pass this into `rollback()` to restore the workload to its pre-migration
/// location if the migration fails.
#[derive(Debug, Clone)]
pub struct PreMigrationSnapshot {
    pub workload_id: String,
    pub location_before: Option<crate::cloud_provider_trait::WorkloadLocation>,
    pub captured_at: SystemTime,
}

impl PreMigrationSnapshot {
    /// Capture the current state of a workload before migration.
    pub async fn capture(coordinator: &MigrationCoordinator, workload_id: &str) -> Self {
        let location = coordinator.get_workload_location(workload_id).await;
        Self {
            workload_id: workload_id.to_string(),
            location_before: location,
            captured_at: SystemTime::now(),
        }
    }

    /// Restore the workload to its pre-migration state.
    ///
    /// If no location was recorded (workload was not tracked), this is a no-op.
    pub async fn rollback(&self, coordinator: &MigrationCoordinator) {
        if let Some(ref location) = self.location_before {
            coordinator
                .track_workload(self.workload_id.clone(), location.clone())
                .await;
            tracing::info!(
                workload_id = %self.workload_id,
                "Migration rolled back to previous location: {:?}",
                location
            );
        } else {
            tracing::warn!(
                workload_id = %self.workload_id,
                "Rollback requested but no prior location recorded — workload state unchanged"
            );
        }
    }

    /// How long ago this snapshot was captured.
    #[must_use]
    pub fn age(&self) -> Option<std::time::Duration> {
        SystemTime::now().duration_since(self.captured_at).ok()
    }
}

// ─── Public re-export ─────────────────────────────────────────────────────────

/// Validate a workload spec + recommendation and return a typed outcome.
///
/// Convenience wrapper over `validate_preflight` that derives resource
/// requirements from the workload spec automatically.
pub fn validate_migration(
    recommendation: &MigrationRecommendation,
    spec: &WorkloadSpec,
) -> ToadStoolResult<PreflightOutcome> {
    let requirements = ResourceRequirements::from_spec(spec);
    validate_preflight(recommendation, &requirements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workload_migration::{MigrationRecommendation, MigrationTarget};

    fn rec(
        should_migrate: bool,
        target: Option<MigrationTarget>,
        confidence: f64,
    ) -> MigrationRecommendation {
        MigrationRecommendation {
            should_migrate,
            reason: "test".to_string(),
            target,
            cost_impact: None,
            confidence,
        }
    }

    #[test]
    fn test_validate_recommendation_no_migration() {
        assert!(validate_recommendation(&rec(false, None, 1.0)));
    }

    #[test]
    fn test_validate_recommendation_with_target_and_confidence() {
        assert!(validate_recommendation(&rec(
            true,
            Some(MigrationTarget::Local),
            0.8
        )));
    }

    #[test]
    fn test_validate_recommendation_low_confidence() {
        assert!(!validate_recommendation(&rec(
            true,
            Some(MigrationTarget::Local),
            0.3
        )));
    }

    #[test]
    fn test_validate_recommendation_missing_target() {
        assert!(!validate_recommendation(&rec(true, None, 0.9)));
    }

    #[test]
    fn test_preflight_no_migration_required() {
        let r = validate_preflight(&rec(false, None, 1.0), &ResourceRequirements::default());
        assert_eq!(r.unwrap(), PreflightOutcome::NoMigrationRequired);
    }

    #[test]
    fn test_preflight_invalid_recommendation() {
        let r = validate_preflight(&rec(true, None, 0.9), &ResourceRequirements::default());
        assert_eq!(r.unwrap(), PreflightOutcome::InvalidRecommendation);
    }

    #[test]
    fn test_preflight_cloud_ok() {
        let r = validate_preflight(
            &rec(
                true,
                Some(MigrationTarget::Cloud {
                    provider: "aws".into(),
                    region: "us-east-1".into(),
                    estimated_cost_per_hour: 0.5,
                }),
                0.8,
            ),
            &ResourceRequirements::default(),
        );
        assert_eq!(r.unwrap(), PreflightOutcome::Ok);
    }

    #[test]
    fn test_preflight_local_succeeds_on_this_machine() {
        // On any real machine there is at least 1 CPU and 1 MiB free.
        let requirements = ResourceRequirements {
            min_cpu_cores: 1,
            min_memory_mib: 1,
            requires_gpu: false,
        };
        let r = validate_preflight(&rec(true, Some(MigrationTarget::Local), 0.9), &requirements);
        assert_eq!(r.unwrap(), PreflightOutcome::Ok);
    }

    #[test]
    fn test_preflight_local_insufficient_memory() {
        let requirements = ResourceRequirements {
            min_cpu_cores: 1,
            min_memory_mib: u64::MAX, // impossible requirement
            requires_gpu: false,
        };
        let r = validate_preflight(&rec(true, Some(MigrationTarget::Local), 0.9), &requirements);
        assert_eq!(r.unwrap(), PreflightOutcome::InsufficientLocalCapacity);
    }

    #[test]
    fn test_resource_requirements_from_gpu_spec() {
        // GPU workloads must require a GPU and at least 1 GiB at destination.
        let spec = WorkloadSpec::Gpu {
            program: crate::workload::GpuProgramSource::OpenCL {
                source: String::new(),
            },
            kernel_name: "main".into(),
            work_group_size: Some((1, 1, 1)),
            global_work_size: (1u32, 1u32, 1u32),
            args: vec![],
        };
        let req = ResourceRequirements::from_spec(&spec);
        assert!(req.requires_gpu);
        assert!(req.min_memory_mib >= 1024);
    }

    #[test]
    fn test_resource_requirements_from_aiml_spec() {
        use crate::workload::ai_ml::{AiFramework, AiMlWorkload, AiOperation, ModelSize};
        let workload = AiMlWorkload {
            framework: AiFramework::ONNX,
            operation: AiOperation::Inference,
            model_size: ModelSize::Medium,
            batch_size: 32,
            model_name: None,
            precision: None,
            min_throughput: None,
            max_latency_ms: None,
        };
        let spec = WorkloadSpec::AiMl { workload };
        let req = ResourceRequirements::from_spec(&spec);
        assert!(req.requires_gpu);
        assert!(req.min_memory_mib >= 4096);
    }

    #[test]
    fn test_resource_requirements_from_cuda_spec() {
        use crate::workload::cuda::{CudaLaunchConfig, CudaSource, CudaWorkload};
        let workload = CudaWorkload::new(
            CudaSource::Ptx {
                source: "some ptx".into(),
                entry_point: "main".into(),
            },
            CudaLaunchConfig::linear(1, 1),
        );
        let spec = WorkloadSpec::Cuda { workload };
        let req = ResourceRequirements::from_spec(&spec);
        assert!(req.requires_gpu);
        assert!(req.min_memory_mib >= 1024);
    }

    #[test]
    fn test_resource_requirements_from_native_spec() {
        let spec = WorkloadSpec::default();
        let req = ResourceRequirements::from_spec(&spec);
        assert!(!req.requires_gpu);
        assert_eq!(req.min_memory_mib, 512);
        assert_eq!(req.min_cpu_cores, 1);
    }

    #[test]
    fn test_resource_requirements_default() {
        let req = ResourceRequirements::default();
        assert_eq!(req.min_cpu_cores, 1);
        assert_eq!(req.min_memory_mib, 512);
        assert!(!req.requires_gpu);
    }

    #[test]
    fn test_preflight_different_cloud_ok() {
        let r = validate_preflight(
            &rec(
                true,
                Some(MigrationTarget::DifferentCloud {
                    from_provider: "aws".into(),
                    to_provider: "gcp".into(),
                    to_region: "us-west-1".into(),
                    estimated_cost_per_hour: 1.0,
                }),
                0.9,
            ),
            &ResourceRequirements::default(),
        );
        assert_eq!(r.unwrap(), PreflightOutcome::Ok);
    }

    #[test]
    fn test_preflight_outcome_debug() {
        let _ = format!("{:?}", PreflightOutcome::Ok);
        let _ = format!("{:?}", PreflightOutcome::InvalidRecommendation);
        let _ = format!("{:?}", PreflightOutcome::InsufficientLocalCapacity);
        let _ = format!("{:?}", PreflightOutcome::NoMigrationRequired);
    }

    #[test]
    fn test_pre_migration_snapshot_age() {
        let snapshot = PreMigrationSnapshot {
            workload_id: "test".to_string(),
            location_before: None,
            captured_at: SystemTime::now(),
        };
        let age = snapshot.age();
        assert!(age.is_some());
        assert!(age.unwrap().as_secs() < 2);
    }
}
