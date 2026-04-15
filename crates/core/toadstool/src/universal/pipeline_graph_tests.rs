// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn compute_triangle_is_valid() {
    let g = compute_triangle_pipeline();
    assert!(g.validate().is_ok());
    assert_eq!(g.stage_count(), 3);
    assert_eq!(g.edge_count(), 2);
}

#[test]
fn compute_triangle_order() {
    let g = compute_triangle_pipeline();
    let order = g.execute_order().expect("valid DAG");
    assert_eq!(order, vec!["discover", "compile", "dispatch"]);
}

#[test]
fn empty_graph_is_valid() {
    let g = PipelineGraph::new("empty");
    assert!(g.validate().is_ok());
    assert_eq!(g.stage_count(), 0);
    assert_eq!(g.execute_order(), Some(vec![]));
}

#[test]
fn cycle_detected() {
    let mut g = PipelineGraph::new("cycle");
    g.add_stage(StageNode {
        id: "a".into(),
        capability: "x".into(),
        substrate: Substrate::Any,
        label: "A".into(),
    });
    g.add_stage(StageNode {
        id: "b".into(),
        capability: "y".into(),
        substrate: Substrate::Any,
        label: "B".into(),
    });
    g.add_edge("a", "b");
    g.add_edge("b", "a");
    assert!(g.validate().is_err());
    assert!(g.execute_order().is_none());
}

#[test]
fn duplicate_stage_id_rejected() {
    let mut g = PipelineGraph::new("dup");
    g.add_stage(StageNode {
        id: "x".into(),
        capability: "c".into(),
        substrate: Substrate::CpuOnly,
        label: "X".into(),
    });
    g.add_stage(StageNode {
        id: "x".into(),
        capability: "d".into(),
        substrate: Substrate::CpuOnly,
        label: "X2".into(),
    });
    assert!(g.validate().unwrap_err().contains("duplicate"));
}

#[test]
fn edge_to_unknown_stage_rejected() {
    let mut g = PipelineGraph::new("bad edge");
    g.add_stage(StageNode {
        id: "a".into(),
        capability: "x".into(),
        substrate: Substrate::CpuOnly,
        label: "A".into(),
    });
    g.add_edge("a", "nonexistent");
    assert!(g.validate().unwrap_err().contains("unknown"));
}

#[test]
fn diamond_dag() {
    let mut g = PipelineGraph::new("diamond");
    for id in ["a", "b", "c", "d"] {
        g.add_stage(StageNode {
            id: id.into(),
            capability: format!("cap.{id}"),
            substrate: Substrate::Any,
            label: id.to_uppercase(),
        });
    }
    g.add_edge("a", "b");
    g.add_edge("a", "c");
    g.add_edge("b", "d");
    g.add_edge("c", "d");
    assert!(g.validate().is_ok());
    let order = g.execute_order().expect("valid DAG");
    assert_eq!(order[0], "a");
    assert_eq!(*order.last().unwrap(), "d");
}

#[test]
fn pipeline_execution_tracking() {
    let mut exec = PipelineExecution::new("test");
    assert!(!exec.all_passed());
    assert_eq!(exec.completed_count(), 0);

    exec.record(StageResult {
        stage_id: "a".into(),
        success: true,
        elapsed_us: 100.0,
        actual_substrate: Substrate::CpuOnly,
        output: StageOutput::Scalar(42.0),
    });
    exec.record(StageResult {
        stage_id: "b".into(),
        success: true,
        elapsed_us: 200.0,
        actual_substrate: Substrate::GpuOnly,
        output: StageOutput::Empty,
    });

    assert!(exec.all_passed());
    assert_eq!(exec.completed_count(), 2);
    assert_eq!(exec.failed_count(), 0);
    assert!((exec.total_elapsed_us() - 300.0).abs() < f64::EPSILON);
}

#[test]
fn pipeline_execution_with_failure() {
    let mut exec = PipelineExecution::new("test");
    exec.record(StageResult {
        stage_id: "a".into(),
        success: true,
        elapsed_us: 50.0,
        actual_substrate: Substrate::CpuOnly,
        output: StageOutput::Empty,
    });
    exec.record(StageResult {
        stage_id: "b".into(),
        success: false,
        elapsed_us: 10.0,
        actual_substrate: Substrate::GpuOnly,
        output: StageOutput::Empty,
    });
    assert!(!exec.all_passed());
    assert_eq!(exec.failed_count(), 1);
}

#[test]
fn stage_output_variants() {
    let scalar = StageOutput::Scalar(std::f64::consts::PI);
    assert!(matches!(scalar, StageOutput::Scalar(v) if (v - std::f64::consts::PI).abs() < 1e-10));

    let vec_out = StageOutput::Vector(vec![1.0, 2.0, 3.0]);
    assert!(matches!(vec_out, StageOutput::Vector(ref v) if v.len() == 3));

    let mut map = HashMap::new();
    map.insert("ipr".to_string(), 0.25);
    let map_out = StageOutput::Map(map);
    assert!(matches!(map_out, StageOutput::Map(ref m) if m.contains_key("ipr")));
}

#[test]
fn stage_lookup() {
    let g = compute_triangle_pipeline();
    let compile = g.stage("compile").expect("has compile stage");
    assert_eq!(compile.capability, "shader.compile");
    assert!(g.stage("nonexistent").is_none());
}

#[test]
fn stages_and_edges_accessors() {
    let g = compute_triangle_pipeline();
    assert_eq!(g.stages().len(), 3);
    assert_eq!(g.edges().len(), 2);
}
