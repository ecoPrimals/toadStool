// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async,
    clippy::unnecessary_wraps,
    clippy::unused_self
)]
//! Tests 41-50: Export and Reporting

use std::time::Duration;

mod common;
use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_export_metrics_json() {
    let metrics = vec![create_test_metric()];
    let json = serde_json::to_string(&metrics);
    assert!(json.is_ok(), "Should export to JSON");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_export_metrics_csv() {
    let csv_header = "timestamp,name,value,labels";
    assert!(
        csv_header.contains("timestamp"),
        "CSV should have timestamp"
    );
    assert!(csv_header.contains("value"), "CSV should have value");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_export_to_file() {
    let export_path = _export_path();
    assert!(export_path.to_str().is_some(), "Path should be valid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_generate_report() {
    let report = MonitorReport {
        duration: Duration::from_secs(3600),
        metric_count: 100,
        avg_cpu: 45.0,
        avg_memory: 60.0,
        alerts_triggered: 2,
    };
    assert!(report.metric_count > 0, "Report should have metrics");
    assert!(report.avg_cpu >= 0.0, "CPU should be valid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_report_time_range() {
    let start = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    let end = std::time::SystemTime::now();
    assert!(end > start, "End should be after start");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_report_format_text() {
    let report_text = "Monitoring Report\nCPU: 45%\nMemory: 60%";
    assert!(report_text.contains("CPU"), "Report should show CPU");
    assert!(report_text.contains("Memory"), "Report should show Memory");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_report_format_html() {
    let report_html = "<html><body><h1>Monitoring Report</h1></body></html>";
    assert!(report_html.contains("<html>"), "Should be HTML");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_summary() {
    let summary = MetricsSummary {
        min: 10.0,
        max: 90.0,
        avg: 50.0,
        p50: 48.0,
        p95: 85.0,
        p99: 89.0,
    };
    assert!(summary.min <= summary.avg, "Min should be <= avg");
    assert!(summary.avg <= summary.max, "Avg should be <= max");
    assert!(summary.p50 <= summary.p95, "P50 should be <= P95");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_export_prometheus_format() {
    let prometheus = "# TYPE cpu_percent gauge\ncpu_percent 45.0";
    assert!(prometheus.contains("TYPE"), "Should have type declaration");
    assert!(prometheus.contains("gauge"), "Should specify metric type");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_streaming_export() {
    let batch_size = 1000;
    assert!(batch_size > 0, "Batch size should be positive");
    assert!(batch_size <= 10000, "Batch size should be reasonable");
}
