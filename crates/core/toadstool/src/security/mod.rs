// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security context and policies for `ToadStool` workloads

mod context;
mod policy;
mod provider;
mod types;

// Re-export all public types for backward compatibility
pub use context::SecurityContext;
pub use policy::{AuditEvent, AuditSettings, SecurityPolicy, SecuritySettings};
pub use provider::SecurityProvider;
pub use types::{Capability, FilesystemSecurity, IsolationLevel, NetworkSecurity, UserContext};

#[cfg(test)]
mod tests {
    use super::*;

    // ─── SecurityContext ───────────────────────────────────────────────────

    #[test]
    fn security_context_default() {
        let ctx = SecurityContext::default();
        assert_eq!(ctx.isolation_level, IsolationLevel::Standard);
        assert_eq!(ctx.capabilities.len(), 2);
        assert!(ctx.capabilities.contains(&Capability::Execute));
        assert!(ctx.capabilities.contains(&Capability::Read));
        assert!(ctx.user_context.is_none());
    }

    #[test]
    fn security_context_for_isolation_level() {
        for level in [
            IsolationLevel::None,
            IsolationLevel::Basic,
            IsolationLevel::Standard,
            IsolationLevel::Enhanced,
            IsolationLevel::Maximum,
        ] {
            let ctx = SecurityContext::for_isolation_level(level.clone());
            assert_eq!(ctx.isolation_level, level);
            assert_eq!(ctx.capabilities, vec![Capability::Execute]);
            assert!(ctx.user_context.is_none());
        }
    }

    #[test]
    fn security_context_with_capability() {
        let ctx = SecurityContext::default()
            .with_capability(Capability::Write)
            .with_capability(Capability::NetworkClient);
        assert!(ctx.has_capability(&Capability::Execute));
        assert!(ctx.has_capability(&Capability::Read));
        assert!(ctx.has_capability(&Capability::Write));
        assert!(ctx.has_capability(&Capability::NetworkClient));
        assert!(!ctx.has_capability(&Capability::NetworkServer));
    }

    #[test]
    fn security_context_has_capability_custom() {
        let cap = Capability::Custom("foo".to_string());
        let ctx = SecurityContext::default().with_capability(cap.clone());
        assert!(ctx.has_capability(&cap));
        assert!(!ctx.has_capability(&Capability::Custom("bar".to_string())));
    }

    #[test]
    fn security_context_with_user_context() {
        let user = UserContext {
            username: Some("testuser".to_string()),
            uid: Some(1000),
            gid: Some(1000),
            groups: vec![100, 101],
        };
        let ctx = SecurityContext::default().with_user_context(user.clone());
        assert!(ctx.user_context.is_some());
        let uc = ctx.user_context.unwrap();
        assert_eq!(uc.username, Some("testuser".to_string()));
        assert_eq!(uc.uid, Some(1000));
        assert_eq!(uc.gid, Some(1000));
        assert_eq!(uc.groups, vec![100, 101]);
    }

    #[test]
    fn security_context_validate_empty_capabilities() {
        let mut ctx = SecurityContext::default();
        ctx.capabilities.clear();
        let result = ctx.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least one capability"));
    }

