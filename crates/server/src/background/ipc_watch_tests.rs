// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::visualization_client::create_visualization_client;

fn state(revision: u64) -> WatcherState {
    WatcherState {
        revision,
        discovery_available: true,
    }
}

#[tokio::test]
async fn revision_advances_on_response() {
    let client = create_visualization_client();
    let mut s = state(0);
    let response = serde_json::json!({"revision": 42});

    process_response(&response, &mut s, &client).await;
    assert_eq!(s.revision, 42);
}

#[tokio::test]
async fn revision_unchanged_without_field() {
    let client = create_visualization_client();
    let mut s = state(7);
    let response = serde_json::json!({"events": []});

    process_response(&response, &mut s, &client).await;
    assert_eq!(s.revision, 7);
}

#[tokio::test]
async fn registered_event_triggers_invalidation() {
    let client = create_visualization_client();
    let mut s = state(0);
    let response = serde_json::json!({
        "revision": 1,
        "events": [{"kind": "registered", "primal": "coralReef"}]
    });

    let invalidated = process_response(&response, &mut s, &client).await;
    assert!(invalidated);
    assert_eq!(s.revision, 1);
}

#[tokio::test]
async fn non_registered_event_does_not_invalidate() {
    let client = create_visualization_client();
    let mut s = state(0);
    let response = serde_json::json!({
        "revision": 2,
        "events": [{"kind": "unregistered", "primal": "coralReef"}]
    });

    let invalidated = process_response(&response, &mut s, &client).await;
    assert!(!invalidated);
    assert_eq!(s.revision, 2);
}

#[tokio::test]
async fn empty_events_no_invalidation() {
    let client = create_visualization_client();
    let mut s = state(0);
    let response = serde_json::json!({"revision": 3, "events": []});

    let invalidated = process_response(&response, &mut s, &client).await;
    assert!(!invalidated);
    assert_eq!(s.revision, 3);
}

#[tokio::test]
async fn missing_events_key_no_panic() {
    let client = create_visualization_client();
    let mut s = state(0);
    let response = serde_json::json!({"revision": 5});

    let invalidated = process_response(&response, &mut s, &client).await;
    assert!(!invalidated);
    assert_eq!(s.revision, 5);
}

#[tokio::test]
async fn multiple_events_invalidates_on_registered() {
    let client = create_visualization_client();
    let mut s = state(0);
    let response = serde_json::json!({
        "revision": 10,
        "events": [
            {"kind": "heartbeat", "primal": "songBird"},
            {"kind": "registered", "primal": "coralReef"},
            {"kind": "unregistered", "primal": "barracuda"}
        ]
    });

    let invalidated = process_response(&response, &mut s, &client).await;
    assert!(invalidated);
}

#[tokio::test]
async fn events_without_kind_default_to_unknown() {
    let client = create_visualization_client();
    let mut s = state(0);
    let response = serde_json::json!({
        "revision": 1,
        "events": [{"primal": "coralReef"}]
    });

    let invalidated = process_response(&response, &mut s, &client).await;
    assert!(!invalidated);
}

#[test]
fn constants_are_sane() {
    assert!(POLL_INTERVAL < POLL_INTERVAL_NO_DISCOVERY);
    assert!(!SHADER_CAPABILITY.is_empty());
}
