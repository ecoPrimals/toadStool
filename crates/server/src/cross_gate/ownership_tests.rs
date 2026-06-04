// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(test)]
mod tests {
    use super::super::GateOwnership;
    use std::sync::Arc;

    #[tokio::test]
    async fn defaults_hardware_owner_to_local_gate() {
        let ownership = GateOwnership::new("tower");
        assert_eq!(ownership.local_gate_id.as_ref(), "tower");
        assert_eq!(ownership.hardware_owner_gate_id().await.as_ref(), "tower");
    }

    #[tokio::test]
    async fn note_gate_update_sets_remote_owner() {
        let ownership = GateOwnership::new("guest");
        ownership
            .note_gate_update(&Arc::from("tower"), true)
            .await;
        assert_eq!(ownership.hardware_owner_gate_id().await.as_ref(), "tower");
        assert!(ownership.caller_is_hardware_owner(Some("tower")).await);
        assert!(!ownership.caller_is_hardware_owner(Some("guest")).await);
    }

    #[tokio::test]
    async fn anonymous_caller_is_not_hardware_owner() {
        let ownership = GateOwnership::new("local");
        assert!(!ownership.caller_is_hardware_owner(None).await);
    }

    #[tokio::test]
    async fn local_gate_is_hardware_owner_by_default() {
        let ownership = GateOwnership::new("local");
        assert!(ownership.caller_is_hardware_owner(Some("local")).await);
    }

    #[tokio::test]
    async fn revert_to_local_owner_after_remote_set() {
        let ownership = GateOwnership::new("local");
        ownership.note_gate_update(&Arc::from("remote-owner"), true).await;
        assert_eq!(
            ownership.hardware_owner_gate_id().await.as_ref(),
            "remote-owner"
        );

        ownership.revert_to_local_owner().await;
        assert_eq!(ownership.hardware_owner_gate_id().await.as_ref(), "local");
    }

    #[tokio::test]
    async fn note_gate_update_false_is_noop() {
        let ownership = GateOwnership::new("local");
        ownership.note_gate_update(&Arc::from("other"), false).await;
        assert_eq!(ownership.hardware_owner_gate_id().await.as_ref(), "local");
    }
}