    #[test]
    fn security_context_validate_write_with_readonly_filesystem() {
        let mut ctx = SecurityContext::default()
            .with_capability(Capability::Write)
            .with_user_context(UserContext {
                username: None,
                uid: None,
                gid: None,
                groups: vec![],
            });
        ctx.filesystem_security.read_only = true;
        let result = ctx.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("read-only filesystem"));
    }

    #[test]
    fn security_context_validate_ok() {
        let ctx = SecurityContext::default();
        assert!(ctx.validate().is_ok());

        let ctx = SecurityContext::default().with_capability(Capability::Write);
        assert!(ctx.validate().is_ok());
    }

    #[test]
    fn security_context_field_access() {
        let ctx = SecurityContext::default();
        assert!(!ctx.network_security.allow_outbound);
        assert!(!ctx.filesystem_security.read_only);
    }

    #[test]
    fn security_context_has_permission() {
        // Default has Execute + Read; add Write to test all three
        let ctx = SecurityContext::default().with_capability(Capability::Write);
        assert!(ctx.has_permission("read"));
        assert!(ctx.has_permission("write"));
        assert!(ctx.has_permission("execute"));
        assert!(!ctx.has_permission("network_client"));

        // Minimal context with only Execute (from for_isolation_level)
        let ctx = SecurityContext::for_isolation_level(IsolationLevel::Basic);
        assert!(ctx.has_permission("execute"));
        assert!(!ctx.has_permission("read"));
    }

    #[test]
    fn security_context_has_permission_wildcard() {
        let ctx = SecurityContext::default();
        assert!(ctx.has_permission("*"));
        let mut empty = SecurityContext::default();
        empty.capabilities.clear();
        assert!(!empty.has_permission("*"));
    }

    #[test]
    fn security_context_has_permission_custom() {
        let ctx =
            SecurityContext::default().with_capability(Capability::Custom("my_cap".to_string()));
        assert!(ctx.has_permission("my_cap"));
        assert!(!ctx.has_permission("other_cap"));
    }

    // ─── IsolationLevel ───────────────────────────────────────────────────

    #[test]
    fn isolation_level_variants() {
        let _ = IsolationLevel::None;
        let _ = IsolationLevel::Basic;
        let _ = IsolationLevel::Standard;
        let _ = IsolationLevel::Enhanced;
        let _ = IsolationLevel::Maximum;
    }

    #[test]
    fn isolation_level_partial_eq() {
        assert_eq!(IsolationLevel::Standard, IsolationLevel::Standard);
        assert_ne!(IsolationLevel::None, IsolationLevel::Maximum);
    }

    #[test]
    fn isolation_level_debug_clone() {
        let level = IsolationLevel::Enhanced;
        let cloned = level.clone();
        assert_eq!(level, cloned);
        assert!(!format!("{:?}", level).is_empty());
    }

    // ─── Capability ───────────────────────────────────────────────────────

    #[test]
    fn capability_variants() {
        let _ = Capability::Execute;
        let _ = Capability::Read;
        let _ = Capability::Write;
        let _ = Capability::NetworkClient;
        let _ = Capability::NetworkServer;
        let _ = Capability::SystemInfo;
        let _ = Capability::ProcessManagement;
        let _ = Capability::Custom("custom".to_string());
    }

    #[test]
    fn capability_partial_eq() {
        assert_eq!(Capability::Read, Capability::Read);
        assert_ne!(Capability::Read, Capability::Write);
        assert_eq!(
            Capability::Custom("x".to_string()),
            Capability::Custom("x".to_string())
        );
        assert_ne!(
            Capability::Custom("x".to_string()),
            Capability::Custom("y".to_string())
        );
    }

    #[test]
    fn capability_debug_clone() {
        let cap = Capability::NetworkServer;
        let cloned = cap.clone();
        assert_eq!(cap, cloned);
        let custom = Capability::Custom("foo".to_string());
        let custom_cloned = custom.clone();
        assert_eq!(custom, custom_cloned);
    }

    // ─── UserContext ──────────────────────────────────────────────────────

    #[test]
    fn user_context_construction() {
        let user = UserContext {
            username: Some("alice".to_string()),
            uid: Some(1000),
            gid: Some(1001),
            groups: vec![100, 200, 300],
        };
        assert_eq!(user.username, Some("alice".to_string()));
        assert_eq!(user.uid, Some(1000));
        assert_eq!(user.gid, Some(1001));
        assert_eq!(user.groups, vec![100, 200, 300]);
    }

    #[test]
    fn user_context_minimal() {
        let user = UserContext {
            username: None,
            uid: None,
            gid: None,
            groups: vec![],
        };
        assert!(user.username.is_none());
        assert!(user.uid.is_none());
        assert!(user.gid.is_none());
        assert!(user.groups.is_empty());
    }

    #[test]
    fn user_context_debug_clone() {
        let user = UserContext {
            username: Some("bob".to_string()),
            uid: Some(42),
            gid: Some(42),
            groups: vec![1, 2],
        };
        let cloned = user.clone();
        assert_eq!(user.username, cloned.username);
        assert_eq!(user.uid, cloned.uid);
    }

    // ─── NetworkSecurity ──────────────────────────────────────────────────

    #[test]
    fn network_security_default() {
        let ns = NetworkSecurity::default();
        assert!(!ns.allow_outbound);
        assert!(!ns.allow_inbound);
        assert!(ns.allowed_domains.is_empty());
        assert!(ns.blocked_domains.is_empty());
        assert!(ns.allowed_ports.is_empty());
        assert!(ns.blocked_ports.is_empty());
    }

    #[test]
    fn network_security_construction() {
        let ns = NetworkSecurity {
            allow_outbound: true,
            allow_inbound: false,
            allowed_domains: vec!["example.com".to_string()],
            blocked_domains: vec!["bad.com".to_string()],
            allowed_ports: vec![80, 443],
            blocked_ports: vec![22],
        };
        assert!(ns.allow_outbound);
        assert!(!ns.allow_inbound);
        assert_eq!(ns.allowed_domains, vec!["example.com".to_string()]);
        assert_eq!(ns.blocked_ports, vec![22]);
    }

    // ─── FilesystemSecurity ───────────────────────────────────────────────

    #[test]
    fn filesystem_security_default() {
        let fs = FilesystemSecurity::default();
        assert!(!fs.read_only);
        assert!(fs.allowed_read_paths.is_empty());
        assert!(fs.allowed_write_paths.is_empty());
        assert!(fs.blocked_paths.is_empty());
    }

    #[test]
    fn filesystem_security_construction() {
        let fs = FilesystemSecurity {
            read_only: true,
            allowed_read_paths: vec!["/tmp".to_string()],
            allowed_write_paths: vec!["/tmp/write".to_string()],
            blocked_paths: vec!["/etc".to_string()],
        };
        assert!(fs.read_only);
        assert_eq!(fs.allowed_read_paths, vec!["/tmp".to_string()]);
        assert_eq!(fs.blocked_paths, vec!["/etc".to_string()]);
    }

    // ─── SecuritySettings ─────────────────────────────────────────────────

    #[test]
    fn security_settings_default() {
        let ss = SecuritySettings::default();
        assert_eq!(ss.default_isolation_level, IsolationLevel::Standard);
        assert_eq!(ss.default_capabilities, vec![Capability::Execute]);
        assert!(ss.security_policies.is_empty());
        assert!(ss.audit_settings.enabled);
    }

    // ─── SecurityPolicy ───────────────────────────────────────────────────

    #[test]
    fn security_policy_construction() {
        let policy = SecurityPolicy {
            name: "strict".to_string(),
            version: "1.0".to_string(),
            isolation_level: IsolationLevel::Maximum,
            allowed_capabilities: vec![Capability::Execute, Capability::Read],
            denied_capabilities: vec![Capability::NetworkServer],
            network_restrictions: NetworkSecurity::default(),
            filesystem_restrictions: FilesystemSecurity::default(),
        };
        assert_eq!(policy.name, "strict");
        assert_eq!(policy.version, "1.0");
        assert_eq!(policy.isolation_level, IsolationLevel::Maximum);
        assert_eq!(policy.allowed_capabilities.len(), 2);
        assert_eq!(policy.denied_capabilities.len(), 1);
    }

    #[test]
    fn security_policy_debug_clone() {
        let policy = SecurityPolicy {
            name: "p".to_string(),
            version: "1".to_string(),
            isolation_level: IsolationLevel::Basic,
            allowed_capabilities: vec![],
            denied_capabilities: vec![],
            network_restrictions: NetworkSecurity::default(),
            filesystem_restrictions: FilesystemSecurity::default(),
        };
        let cloned = policy.clone();
        assert_eq!(policy.name, cloned.name);
    }

    // ─── AuditSettings & AuditEvent ────────────────────────────────────────

    #[test]
    fn audit_settings_default() {
        let as_ = AuditSettings::default();
        assert!(as_.enabled);
        assert_eq!(as_.log_level, "info");
        assert_eq!(as_.events.len(), 3);
        assert!(as_.events.contains(&AuditEvent::ExecutionStart));
        assert!(as_.events.contains(&AuditEvent::ExecutionEnd));
        assert!(as_.events.contains(&AuditEvent::SecurityViolation));
    }

    #[test]
    fn audit_event_variants_partial_eq() {
        assert_eq!(AuditEvent::ExecutionStart, AuditEvent::ExecutionStart);
        assert_ne!(AuditEvent::ExecutionStart, AuditEvent::CapabilityUsed);
    }

    // ─── Serialization round-trips ────────────────────────────────────────

    #[test]
    fn security_context_serialization_roundtrip() {
        let ctx = SecurityContext::default()
            .with_capability(Capability::Write)
            .with_user_context(UserContext {
                username: Some("u".to_string()),
                uid: Some(1000),
                gid: Some(1000),
                groups: vec![100],
            });
        let json = serde_json::to_string(&ctx).unwrap();
        let restored: SecurityContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx.isolation_level, restored.isolation_level);
        assert_eq!(ctx.capabilities, restored.capabilities);
        assert_eq!(
            ctx.user_context.as_ref().unwrap().username,
            restored.user_context.as_ref().unwrap().username
        );
    }

    #[test]
    fn isolation_level_serialization_roundtrip() {
        for level in [
            IsolationLevel::None,
            IsolationLevel::Basic,
            IsolationLevel::Standard,
            IsolationLevel::Enhanced,
            IsolationLevel::Maximum,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let restored: IsolationLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, restored);
        }
    }

    #[test]
    fn capability_serialization_roundtrip() {
        let cap = Capability::Custom("my_cap".to_string());
        let json = serde_json::to_string(&cap).unwrap();
        let restored: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(cap, restored);
    }

    #[test]
    fn network_security_serialization_roundtrip() {
        let ns = NetworkSecurity {
            allow_outbound: true,
            allow_inbound: true,
            allowed_domains: vec!["a.com".to_string()],
            blocked_domains: vec![],
            allowed_ports: vec![443],
            blocked_ports: vec![22],
        };
        let json = serde_json::to_string(&ns).unwrap();
        let restored: NetworkSecurity = serde_json::from_str(&json).unwrap();
        assert_eq!(ns.allow_outbound, restored.allow_outbound);
        assert_eq!(ns.allowed_domains, restored.allowed_domains);
    }

    #[test]
    fn security_policy_serialization_roundtrip() {
        let policy = SecurityPolicy {
            name: "test".to_string(),
            version: "1.0".to_string(),
            isolation_level: IsolationLevel::Enhanced,
            allowed_capabilities: vec![Capability::Execute],
            denied_capabilities: vec![],
            network_restrictions: NetworkSecurity::default(),
            filesystem_restrictions: FilesystemSecurity::default(),
        };
        let json = serde_json::to_string(&policy).unwrap();
        let restored: SecurityPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy.name, restored.name);
        assert_eq!(policy.isolation_level, restored.isolation_level);
    }
}
