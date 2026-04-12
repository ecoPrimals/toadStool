// SPDX-License-Identifier: AGPL-3.0-or-later
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
    assert!(
        flows.iter().any(|f| f.from == Spring::HealthSpring),
        "healthSpring should have source contributions"
    );
}
