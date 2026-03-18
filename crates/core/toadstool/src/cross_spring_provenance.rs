// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cross-spring provenance tracking for the ecoPrimals ecosystem.
//!
//! Tracks how patterns, precision techniques, and shader primitives flow
//! between springs via barraCuda and toadStool. This module is an
//! introspection API — it documents the evolution story of cross-spring
//! contributions so any primal can query which springs benefit from which.

use serde::Serialize;
use std::borrow::Cow;

/// Domain categories for spring contributions.
///
/// String representation uses SCREAMING_SNAKE_CASE per wetSpring V109
/// naming convention for cross-primal identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum SpringDomain {
    /// Numerical precision (DF64, f64, Kahan).
    Precision,
    /// Molecular dynamics simulations.
    MolecularDynamics,
    /// Lattice QCD physics.
    LatticeQcd,
    /// Condensed matter physics.
    CondensedMatter,
    /// Bioinformatics (alignment, phylogenetics).
    Bioinformatics,
    /// Machine learning and neural nets.
    MachineLearning,
    /// Hydrology and ET₀.
    Hydrology,
    /// Statistical tests and metrics.
    Statistics,
    /// Numerical stability and conditioning.
    NumericalStability,
    /// Pharmacokinetics and PK/PD.
    Pharmacokinetics,
    /// Biosignal processing.
    Biosignal,
    /// Microbiome and metagenomics.
    Microbiome,
    /// Agriculture and crop modeling.
    Agriculture,
    /// Environmental modeling.
    Environmental,
    /// Phylogenetics and evolution.
    Phylogenetics,
    /// Mass spectrometry.
    MassSpectrometry,
    /// Uncertainty quantification.
    UncertaintyQuantification,
    /// Evolutionary computation.
    EvolutionaryComputation,
    /// Optimization algorithms.
    Optimization,
}

impl SpringDomain {
    /// SCREAMING_SNAKE_CASE identifier per wetSpring V109 convention.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Precision => "PRECISION",
            Self::MolecularDynamics => "MOLECULAR_DYNAMICS",
            Self::LatticeQcd => "LATTICE_QCD",
            Self::CondensedMatter => "CONDENSED_MATTER",
            Self::Bioinformatics => "BIOINFORMATICS",
            Self::MachineLearning => "MACHINE_LEARNING",
            Self::Hydrology => "HYDROLOGY",
            Self::Statistics => "STATISTICS",
            Self::NumericalStability => "NUMERICAL_STABILITY",
            Self::Pharmacokinetics => "PHARMACOKINETICS",
            Self::Biosignal => "BIOSIGNAL",
            Self::Microbiome => "MICROBIOME",
            Self::Agriculture => "AGRICULTURE",
            Self::Environmental => "ENVIRONMENTAL",
            Self::Phylogenetics => "PHYLOGENETICS",
            Self::MassSpectrometry => "MASS_SPECTROMETRY",
            Self::UncertaintyQuantification => "UNCERTAINTY_QUANTIFICATION",
            Self::EvolutionaryComputation => "EVOLUTIONARY_COMPUTATION",
            Self::Optimization => "OPTIMIZATION",
        }
    }
}

/// A spring in the ecoPrimals ecosystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum Spring {
    /// hotSpring: lattice QCD, MD, precision.
    HotSpring,
    /// wetSpring: bioinformatics, wet-lab pipelines.
    WetSpring,
    /// neuralSpring: ML, coralForge, metalForge.
    NeuralSpring,
    /// airSpring: hydrology, IoT, ET₀.
    AirSpring,
    /// groundSpring: condensed matter, Anderson.
    GroundSpring,
    /// healthSpring: PK/PD, biosignal.
    HealthSpring,
}

impl Spring {
    /// All springs in the ecoPrimals ecosystem.
    pub const ALL: &[Self] = &[
        Self::HotSpring,
        Self::WetSpring,
        Self::NeuralSpring,
        Self::AirSpring,
        Self::GroundSpring,
        Self::HealthSpring,
    ];

    /// Returns camelCase spring name (e.g. hotSpring).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::HotSpring => "hotSpring",
            Self::WetSpring => "wetSpring",
            Self::NeuralSpring => "neuralSpring",
            Self::AirSpring => "airSpring",
            Self::GroundSpring => "groundSpring",
            Self::HealthSpring => "healthSpring",
        }
    }
}

