// SPDX-License-Identifier: AGPL-3.0-or-later
//! JH-2 resource envelope enforcement tests — mem, cpu, timeout, and integrated dispatch paths.

use super::super::submit::enforce_envelope;
use super::{submit_params, test_handler};
use crate::pure_jsonrpc::handler::method_gate::{CallerContext, ResourceEnvelope};

fn envelope(
    mem_mb: Option<u64>,
    cpu_cores: Option<u32>,
    max_timeout_ms: Option<u64>,
) -> ResourceEnvelope {
    ResourceEnvelope {
        mem_mb,
        cpu_cores,
        max_timeout_ms,
        method_allowlist: vec![],
    }
}

fn ctx_with(env: ResourceEnvelope) -> CallerContext {
    CallerContext {
        identity: Some("did:key:z6Mk_test".into()),
        envelope: Some(env),
        ..CallerContext::anonymous()
    }
}

#[test]
fn no_envelope_always_passes() {
    let ctx = CallerContext::anonymous();
    assert!(enforce_envelope(&ctx, 1024 * 1024 * 100, 256, 5000).is_ok());
}

#[test]
fn envelope_without_mem_limit_passes() {
    let ctx = ctx_with(envelope(None, Some(4), None));
    assert!(enforce_envelope(&ctx, 1024 * 1024 * 500, 256, 5000).is_ok());
}

#[test]
fn envelope_mem_limit_allows_within_bounds() {
    let ctx = ctx_with(envelope(Some(100), None, None));
    assert!(enforce_envelope(&ctx, 50 * 1024 * 1024, 256, 5000).is_ok());
}

#[test]
fn envelope_mem_limit_rejects_over_bounds() {
    let ctx = ctx_with(envelope(Some(10), None, None));
    let err = enforce_envelope(&ctx, 20 * 1024 * 1024, 256, 5000).unwrap_err();
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
    );
    assert!(err.message.contains("exceeds token envelope"));
}

#[test]
fn envelope_mem_limit_boundary_exact() {
    let ctx = ctx_with(envelope(Some(1), None, None));
    assert!(enforce_envelope(&ctx, 1024 * 1024, 256, 5000).is_ok());
    assert!(enforce_envelope(&ctx, 1024 * 1024 + 1, 256, 5000).is_err());
}

#[test]
fn envelope_cpu_cores_allows_within_bounds() {
    let ctx = ctx_with(envelope(None, Some(2), None));
    assert!(enforce_envelope(&ctx, 100, 2048, 5000).is_ok());
}

#[test]
fn envelope_cpu_cores_rejects_over_bounds() {
    let ctx = ctx_with(envelope(None, Some(2), None));
    let err = enforce_envelope(&ctx, 100, 2049, 5000).unwrap_err();
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
    );
    assert!(err.message.contains("cpu_cores"));
}

#[test]
fn envelope_timeout_allows_within_bounds() {
    let ctx = ctx_with(envelope(None, None, Some(10_000)));
    assert!(enforce_envelope(&ctx, 100, 256, 10_000).is_ok());
}

#[test]
fn envelope_timeout_rejects_over_bounds() {
    let ctx = ctx_with(envelope(None, None, Some(5_000)));
    let err = enforce_envelope(&ctx, 100, 256, 5_001).unwrap_err();
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
    );
    assert!(err.message.contains("timeout"));
}

#[test]
fn envelope_all_dimensions_checked() {
    let ctx = ctx_with(envelope(Some(100), Some(4), Some(30_000)));
    assert!(enforce_envelope(&ctx, 50 * 1024 * 1024, 4096, 30_000).is_ok());
    assert!(enforce_envelope(&ctx, 200 * 1024 * 1024, 4096, 30_000).is_err());
    assert!(enforce_envelope(&ctx, 50 * 1024 * 1024, 4097, 30_000).is_err());
    assert!(enforce_envelope(&ctx, 50 * 1024 * 1024, 4096, 30_001).is_err());
}

#[tokio::test]
async fn dispatch_submit_with_context_no_envelope_succeeds() {
    let handler = test_handler();
    let params = submit_params("0000:01:00.0", "passthrough");
    let ctx = CallerContext::anonymous();
    let result = handler
        .dispatch_submit_with_context(Some(&params), &ctx)
        .await
        .expect("should succeed without envelope");
    assert!(result["job_id"].as_str().is_some());
}

#[tokio::test]
async fn dispatch_submit_with_context_envelope_allows() {
    let handler = test_handler();
    let params = submit_params("0000:01:00.0", "passthrough");
    let ctx = ctx_with(envelope(Some(100), None, None));
    let result = handler
        .dispatch_submit_with_context(Some(&params), &ctx)
        .await
        .expect("3-byte binary is well within 100 MB envelope");
    assert!(result["job_id"].as_str().is_some());
}

#[tokio::test]
async fn dispatch_submit_with_context_envelope_rejects_mem() {
    let handler = test_handler();
    let mut large_binary = vec![0u8; 2 * 1024 * 1024];
    large_binary[0] = 1;
    let params = serde_json::json!({
        "binary": large_binary,
        "bdf": "0000:01:00.0",
        "dispatch_mode": "passthrough",
    });
    let ctx = ctx_with(envelope(Some(1), None, None));
    let err = handler
        .dispatch_submit_with_context(Some(&params), &ctx)
        .await
        .unwrap_err();
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
    );
}

#[tokio::test]
async fn dispatch_submit_rejects_timeout_over_envelope() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [1, 2, 3],
        "bdf": "0000:01:00.0",
        "dispatch_mode": "passthrough",
        "timeout_ms": 60_000,
    });
    let ctx = ctx_with(envelope(None, None, Some(5_000)));
    let err = handler
        .dispatch_submit_with_context(Some(&params), &ctx)
        .await
        .unwrap_err();
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
    );
    assert!(err.message.contains("timeout"));
}

#[tokio::test]
async fn shader_dispatch_enforces_envelope() {
    let handler = test_handler();
    let mut large_binary = vec![0u8; 2 * 1024 * 1024];
    large_binary[0] = 1;
    let params = serde_json::json!({
        "binary": large_binary,
        "bdf": "0000:01:00.0",
        "dispatch_mode": "passthrough",
    });
    let ctx = ctx_with(envelope(Some(1), None, None));
    let err = handler
        .shader_dispatch_with_context(Some(&params), &ctx)
        .await
        .unwrap_err();
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED
    );
}
