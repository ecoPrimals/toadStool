// SPDX-License-Identifier: AGPL-3.0-or-later
//! `LearningAdvisor` — identify teacher/student GPU pairs and learning opportunities.
//!
//! Given a fleet of GPUs (from sysmon discovery), the advisor determines which
//! GPUs can teach others. The core insight: a GPU with working compute can
//! teach a same-vendor (or even cross-vendor) GPU that lacks firmware/init.

use super::capability_gap::CapabilityGap;
use crate::distiller::{GpuArch, Vendor};
use crate::knowledge::arch_map;
use serde::{Deserialize, Serialize};
use toadstool_sysmon::FirmwareInventory;

/// A discovered GPU in the fleet with its capabilities.
#[derive(Debug, Clone)]
pub struct FleetGpu {
    /// Unique identifier (card index or PCI slot).
    pub id: String,
    /// Architecture.
    pub arch: GpuArch,
    /// Firmware inventory.
    pub firmware: FirmwareInventory,
    /// Whether compute dispatch has been verified to work.
    pub compute_works: bool,
    /// DRM driver name.
    pub driver: String,
}

/// A learning opportunity — one GPU can teach another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningOpportunity {
    /// The working GPU that will be observed.
    pub teacher: String,
    /// Architecture of the teacher GPU.
    pub teacher_arch: GpuArch,
    /// The blocked GPU that will receive the learned recipe.
    pub student: String,
    /// Architecture of the student GPU.
    pub student_arch: GpuArch,
    /// What capability the student is missing.
    pub gap: CapabilityGap,
    /// Estimated confidence that the teacher can help (0.0..1.0).
    pub confidence: f64,
    /// Whether this crosses vendor boundaries.
    pub cross_vendor: bool,
    /// Human-readable explanation.
    pub rationale: String,
}

/// `LearningAdvisor` — reasons about what can be learned from the GPU fleet.
pub struct LearningAdvisor {
    fleet: Vec<FleetGpu>,
}

impl LearningAdvisor {
    /// Create an advisor from a fleet of discovered GPUs.
    #[must_use]
    pub const fn new(fleet: Vec<FleetGpu>) -> Self {
        Self { fleet }
    }

    /// Identify all learning opportunities in the fleet.
    ///
    /// For each blocked GPU, checks if any working GPU can serve as a teacher.
    /// Returns opportunities sorted by confidence (highest first).
    #[must_use]
    pub fn opportunities(&self) -> Vec<LearningOpportunity> {
        let teachers: Vec<&FleetGpu> = self.fleet.iter().filter(|g| g.compute_works).collect();
        let students: Vec<&FleetGpu> = self.fleet.iter().filter(|g| !g.compute_works).collect();

        let mut opportunities = Vec::new();

        for student in &students {
            let gap = CapabilityGap::diagnose(&student.firmware, &student.arch);

            for teacher in &teachers {
                let cross_vendor = teacher.arch.vendor != student.arch.vendor;
                let similarity = arch_map::architecture_similarity(&teacher.arch, &student.arch);

                let confidence = if cross_vendor {
                    // Cross-vendor: only universal patterns (power, context, channel)
                    similarity * 0.5
                } else if teacher.arch.generation == student.arch.generation {
                    // Same generation: very high confidence
                    0.9
                } else {
                    // Same vendor, different generation
                    similarity * 0.8
                };

                // Skip very low confidence pairs
                if confidence < 0.05 {
                    continue;
                }

                let rationale = format_rationale(teacher, student, &gap, cross_vendor);

                opportunities.push(LearningOpportunity {
                    teacher: teacher.id.clone(),
                    teacher_arch: teacher.arch.clone(),
                    student: student.id.clone(),
                    student_arch: student.arch.clone(),
                    gap: gap.clone(),
                    confidence,
                    cross_vendor,
                    rationale,
                });
            }
        }

        opportunities.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        opportunities
    }

    /// Quick check: are there any learning opportunities?
    #[must_use]
    pub fn has_opportunities(&self) -> bool {
        let has_teachers = self.fleet.iter().any(|g| g.compute_works);
        let has_students = self.fleet.iter().any(|g| !g.compute_works);
        has_teachers && has_students
    }