/// A documented cross-spring contribution.
#[derive(Debug, Clone, Serialize)]
pub struct CrossSpringFlow {
    /// Source spring.
    pub from: Spring,
    /// Target springs that consume this pattern.
    pub to: &'static [Spring],
    /// Domain category.
    pub domain: SpringDomain,
    /// Pattern or kernel identifier.
    pub pattern: Cow<'static, str>,
    /// Human-readable description of the flow.
    pub description: Cow<'static, str>,
    /// Session or version when absorbed.
    pub session: Cow<'static, str>,
}

/// Build the complete cross-spring provenance registry.
///
/// Each entry documents how a pattern or technique from one spring
/// was adopted by or benefits other springs in the ecosystem.
#[must_use]
pub fn cross_spring_flows() -> Vec<CrossSpringFlow> {
    vec![
        // hotSpring precision -> ALL springs
        CrossSpringFlow {
            from: Spring::HotSpring,
            to: &[
                Spring::WetSpring,
                Spring::NeuralSpring,
                Spring::AirSpring,
                Spring::GroundSpring,
            ],
            domain: SpringDomain::Precision,
            pattern: "df64_core.wgsl + df64_transcendentals.wgsl".into(),
            description: "DF64 double-float f32-pair arithmetic (~48-bit mantissa). \
                Originated in hotSpring's lattice QCD precision requirements. \
                Kahan summation and FMA control patterns enable f64-quality \
                results on GPUs lacking native f64 support."
                .into(),
            session: "S49 (f32→f64 evolution), S71 (15 DF64 transcendentals)".into(),
        },
        CrossSpringFlow {
            from: Spring::HotSpring,
            to: &[Spring::NeuralSpring],
            domain: SpringDomain::Precision,
            pattern: "FMA control + Kahan summation → coralForge attention".into(),
            description: "hotSpring's lattice QCD precision patterns (FMA control, \
                Kahan summation for catastrophic cancellation) were adopted by \
                neuralSpring's coralForge streaming attention primitives \
                (gelu_f64, layer_norm_f64, softmax_f64, sdpa_scores_f64)."
                .into(),
            session: "coralReef Phase 10 cross-spring rewire".into(),
        },
        // hotSpring MD -> wetSpring
        CrossSpringFlow {
            from: Spring::HotSpring,
            to: &[Spring::WetSpring],
            domain: SpringDomain::MolecularDynamics,
            pattern: "stress_virial_f64.wgsl".into(),
            description: "Off-diagonal stress tensor from hotSpring MD simulations \
                is used by wetSpring for mechanical property validation in \
                bio-material pipelines."
                .into(),
            session: "S83 (cross-spring absorption)".into(),
        },
        CrossSpringFlow {
            from: Spring::HotSpring,
            to: &[Spring::WetSpring],
            domain: SpringDomain::MachineLearning,
            pattern: "esn_readout_f64.wgsl".into(),
            description: "Echo State Network readout layer from hotSpring's reservoir \
                computing, reused by wetSpring for temporal bio-signal modeling."
                .into(),
            session: "S83".into(),
        },
        CrossSpringFlow {
            from: Spring::HotSpring,
            to: &[Spring::NeuralSpring],
            domain: SpringDomain::LatticeQcd,
            pattern: "cg_kernels_f64.wgsl".into(),
            description: "Conjugate gradient solver from hotSpring's lattice QCD \
                is referenced by neuralSpring for large-scale linear system \
                solving in metalForge experiments."
                .into(),
            session: "S83".into(),
        },
        // neuralSpring statistics -> wetSpring + groundSpring
        CrossSpringFlow {
            from: Spring::NeuralSpring,
            to: &[Spring::WetSpring, Spring::GroundSpring],
            domain: SpringDomain::Statistics,
            pattern: "fused_kl_divergence_f64.wgsl".into(),
            description: "KL divergence from neuralSpring's information-theoretic \
                validation, absorbed by wetSpring for cross-entropy metrics \
                and referenced by groundSpring for Anderson model fitness scoring."
                .into(),
            session: "barraCuda provenance registry".into(),
        },
        CrossSpringFlow {
            from: Spring::NeuralSpring,
            to: &[Spring::HotSpring, Spring::WetSpring],
            domain: SpringDomain::Statistics,
            pattern: "fused_chi_squared_f64.wgsl".into(),
            description: "Chi-squared batch test from neuralSpring, consumed by \
                hotSpring for lattice QCD observable validation and wetSpring \
                for goodness-of-fit in metagenomic analysis."
                .into(),
            session: "barraCuda provenance registry".into(),
        },
        CrossSpringFlow {
            from: Spring::NeuralSpring,
            to: &[Spring::GroundSpring, Spring::HotSpring],
            domain: SpringDomain::Statistics,
            pattern: "matrix_correlation_f64.wgsl".into(),
            description: "Matrix correlation from neuralSpring, used by groundSpring \
                for Anderson model parameter correlation and hotSpring for \
                observable cross-correlation analysis."
                .into(),
            session: "barraCuda provenance registry".into(),
        },
        CrossSpringFlow {
            from: Spring::NeuralSpring,
            to: &[Spring::AirSpring],
            domain: SpringDomain::Statistics,
            pattern: "linear_regression_f64.wgsl".into(),
            description: "GPU linear regression from neuralSpring, adopted by airSpring \
                for trend analysis in ET₀ and crop coefficient calibration."
                .into(),
            session: "barraCuda provenance registry".into(),
        },
        // wetSpring bio -> neuralSpring
        CrossSpringFlow {
            from: Spring::WetSpring,
            to: &[Spring::NeuralSpring],
            domain: SpringDomain::Bioinformatics,
            pattern: "smith_waterman_banded_f64.wgsl + gillespie_ssa_f64.wgsl".into(),
            description: "Smith-Waterman sequence alignment and Gillespie SSA stochastic \
                simulation from wetSpring bioinformatics, consumed by neuralSpring \
                for neuroevolution fitness evaluation and stochastic search."
                .into(),
            session: "barraCuda provenance registry".into(),
        },
        CrossSpringFlow {
            from: Spring::WetSpring,
            to: &[Spring::AirSpring, Spring::HotSpring],
            domain: SpringDomain::Statistics,
            pattern: "fused_map_reduce_f64.wgsl".into(),
            description: "Fused map-reduce primitive from wetSpring, adopted by \
                airSpring for gridded hydrology reductions and hotSpring \
                for MD observable aggregation."
                .into(),
            session: "S83".into(),
        },
        // airSpring hydrology -> wetSpring
        CrossSpringFlow {
            from: Spring::AirSpring,
            to: &[Spring::WetSpring],
            domain: SpringDomain::Hydrology,
            pattern: "hargreaves_et0_f64.wgsl + seasonal_pipeline.wgsl".into(),
            description: "FAO56 Hargreaves ET₀ and seasonal crop pipeline from \
                airSpring hydrology, consumed by wetSpring for environmental \
                parameter estimation in bio-ecosystem models."
                .into(),
            session: "S83 (airSpring batch ops absorption)".into(),
        },
        CrossSpringFlow {
            from: Spring::AirSpring,
            to: &[Spring::WetSpring],
            domain: SpringDomain::Statistics,
            pattern: "moving_window_f64.wgsl".into(),
            description: "Moving window statistics (mean/var/min/max) from airSpring \
                IoT/streaming, consumed by wetSpring for temporal bio-signal \
                smoothing and trend detection."
                .into(),
            session: "barraCuda provenance registry".into(),
        },
        // groundSpring condensed matter -> neuralSpring + hotSpring
        CrossSpringFlow {
            from: Spring::GroundSpring,
            to: &[Spring::NeuralSpring, Spring::HotSpring],
            domain: SpringDomain::CondensedMatter,
            pattern: "anderson_lyapunov_f64.wgsl".into(),
            description: "Anderson localization Lyapunov exponent from groundSpring \
                condensed matter physics, referenced by neuralSpring for disorder \
                sweep validation in metalForge experiments and hotSpring for \
                transport property verification."
                .into(),
            session: "S83, barraCuda provenance registry".into(),
        },
        CrossSpringFlow {
            from: Spring::GroundSpring,
            to: Spring::ALL,
            domain: SpringDomain::Statistics,
            pattern: "chi_squared_f64.wgsl".into(),
            description: "Universal chi-squared test from groundSpring, consumed by \
                ALL springs for goodness-of-fit validation in their respective \
                scientific domains."
                .into(),
            session: "barraCuda provenance registry".into(),
        },
        // groundSpring precision discovery -> ALL springs via toadStool
        CrossSpringFlow {
            from: Spring::GroundSpring,
            to: Spring::ALL,
            domain: SpringDomain::NumericalStability,
            pattern: "f64 shared-memory bug → PrecisionRoutingAdvice".into(),
            description: "groundSpring V84-V85 discovered that naga/SPIR-V f64 shared-memory \
                reductions return zeros on ALL tested GPUs. This led to toadStool's \
                PrecisionRoutingAdvice enum (F64Native, F64NativeNoSharedMem, Df64Only, \
                F32Only) and f64_shared_memory_reliable flag on GpuAdapterInfo, \
                benefiting all springs' GPU dispatch decisions."
                .into(),
            session: "S128 (groundSpring V84-V85 absorption)".into(),
        },
        // healthSpring PK/PD dispatch thresholds -> ALL springs via toadStool
        CrossSpringFlow {
            from: Spring::HealthSpring,
            to: Spring::ALL,
            domain: SpringDomain::Pharmacokinetics,
            pattern: "PopulationPk/DoseResponse WorkloadPatterns".into(),
            description: "healthSpring V19 contributed PopulationPk and DoseResponse \
                workload patterns with GPU crossover thresholds validated on real \
                PK/PD datasets. Absorbed into toadStool WorkloadRouter for substrate \
                routing, available to all springs."
                .into(),
            session: "S145 (healthSpring V14.1-V19 absorption)".into(),
        },
        CrossSpringFlow {
            from: Spring::HealthSpring,
            to: &[Spring::WetSpring, Spring::NeuralSpring],
            domain: SpringDomain::Biosignal,
            pattern: "DiversityIndex WorkloadPattern".into(),
            description: "healthSpring/wetSpring co-developed DiversityIndex (Shannon/Simpson) \
                GPU dispatch pattern, absorbed into toadStool's WorkloadRouter. \
                Benefits microbiome and ecological diversity computations."
                .into(),
            session: "S145 (healthSpring V19 absorption)".into(),
        },
    ]
}

