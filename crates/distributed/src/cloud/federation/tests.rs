// SPDX-License-Identifier: AGPL-3.0-or-later
use super::policy::MIN_HEARTBEAT_INTERVAL_SECS;
use super::*;
use crate::cloud::ReplicaStatus;

use super::super::types::{DataReplica, FederationConfig, FederationNode, TopologyType};

fn make_config(id: &str) -> FederationConfig {
    FederationConfig {
        federation_id: id.to_string(),
        discovery_endpoints: vec!["https://discovery.example.com".to_string()],
        trust_anchors: vec!["anchor-1".to_string()],
    }
}

fn make_node(id: &str, provider: &str) -> FederationNode {
    FederationNode {
        id: id.to_string(),
        provider: provider.to_string(),
        region: "us-east-1".to_string(),
        capabilities: vec!["compute".to_string()],
    }
}

fn make_replica(id: &str, location: &str) -> DataReplica {
    DataReplica {
        id: id.to_string(),
        location: location.to_string(),
        status: ReplicaStatus::Synced,
    }
}

#[tokio::test]
async fn test_new_federation_manager_is_empty() {
    let mgr = CloudFederationManager::new(make_config("fed-001"))
        .await
        .unwrap();
    assert_eq!(mgr.federation_id(), "fed-001");
    assert_eq!(mgr.node_ids().count(), 0);
    assert_eq!(mgr.replica_count(), 0);
    assert_eq!(mgr.member_count(), 0);
}

#[tokio::test]
async fn test_add_node_increases_count() {
    let mut mgr = CloudFederationManager::new(make_config("fed-002"))
        .await
        .unwrap();
    mgr.add_node(make_node("node-a", "aws"), vec![]).unwrap();
    assert_eq!(mgr.node_ids().count(), 1);
    assert_eq!(mgr.member_count(), 1);

    mgr.add_node(make_node("node-b", "gcp"), vec![]).unwrap();
    assert_eq!(mgr.node_ids().count(), 2);
    assert_eq!(mgr.member_count(), 2);
}

#[tokio::test]
async fn test_add_node_duplicate_fails() {
    let mut mgr = CloudFederationManager::new(make_config("fed-dup"))
        .await
        .unwrap();
    mgr.add_node(make_node("node-a", "aws"), vec![]).unwrap();
    let res = mgr.add_node(make_node("node-a", "gcp"), vec![]);
    assert!(res.is_err());
}

#[tokio::test]
async fn test_remove_node() {
    let mut mgr = CloudFederationManager::new(make_config("fed-rm"))
        .await
        .unwrap();
    mgr.add_node(make_node("node-a", "aws"), vec![]).unwrap();
    mgr.remove_node("node-a").unwrap();
    assert_eq!(mgr.member_count(), 0);
    assert_eq!(mgr.node_ids().count(), 0);
}

#[tokio::test]
async fn test_remove_non_member_fails() {
    let mut mgr = CloudFederationManager::new(make_config("fed-rm2"))
        .await
        .unwrap();
    let res = mgr.remove_node("nonexistent");
    assert!(res.is_err());
}

#[tokio::test(start_paused = true)]
async fn test_heartbeat_keeps_member_alive() {
    let mut mgr = CloudFederationManager::new(make_config("fed-hb"))
        .await
        .unwrap();
    mgr.add_node(make_node("node-a", "aws"), vec![]).unwrap();
    assert!(mgr.is_member_alive("node-a"));

    tokio::time::advance(std::time::Duration::from_secs(2)).await;
    mgr.record_heartbeat("node-a").unwrap();
    assert!(mgr.is_member_alive("node-a"));
}

#[tokio::test]
async fn test_heartbeat_non_member_fails() {
    let mut mgr = CloudFederationManager::new(make_config("fed-hb2"))
        .await
        .unwrap();
    let res = mgr.record_heartbeat("nonexistent");
    assert!(res.is_err());
}

