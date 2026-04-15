// SPDX-License-Identifier: AGPL-3.0-or-later
use toadstool_auto_config::intelligent::UsageLearner;

#[test]
fn test_usage_learner_new() {
    let learner = UsageLearner::new();
    let _ = learner;
}

#[test]
fn test_usage_learner_default() {
    let learner = UsageLearner::default();
    let _ = learner;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_usage_learner_analyze_environment() {
    let mut learner = UsageLearner::new();

    let result = learner.analyze_environment().await;

    assert!(result.is_ok());

    if let Ok(hints) = result {
        assert!(hints.expected_cpu_usage >= 0.0 && hints.expected_cpu_usage <= 1.0);
        assert!(hints.expected_memory_usage >= 0.0 && hints.expected_memory_usage <= 1.0);
    }
}