/// Compute the cross-spring matrix: counts of shared patterns from→to.
#[must_use]
pub fn cross_spring_matrix() -> Vec<(Spring, Spring, usize)> {
    let flows = cross_spring_flows();
    let mut matrix = Vec::new();

    for from in Spring::ALL {
        for to in Spring::ALL {
            if from == to {
                continue;
            }
            let count = flows
                .iter()
                .filter(|f| f.from == *from && f.to.contains(to))
                .count();
            if count > 0 {
                matrix.push((*from, *to, count));
            }
        }
    }

    matrix
}

/// Serialize the provenance data as JSON for the `toadstool.provenance` IPC method.
#[must_use]
pub fn provenance_json() -> serde_json::Value {
    let flows = cross_spring_flows();
    let matrix = cross_spring_matrix();

    serde_json::json!({
        "total_flows": flows.len(),
        "springs": Spring::ALL.iter().map(|s| s.name()).collect::<Vec<_>>(),
        "flows": flows,
        "matrix": matrix.iter().map(|(from, to, count)| {
            serde_json::json!({
                "from": from.name(),
                "to": to.name(),
                "shared_patterns": count
            })
        }).collect::<Vec<_>>(),
        "domains": [
            "PRECISION", "MOLECULAR_DYNAMICS", "LATTICE_QCD", "CONDENSED_MATTER",
            "BIOINFORMATICS", "MACHINE_LEARNING", "HYDROLOGY", "STATISTICS",
            "NUMERICAL_STABILITY", "PHARMACOKINETICS", "BIOSIGNAL", "MICROBIOME",
            "AGRICULTURE", "ENVIRONMENTAL", "PHYLOGENETICS", "MASS_SPECTROMETRY",
            "UNCERTAINTY_QUANTIFICATION", "EVOLUTIONARY_COMPUTATION", "OPTIMIZATION"
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_springs_represented_as_sources() {
        let flows = cross_spring_flows();
        for spring in Spring::ALL {
            assert!(
                flows.iter().any(|f| f.from == *spring),
                "{} should have at least one contribution",
                spring.name()
            );
        }
    }

    #[test]
    fn test_all_springs_represented_as_consumers() {
        let flows = cross_spring_flows();
        for spring in Spring::ALL {
            assert!(
                flows.iter().any(|f| f.to.contains(spring)),
                "{} should consume at least one cross-spring pattern",
                spring.name()
            );
        }
    }

    #[test]
    fn test_hotspring_precision_reaches_all() {
        let flows = cross_spring_flows();
        let df64_flow = flows
            .iter()
            .find(|f| f.pattern.contains("df64_core"))
            .expect("df64_core flow should exist");
        assert_eq!(df64_flow.from, Spring::HotSpring);
        assert!(
            df64_flow.to.len() >= 4,
            "df64 should reach at least 4 springs"
        );
    }

    #[test]
    fn test_groundspring_bug_discovery_reaches_all() {
        let flows = cross_spring_flows();
        let bug_flow = flows
            .iter()
            .find(|f| f.pattern.contains("PrecisionRoutingAdvice"))
            .expect("PrecisionRoutingAdvice flow should exist");
        assert_eq!(bug_flow.from, Spring::GroundSpring);
        assert_eq!(
            bug_flow.to.len(),
            Spring::ALL.len(),
            "bug discovery should reach all springs"
        );
    }

    #[test]
    fn test_cross_spring_matrix_non_empty() {
        let matrix = cross_spring_matrix();
        assert!(!matrix.is_empty());
        for (from, to, count) in &matrix {
            assert_ne!(from, to, "diagonal entries should be excluded");
            assert!(*count > 0);
        }
    }

    #[test]
    fn test_provenance_json_structure() {
        let json = provenance_json();
        assert!(json["total_flows"].as_u64().unwrap() > 10);
        assert_eq!(json["springs"].as_array().unwrap().len(), Spring::ALL.len());
        assert!(!json["flows"].as_array().unwrap().is_empty());
        assert!(!json["matrix"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_neuralspring_statistics_to_wetspring() {
        let flows = cross_spring_flows();
        let kl = flows
            .iter()
            .find(|f| f.pattern.contains("kl_divergence"))
            .expect("KL divergence flow should exist");
        assert_eq!(kl.from, Spring::NeuralSpring);
        assert!(kl.to.contains(&Spring::WetSpring));
    }

    #[test]
    fn test_wetspring_bio_to_neuralspring() {
        let flows = cross_spring_flows();
        let sw = flows
            .iter()
            .find(|f| f.pattern.contains("smith_waterman"))
            .expect("Smith-Waterman flow should exist");
        assert_eq!(sw.from, Spring::WetSpring);
        assert!(sw.to.contains(&Spring::NeuralSpring));
    }

    #[test]
    fn test_airspring_hydrology_to_wetspring() {
        let flows = cross_spring_flows();
        let et0 = flows
            .iter()
            .find(|f| f.pattern.contains("hargreaves"))
            .expect("ET₀ flow should exist");
        assert_eq!(et0.from, Spring::AirSpring);
        assert!(et0.to.contains(&Spring::WetSpring));
    }

    #[test]
    fn test_spring_name_lowercase_convention() {
        for spring in Spring::ALL {
            let name = spring.name();
            assert!(
                name.contains("Spring"),
                "Spring names should follow camelCase convention: {name}"
            );
        }
    }

    #[test]
    fn test_flow_count_minimum() {
        let flows = cross_spring_flows();
        assert!(
            flows.len() >= 15,
            "Should have at least 15 documented flows"
        );
    }

    #[test]
    fn test_spring_domain_as_str_screaming_snake() {
        assert_eq!(SpringDomain::Precision.as_str(), "PRECISION");
        assert_eq!(SpringDomain::LatticeQcd.as_str(), "LATTICE_QCD");
        assert_eq!(SpringDomain::Pharmacokinetics.as_str(), "PHARMACOKINETICS");
        assert_eq!(
            SpringDomain::UncertaintyQuantification.as_str(),
            "UNCERTAINTY_QUANTIFICATION"
        );
        for domain in [
            SpringDomain::Precision,
            SpringDomain::MolecularDynamics,
            SpringDomain::Bioinformatics,
            SpringDomain::Pharmacokinetics,
            SpringDomain::Optimization,
        ] {
            let s = domain.as_str();
            assert_eq!(s, s.to_uppercase(), "as_str must be SCREAMING_SNAKE_CASE");
        }
    }

    #[test]
    fn test_healthspring_flows_exist() {
        let flows = cross_spring_flows();
        let hs_flows: Vec<_> = flows
            .iter()
            .filter(|f| f.from == Spring::HealthSpring)
            .collect();
        assert!(
            !hs_flows.is_empty(),
            "healthSpring should have source contributions"
        );
    }
}