#[tokio::test]
async fn test_capability_exchange() {
    let mut mgr = CloudFederationManager::new(make_config("fed-cap"))
        .await
        .unwrap();
    mgr.add_node(
        FederationNode {
            id: "n1".to_string(),
            provider: "aws".to_string(),
            region: "us-east-1".to_string(),
            capabilities: vec!["compute".to_string(), "gpu".to_string()],
        },
        vec![],
    )
    .unwrap();
    mgr.add_node(
        FederationNode {
            id: "n2".to_string(),
            provider: "gcp".to_string(),
            region: "us-west-1".to_string(),
            capabilities: vec!["compute".to_string(), "storage".to_string()],
        },
        vec![],
    )
    .unwrap();

    let caps = mgr.get_federation_capabilities();
    assert!(caps.get("compute").map(|v| v.len() == 2).unwrap_or(false));
    assert!(caps.contains_key("gpu"));
    assert!(caps.contains_key("storage"));
}

#[tokio::test]
async fn test_discover_nodes_unreachable_endpoints() {
    let mgr = CloudFederationManager::new(make_config("fed-disc"))
        .await
        .unwrap();
    let res = mgr.discover_nodes().await;
    assert!(res.is_ok());
    // Unreachable endpoints produce empty results, not errors
    assert!(res.unwrap().is_empty());
}

#[tokio::test]
async fn test_discover_nodes_no_endpoints() {
    let config = FederationConfig {
        federation_id: "fed-empty".to_string(),
        discovery_endpoints: vec![],
        trust_anchors: vec![],
    };
    let mgr = CloudFederationManager::new(config).await.unwrap();
    let res = mgr.discover_nodes().await.unwrap();
    assert!(res.is_empty());
}

#[tokio::test]
async fn test_add_node_ids_are_accessible() {
    let mut mgr = CloudFederationManager::new(make_config("fed-003"))
        .await
        .unwrap();
    mgr.add_node(make_node("alpha", "aws"), vec![]).unwrap();
    mgr.add_node(make_node("beta", "azure"), vec![]).unwrap();

    let ids: Vec<&str> = mgr.node_ids().collect();
    assert!(ids.contains(&"alpha"));
    assert!(ids.contains(&"beta"));
}

#[tokio::test]
async fn test_register_replica_increases_count() {
    let mut mgr = CloudFederationManager::new(make_config("fed-005"))
        .await
        .unwrap();
    assert_eq!(mgr.replica_count(), 0);

    mgr.register_replica(make_replica("replica-1", "us-east-1"));
    assert_eq!(mgr.replica_count(), 1);

    mgr.register_replica(make_replica("replica-2", "eu-west-1"));
    assert_eq!(mgr.replica_count(), 2);
}

#[tokio::test]
async fn test_topology_type_defaults_to_centralized() {
    let mgr = CloudFederationManager::new(make_config("fed-007"))
        .await
        .unwrap();
    assert!(matches!(mgr.topology_type(), TopologyType::Centralized));
}

#[tokio::test]
async fn test_federation_id_round_trip() {
    let id = "my-unique-federation-42";
    let mgr = CloudFederationManager::new(make_config(id)).await.unwrap();
    assert_eq!(mgr.federation_id(), id);
}

#[tokio::test]
async fn test_add_node_empty_id_fails() {
    let mut mgr = CloudFederationManager::new(make_config("fed-empty"))
        .await
        .unwrap();
    let node = FederationNode {
        id: String::new(),
        provider: "aws".to_string(),
        region: "us-east-1".to_string(),
        capabilities: vec!["compute".to_string()],
    };
    let res = mgr.add_node(node, vec![]);
    assert!(res.is_err());
}

#[tokio::test]
async fn test_get_member_capabilities_returns_capabilities() {
    let mut mgr = CloudFederationManager::new(make_config("fed-caps"))
        .await
        .unwrap();
    mgr.add_node(
        FederationNode {
            id: "node-x".to_string(),
            provider: "gcp".to_string(),
            region: "us-west-1".to_string(),
            capabilities: vec!["compute".to_string(), "storage".to_string()],
        },
        vec![],
    )
    .unwrap();

    let caps = mgr.get_member_capabilities("node-x").unwrap();
    assert_eq!(caps.len(), 2);
    assert!(caps.contains(&"compute".to_string()));
    assert!(caps.contains(&"storage".to_string()));
}

