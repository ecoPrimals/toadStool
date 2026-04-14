// SPDX-License-Identifier: AGPL-3.0-or-later


#[tokio::test]
async fn pipeline_submit_empty_stages_rejected() {
    let handler = super::super::DispatchHandler::new(
        crate::visualization_client::create_visualization_client(),
    );
    let params = serde_json::json!({
        "name": "empty",
        "stages": [],
        "edges": []
    });
    let err = handler.pipeline_submit(Some(&params)).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn pipeline_submit_single_stage_passthrough() {
    let handler = super::super::DispatchHandler::new(
        crate::visualization_client::create_visualization_client(),
    );
    let params = serde_json::json!({
        "name": "single_stage",
        "stages": [{
            "id": "dispatch",
            "method": "compute.dispatch.submit",
            "params": {
                "binary": [1, 2, 3],
                "bdf": "0000:03:00.0",
                "dispatch_mode": "passthrough"
            }
        }]
    });
    let result = handler.pipeline_submit(Some(&params)).await.unwrap();
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["operation"], "pipeline.submit");
    assert!(result["job_id"].as_str().is_some());
    assert_eq!(result["metadata"]["stage_count"], 1);
    assert_eq!(result["metadata"]["stages_completed"], 1);
}

#[tokio::test]
async fn pipeline_submit_multi_stage_ordered() {
    let handler = super::super::DispatchHandler::new(
        crate::visualization_client::create_visualization_client(),
    );
    let params = serde_json::json!({
        "name": "inference_pipeline",
        "stages": [
            {
                "id": "tokenize",
                "method": "compute.dispatch.submit",
                "params": {"binary": [1, 2], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"}
            },
            {
                "id": "attention",
                "method": "compute.dispatch.submit",
                "params": {"binary": [3, 4], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"},
                "substrate": "gpu_only"
            },
            {
                "id": "ffn",
                "method": "compute.dispatch.submit",
                "params": {"binary": [5, 6], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"},
                "substrate": "gpu_only"
            }
        ],
        "edges": [["tokenize", "attention"], ["attention", "ffn"]]
    });
    let result = handler.pipeline_submit(Some(&params)).await.unwrap();
    assert_eq!(result["status"], "completed");
    assert_eq!(result["metadata"]["stage_count"], 3);
    assert_eq!(result["metadata"]["stages_completed"], 3);

    let stage_results = result["output"]["stage_results"].as_array().unwrap();
    assert_eq!(stage_results.len(), 3);
    assert_eq!(stage_results[0]["stage_id"], "tokenize");
    assert_eq!(stage_results[1]["stage_id"], "attention");
    assert_eq!(stage_results[2]["stage_id"], "ffn");
}

#[tokio::test]
async fn pipeline_status_returns_tracked_pipeline() {
    let handler = super::super::DispatchHandler::new(
        crate::visualization_client::create_visualization_client(),
    );
    let submit_params = serde_json::json!({
        "name": "tracked",
        "stages": [{
            "id": "s1",
            "method": "compute.dispatch.submit",
            "params": {"binary": [1], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"}
        }]
    });
    let submit_result = handler.pipeline_submit(Some(&submit_params)).await.unwrap();
    let pipeline_id = submit_result["job_id"].as_str().unwrap();

    let status_params = serde_json::json!({"pipeline_id": pipeline_id});
    let status = handler.pipeline_status(Some(&status_params)).await.unwrap();
    assert_eq!(status["domain"], "compute.dispatch");
    assert_eq!(status["operation"], "pipeline.status");
    assert_eq!(status["job_id"], pipeline_id);
    assert_eq!(status["status"], "completed");
}

#[tokio::test]
async fn pipeline_status_not_found() {
    let handler = super::super::DispatchHandler::new(
        crate::visualization_client::create_visualization_client(),
    );
    let params = serde_json::json!({"pipeline_id": "nonexistent"});
    let err = handler.pipeline_status(Some(&params)).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn pipeline_submit_cycle_rejected() {
    let handler = super::super::DispatchHandler::new(
        crate::visualization_client::create_visualization_client(),
    );
    let params = serde_json::json!({
        "name": "cyclic",
        "stages": [
            {"id": "a", "method": "compute.dispatch.submit", "params": {"binary": [1], "bdf": "x", "dispatch_mode": "passthrough"}},
            {"id": "b", "method": "compute.dispatch.submit", "params": {"binary": [2], "bdf": "x", "dispatch_mode": "passthrough"}}
        ],
        "edges": [["a", "b"], ["b", "a"]]
    });
    let result = handler.pipeline_submit(Some(&params)).await.unwrap();
    assert_eq!(result["status"], "failed");
    assert!(result["error"].as_str().unwrap().contains("cycle"));
}

#[tokio::test]
async fn pipeline_submit_unsupported_method_fails() {
    let handler = super::super::DispatchHandler::new(
        crate::visualization_client::create_visualization_client(),
    );
    let params = serde_json::json!({
        "name": "bad_method",
        "stages": [{
            "id": "s1",
            "method": "unknown.method",
            "params": {}
        }]
    });
    let result = handler.pipeline_submit(Some(&params)).await.unwrap();
    assert_eq!(result["status"], "partial_failure");
    let err = result["error"].as_str().unwrap();
    assert!(err.contains("Unsupported"));
}

#[tokio::test]
async fn pipeline_submit_downstream_receives_previous_results() {
    let handler = super::super::DispatchHandler::new(
        crate::visualization_client::create_visualization_client(),
    );
    let params = serde_json::json!({
        "name": "chain",
        "stages": [
            {
                "id": "first",
                "method": "compute.dispatch.submit",
                "params": {"binary": [1], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"}
            },
            {
                "id": "second",
                "method": "compute.dispatch.submit",
                "params": {"binary": [2], "bdf": "0000:03:00.0", "dispatch_mode": "passthrough"}
            }
        ],
        "edges": [["first", "second"]]
    });
    let result = handler.pipeline_submit(Some(&params)).await.unwrap();
    assert_eq!(result["status"], "completed");
    assert_eq!(result["metadata"]["stages_completed"], 2);
}