    /// Get a summary of the fleet status.
    #[must_use]
    pub fn fleet_summary(&self) -> FleetSummary {
        FleetSummary {
            total_gpus: self.fleet.len(),
            working: self.fleet.iter().filter(|g| g.compute_works).count(),
            blocked: self.fleet.iter().filter(|g| !g.compute_works).count(),
            vendors: {
                let mut v: Vec<Vendor> = self.fleet.iter().map(|g| g.arch.vendor).collect();
                v.sort_by_key(|v| format!("{v:?}"));
                v.dedup();
                v
            },
        }
    }
}

/// Summary of fleet status.
#[derive(Debug, Clone)]
pub struct FleetSummary {
    /// Total GPUs discovered in the fleet.
    pub total_gpus: usize,
    /// GPUs with verified compute.
    pub working: usize,
    /// GPUs blocked (missing firmware or init).
    pub blocked: usize,
    /// Unique vendors present in the fleet.
    pub vendors: Vec<Vendor>,
}

fn format_rationale(
    teacher: &FleetGpu,
    student: &FleetGpu,
    gap: &CapabilityGap,
    cross_vendor: bool,
) -> String {
    let vendor_note = if cross_vendor {
        format!(
            "cross-vendor ({} → {}): universal init patterns only",
            teacher.arch.vendor, student.arch.vendor
        )
    } else {
        format!(
            "same vendor ({}): register-level learning possible",
            teacher.arch.vendor
        )
    };

    format!(
        "{} ({}/{}) can teach {} ({}/{}): {}. Gap: {}",
        teacher.id,
        teacher.arch.generation,
        teacher.arch.compute_class,
        student.id,
        student.arch.generation,
        student.arch.compute_class,
        vendor_note,
        gap,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gpu(id: &str, vendor: Vendor, generation: &str, cc: &str, works: bool) -> FleetGpu {
        FleetGpu {
            id: id.into(),
            arch: GpuArch {
                vendor,
                generation: generation.into(),
                chip: format!("test_{cc}"),
                compute_class: cc.into(),
            },
            firmware: if works {
                FirmwareInventory {
                    compute_viable: true,
                    ..Default::default()
                }
            } else {
                FirmwareInventory::default()
            },
            compute_works: works,
            driver: "test".into(),
        }
    }

    #[test]
    fn same_vendor_opportunity() {
        let fleet = vec![
            make_gpu("card0", Vendor::Nvidia, "Ada", "sm89", true),
            make_gpu("card1", Vendor::Nvidia, "Volta", "sm70", false),
        ];
        let advisor = LearningAdvisor::new(fleet);
        let opps = advisor.opportunities();
        assert_eq!(opps.len(), 1);
        assert!(!opps[0].cross_vendor);
        assert!(opps[0].confidence > 0.3);
    }

    #[test]
    fn cross_vendor_opportunity() {
        let fleet = vec![
            make_gpu("amd0", Vendor::Amd, "RDNA2", "gfx1030", true),
            make_gpu("nv0", Vendor::Nvidia, "Volta", "sm70", false),
        ];
        let advisor = LearningAdvisor::new(fleet);
        let opps = advisor.opportunities();
        assert_eq!(opps.len(), 1);
        assert!(opps[0].cross_vendor);
        assert!(opps[0].confidence < 0.2); // Low confidence for cross-vendor
    }

    #[test]
    fn no_opportunities_when_all_working() {
        let fleet = vec![
            make_gpu("card0", Vendor::Nvidia, "Ada", "sm89", true),
            make_gpu("card1", Vendor::Amd, "RDNA2", "gfx1030", true),
        ];
        let advisor = LearningAdvisor::new(fleet);
        assert!(!advisor.has_opportunities());
        assert!(advisor.opportunities().is_empty());
    }

    #[test]
    fn fleet_summary() {
        let fleet = vec![
            make_gpu("card0", Vendor::Nvidia, "Ada", "sm89", true),
            make_gpu("card1", Vendor::Nvidia, "Volta", "sm70", false),
            make_gpu("card2", Vendor::Amd, "RDNA2", "gfx1030", true),
        ];
        let advisor = LearningAdvisor::new(fleet);
        let summary = advisor.fleet_summary();
        assert_eq!(summary.total_gpus, 3);
        assert_eq!(summary.working, 2);
        assert_eq!(summary.blocked, 1);
    }
}