#[tokio::test]
async fn test_get_member_capabilities_non_member_fails() {
    let mgr = CloudFederationManager::new(make_config("fed-caps2"))
        .await
        .unwrap();
    let res = mgr.get_member_capabilities("nonexistent");
    assert!(res.is_err());
}

#[tokio::test(start_paused = true)]
async fn test_record_heartbeat_with_capabilities_updates_caps() {
    let mut mgr = CloudFederationManager::new(make_config("fed-hbc"))
        .await
        .unwrap();
    mgr.add_node(make_node("node-hb", "aws"), vec![]).unwrap();
    tokio::time::advance(std::time::Duration::from_secs(2)).await;
    mgr.record_heartbeat_with_capabilities("node-hb", vec!["gpu".to_string()])
        .unwrap();

    let caps = mgr.get_member_capabilities("node-hb").unwrap();
    assert_eq!(caps, vec!["gpu".to_string()]);
}

#[tokio::test]
async fn test_set_heartbeat_timeout() {
    let mut mgr = CloudFederationManager::new(make_config("fed-timeout"))
        .await
        .unwrap();
    mgr.set_heartbeat_timeout(120);
    mgr.add_node(make_node("n1", "aws"), vec![]).unwrap();
    assert!(mgr.is_member_alive("n1"));
}

#[tokio::test]
async fn test_replication_factor() {
    let mgr = CloudFederationManager::new(make_config("fed-repl"))
        .await
        .unwrap();
    let factor = mgr.replication_factor();
    assert!(factor <= 10); // Default config should have reasonable factor
}

#[tokio::test]
async fn test_is_network_encrypted() {
    let mgr = CloudFederationManager::new(make_config("fed-net"))
        .await
        .unwrap();
    let _ = mgr.is_network_encrypted();
}

#[tokio::test]
async fn test_record_heartbeat_immediately_rate_limited() {
    let mut mgr = CloudFederationManager::new(make_config("fed-rl"))
        .await
        .unwrap();
    mgr.add_node(make_node("n1", "aws"), vec![]).unwrap();
    let err = mgr.record_heartbeat("n1").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("rate limit") || msg.contains("Heartbeat"),
        "unexpected error: {msg}"
    );
    assert!(msg.contains('1') || msg.contains("interval"));
}

#[tokio::test(start_paused = true)]
async fn test_is_member_alive_false_when_stale() {
    let mut mgr = CloudFederationManager::new(make_config("fed-stale"))
        .await
        .unwrap();
    mgr.add_node(make_node("n1", "aws"), vec![]).unwrap();
    assert!(mgr.is_member_alive("n1"));
    tokio::time::advance(std::time::Duration::from_secs(
        DEFAULT_HEARTBEAT_TIMEOUT_SECS + 1,
    ))
    .await;
    assert!(!mgr.is_member_alive("n1"));
}

#[tokio::test(start_paused = true)]
async fn test_alive_members_excludes_stale() {
    let mut mgr = CloudFederationManager::new(make_config("fed-alive"))
        .await
        .unwrap();
    mgr.add_node(make_node("fresh", "aws"), vec![]).unwrap();
    mgr.add_node(make_node("stale", "gcp"), vec![]).unwrap();
    tokio::time::advance(std::time::Duration::from_secs(
        DEFAULT_HEARTBEAT_TIMEOUT_SECS + 1,
    ))
    .await;
    assert!(mgr.alive_members().is_empty());
    mgr.record_heartbeat("fresh").unwrap();
    let mut alive = mgr.alive_members();
    alive.sort();
    assert_eq!(alive, vec!["fresh".to_string()]);
}

