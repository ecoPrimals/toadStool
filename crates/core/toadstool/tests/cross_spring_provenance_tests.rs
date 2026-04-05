// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests for cross-spring provenance tracking.

use toadstool::cross_spring_provenance::{
    Spring, SpringDomain, cross_spring_flows, cross_spring_matrix, provenance_json,
};

#[test]
fn test_every_spring_contributes() {
    let flows = cross_spring_flows();
    for spring in Spring::ALL {
        let count = flows.iter().filter(|f| f.from == *spring).count();
        assert!(
            count >= 1,
            "{} should contribute at least 1 cross-spring flow, got {count}",
            spring.name()
        );
    }
}

#[test]
fn test_every_spring_consumes() {
    let flows = cross_spring_flows();
    for spring in Spring::ALL {
        let count = flows.iter().filter(|f| f.to.contains(spring)).count();
        assert!(
            count >= 1,
            "{} should consume at least 1 cross-spring flow, got {count}",
            spring.name()
        );
    }
}

#[test]
fn test_hotspring_has_most_contributions() {
    let flows = cross_spring_flows();
    let hot_count = flows.iter().filter(|f| f.from == Spring::HotSpring).count();
    for spring in Spring::ALL {
        if *spring == Spring::HotSpring {
            continue;
        }
        let count = flows.iter().filter(|f| f.from == *spring).count();
        assert!(
            hot_count >= count,
            "hotSpring ({hot_count}) should have >= contributions than {} ({count})",
            spring.name()
        );
    }
}

#[test]
fn test_precision_domain_present() {
    let flows = cross_spring_flows();
    assert!(
        flows.iter().any(|f| f.domain == SpringDomain::Precision),
        "Precision domain should have at least one flow"
    );
}

#[test]
fn test_numerical_stability_domain_present() {
    let flows = cross_spring_flows();
    assert!(
        flows
            .iter()
            .any(|f| f.domain == SpringDomain::NumericalStability),
        "NumericalStability domain should have at least one flow"
    );
}

#[test]
fn test_matrix_covers_multiple_pairs() {
    let matrix = cross_spring_matrix();
    assert!(
        matrix.len() >= 8,
        "Cross-spring matrix should have at least 8 source->target pairs, got {}",
        matrix.len()
    );
}

#[test]
fn test_matrix_excludes_self_references() {
    let matrix = cross_spring_matrix();
    for (from, to, _) in &matrix {
        assert_ne!(from, to, "Matrix should exclude self-references");
    }
}

#[test]
fn test_hotspring_to_wetspring_flow() {
    let matrix = cross_spring_matrix();
    let flow = matrix
        .iter()
        .find(|(from, to, _)| *from == Spring::HotSpring && *to == Spring::WetSpring);
    assert!(
        flow.is_some(),
        "hotSpring -> wetSpring flow should exist in matrix"
    );
    let (_, _, count) = flow.unwrap();
    assert!(*count >= 2, "hotSpring -> wetSpring should have >= 2 flows");
}

#[test]
fn test_provenance_json_is_valid() {
    let json = provenance_json();
    let text = serde_json::to_string_pretty(&json).unwrap();
    assert!(text.len() > 500, "Provenance JSON should be substantial");
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.is_object());
}

#[test]
fn test_provenance_json_contains_domains() {
    let json = provenance_json();
    let domains = json["domains"].as_array().expect("domains array");
    assert!(domains.len() >= 8);
    let domain_strs: Vec<&str> = domains.iter().filter_map(|v| v.as_str()).collect();
    assert!(domain_strs.contains(&"PRECISION"));
    assert!(domain_strs.contains(&"NUMERICAL_STABILITY"));
    assert!(domain_strs.contains(&"BIOINFORMATICS"));
}

#[test]
fn test_every_flow_has_session() {
    let flows = cross_spring_flows();
    for flow in &flows {
        assert!(
            !flow.session.is_empty(),
            "Flow '{}' should have a session reference",
            flow.pattern
        );
    }
}

#[test]
fn test_every_flow_has_description() {
    let flows = cross_spring_flows();
    for flow in &flows {
        assert!(
            flow.description.len() >= 20,
            "Flow '{}' description too short: '{}'",
            flow.pattern,
            flow.description
        );
    }
}

#[test]
fn test_groundspring_chi_squared_reaches_all() {
    let flows = cross_spring_flows();
    let chi = flows
        .iter()
        .find(|f| f.from == Spring::GroundSpring && f.pattern == "chi_squared_f64.wgsl")
        .expect("groundSpring chi-squared should exist");
    assert_eq!(
        chi.to.len(),
        Spring::ALL.len(),
        "chi-squared should reach all springs"
    );
}