#[tokio::test(start_paused = true)]
async fn test_get_federation_capabilities_aggregates_only_alive() {
    let mut mgr = CloudFederationManager::new(make_config("fed-agg"))
        .await
        .unwrap();
    mgr.add_node(
        FederationNode {
            id: "a".to_string(),
            provider: "aws".to_string(),
            region: "r1".to_string(),
            capabilities: vec!["compute".to_string()],
        },
        vec![],
    )
    .unwrap();
    mgr.add_node(
        FederationNode {
            id: "b".to_string(),
            provider: "gcp".to_string(),
            region: "r2".to_string(),
            capabilities: vec!["storage".to_string()],
        },
        vec![],
    )
    .unwrap();
    tokio::time::advance(std::time::Duration::from_secs(
        DEFAULT_HEARTBEAT_TIMEOUT_SECS + 1,
    ))
    .await;
    assert!(mgr.get_federation_capabilities().is_empty());
    mgr.record_heartbeat("a").unwrap();
    let caps = mgr.get_federation_capabilities();
    assert_eq!(caps.get("compute").map(Vec::len), Some(1));
    assert!(!caps.contains_key("storage"));
}

#[tokio::test]
async fn test_is_member_alive_unknown_node_false() {
    let mgr = CloudFederationManager::new(make_config("fed-ghost"))
        .await
        .unwrap();
    assert!(!mgr.is_member_alive("no-such-node"));
}

#[tokio::test]
async fn test_set_heartbeat_timeout_zero_all_considered_stale() {
    let mut mgr = CloudFederationManager::new(make_config("fed-t0"))
        .await
        .unwrap();
    mgr.set_heartbeat_timeout(0);
    mgr.add_node(make_node("n1", "aws"), vec![]).unwrap();
    assert!(!mgr.is_member_alive("n1"));
    assert!(mgr.alive_members().is_empty());
}

#[tokio::test]
async fn test_register_replica_same_id_replaces() {
    let mut mgr = CloudFederationManager::new(make_config("fed-repl-dup"))
        .await
        .unwrap();
    mgr.register_replica(make_replica("r1", "us-east-1"));
    mgr.register_replica(DataReplica {
        id: "r1".to_string(),
        location: "eu-west-1".to_string(),
        status: ReplicaStatus::Syncing,
    });
    assert_eq!(mgr.replica_count(), 1);
}

#[tokio::test]
async fn test_default_replication_factor_and_network_encryption() {
    let mgr = CloudFederationManager::new(make_config("fed-def"))
        .await
        .unwrap();
    assert_eq!(mgr.replication_factor(), 0);
    assert!(!mgr.is_network_encrypted());
}

#[tokio::test]
async fn test_federation_id_accepts_uuid_like_string() {
    let id = "550e8400-e29b-41d4-a716-446655440000";
    let mgr = CloudFederationManager::new(make_config(id)).await.unwrap();
    assert_eq!(mgr.federation_id(), id);
}

#[tokio::test]
async fn test_federation_error_display_variants() {
    let cases: Vec<(FederationError, &str)> = vec![
        (
            FederationError::NotAMember {
                node_id: "x".into(),
            },
            "not a federation member",
        ),
        (
            FederationError::AlreadyMember {
                node_id: "y".into(),
            },
            "already a member",
        ),
        (
            FederationError::DiscoveryNotImplemented("peek".into()),
            "Discovery not yet implemented",
        ),
        (
            FederationError::CrossFederationNotImplemented("join".into()),
            "Cross-federation coordination not yet implemented",
        ),
        (
            FederationError::MemberStale {
                node_id: "z".into(),
                timeout_secs: 30,
            },
            "heartbeat within timeout",
        ),
        (
            FederationError::HeartbeatRateLimited {
                min_interval_secs: MIN_HEARTBEAT_INTERVAL_SECS,
            },
            "Heartbeat rate limit",
        ),
        (FederationError::InvalidNode("bad".into()), "Invalid node"),
    ];
    for (err, needle) in cases {
        let s = err.to_string();
        assert!(
            s.contains(needle),
            "expected {needle:?} in message, got: {s}"
        );
    }
}

#[tokio::test]
async fn test_add_duplicate_and_remove_errors_surface_reason() {
    let mut mgr = CloudFederationManager::new(make_config("fed-err-msg"))
        .await
        .unwrap();
    mgr.add_node(make_node("only", "aws"), vec![]).unwrap();
    let dup = mgr
        .add_node(make_node("only", "gcp"), vec![])
        .unwrap_err()
        .to_string();
    assert!(dup.contains("already a member"));
    let rm = mgr.remove_node("missing").unwrap_err().to_string();
    assert!(rm.contains("not a federation member"));
}
