// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security and network policies extension
//!
//! Provides cross-primal security and network policy configuration.

use toadstool::error::ToadStoolResult;
use tracing::{debug, info, trace};

/// Security extension trait
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
pub(crate) trait SecurityExt {
    /// Apply cross-primal security configuration
    async fn apply_cross_primal_security_config(&self) -> ToadStoolResult<()>;

    /// Apply network policies configuration
    async fn apply_network_policies_config(&self) -> ToadStoolResult<()>;

    /// Validate cross-primal security configuration
    fn validate_cross_primal_security_config(&self) -> ToadStoolResult<()>;

    /// Validate network policies configuration
    fn validate_network_policies_config(&self) -> ToadStoolResult<()>;
}

impl SecurityExt for super::OrchestrationNetworkConfigurator {
    async fn apply_cross_primal_security_config(&self) -> ToadStoolResult<()> {
        info!("🔐 Applying cross-primal security configuration");

        let config = &self.config.cross_primal_security;
        debug!("Authentication method: {}", config.authentication.method);
        debug!(
            "configuration stored; runtime application deferred to orchestration layer (cross-primal security)"
        );

        Ok(())
    }

    async fn apply_network_policies_config(&self) -> ToadStoolResult<()> {
        info!("🛡️ Applying network policies configuration");

        let config = &self.config.network_policies;
        debug!("Default policy: {}", config.default_policy);
        debug!(
            "configuration stored; runtime application deferred to orchestration layer (network policies)"
        );

        Ok(())
    }

    fn validate_cross_primal_security_config(&self) -> ToadStoolResult<()> {
        trace!(
            "validate_cross_primal_security_config: structural checks (methods, endpoints); no credential or policy evaluation"
        );
        let config = &self.config.cross_primal_security;

        if !config.enabled {
            return Ok(());
        }

        if config.authentication.method.trim().is_empty() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Authentication method cannot be empty when cross-primal security is enabled"
                    .to_string(),
            ));
        }

        if config.authorization.model.trim().is_empty() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Authorization model cannot be empty when cross-primal security is enabled"
                    .to_string(),
            ));
        }

        if config
            .authorization
            .policy_engine
            .engine_type
            .trim()
            .is_empty()
        {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Policy engine type cannot be empty when cross-primal security is enabled"
                    .to_string(),
            ));
        }

        let pki = &config.authentication.security;
        if pki.enabled && pki.endpoint.trim().is_empty() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "PKI / security integration endpoint cannot be empty when enabled".to_string(),
            ));
        }

        if config.network_isolation.enabled
            && config.network_isolation.isolation_level.trim().is_empty()
        {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Network isolation level cannot be empty when isolation is enabled".to_string(),
            ));
        }

        if config.audit_logging.enabled && config.audit_logging.destinations.is_empty() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "At least one audit log destination must be configured when audit logging is enabled"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn validate_network_policies_config(&self) -> ToadStoolResult<()> {
        trace!(
            "validate_network_policies_config: structural checks (names, ports, actions); no dataplane enforcement test"
        );
        let config = &self.config.network_policies;

        if !config.enabled {
            return Ok(());
        }

        if config.default_policy.trim().is_empty() {
            return Err(toadstool::error::ToadStoolError::configuration(
                "Default network policy cannot be empty when network policies are enabled"
                    .to_string(),
            ));
        }

        for rule in &config.ingress_rules {
            if rule.name.trim().is_empty() {
                return Err(toadstool::error::ToadStoolError::configuration(
                    "Ingress rule name cannot be empty".to_string(),
                ));
            }
            if rule.action.trim().is_empty() {
                return Err(toadstool::error::ToadStoolError::configuration(format!(
                    "Ingress rule `{}` action cannot be empty",
                    rule.name
                )));
            }
            for p in &rule.ports {
                if let Some(end) = p.end_port {
                    if end < p.port {
                        return Err(toadstool::error::ToadStoolError::configuration(format!(
                            "Ingress rule `{}`: port range end {} is before start {}",
                            rule.name, end, p.port
                        )));
                    }
                }
            }
        }

        for rule in &config.egress_rules {
            if rule.name.trim().is_empty() {
                return Err(toadstool::error::ToadStoolError::configuration(
                    "Egress rule name cannot be empty".to_string(),
                ));
            }
            if rule.action.trim().is_empty() {
                return Err(toadstool::error::ToadStoolError::configuration(format!(
                    "Egress rule `{}` action cannot be empty",
                    rule.name
                )));
            }
            for p in &rule.ports {
                if let Some(end) = p.end_port {
                    if end < p.port {
                        return Err(toadstool::error::ToadStoolError::configuration(format!(
                            "Egress rule `{}`: port range end {} is before start {}",
                            rule.name, end, p.port
                        )));
                    }
                }
            }
        }

        for policy in &config.service_mesh_policies {
            if policy.name.trim().is_empty() {
                return Err(toadstool::error::ToadStoolError::configuration(
                    "Service mesh policy name cannot be empty".to_string(),
                ));
            }
            if policy.policy_type.trim().is_empty() {
                return Err(toadstool::error::ToadStoolError::configuration(format!(
                    "Service mesh policy `{}` type cannot be empty",
                    policy.name
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SecurityExt;
    use crate::network_config::{
        EgressRule, IngressRule, NetworkPoliciesConfig, NetworkPort, NetworkSelector,
        OrchestrationNetworkConfigurator, ServiceMeshPolicy,
    };
    use std::collections::HashMap;

    fn configurator_with_network_policies(
        cfg: NetworkPoliciesConfig,
    ) -> OrchestrationNetworkConfigurator {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.network_policies = cfg;
        c
    }

    #[test]
    fn validate_cross_primal_security_default_succeeds() {
        let c = OrchestrationNetworkConfigurator::new();
        assert!(c.validate_cross_primal_security_config().is_ok());
    }

    #[test]
    fn validate_cross_primal_security_skips_when_disabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.cross_primal_security.enabled = false;
        c.config.cross_primal_security.authentication.method = String::new();
        c.config.cross_primal_security.authorization.model = String::new();
        assert!(c.validate_cross_primal_security_config().is_ok());
    }

    #[test]
    fn validate_cross_primal_security_rejects_empty_auth_method_when_enabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.cross_primal_security.enabled = true;
        c.config.cross_primal_security.authentication.method = "  \t  ".to_string();
        assert!(c.validate_cross_primal_security_config().is_err());
    }

    #[test]
    fn validate_cross_primal_security_rejects_empty_authorization_model_when_enabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.cross_primal_security.enabled = true;
        c.config.cross_primal_security.authorization.model = String::new();
        assert!(c.validate_cross_primal_security_config().is_err());
    }

    #[test]
    fn validate_cross_primal_security_rejects_empty_policy_engine_type_when_enabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.cross_primal_security.enabled = true;
        c.config
            .cross_primal_security
            .authorization
            .policy_engine
            .engine_type = " ".to_string();
        assert!(c.validate_cross_primal_security_config().is_err());
    }

    #[test]
    fn validate_cross_primal_security_rejects_pki_enabled_without_endpoint() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.cross_primal_security.enabled = true;
        c.config
            .cross_primal_security
            .authentication
            .security
            .enabled = true;
        c.config
            .cross_primal_security
            .authentication
            .security
            .endpoint = String::new();
        assert!(c.validate_cross_primal_security_config().is_err());
    }

    #[test]
    fn validate_cross_primal_security_rejects_isolation_enabled_without_level() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.cross_primal_security.enabled = true;
        c.config.cross_primal_security.network_isolation.enabled = true;
        c.config
            .cross_primal_security
            .network_isolation
            .isolation_level = "  ".to_string();
        assert!(c.validate_cross_primal_security_config().is_err());
    }

    #[test]
    fn validate_cross_primal_security_rejects_audit_enabled_without_destinations() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.cross_primal_security.enabled = true;
        c.config.cross_primal_security.audit_logging.enabled = true;
        c.config
            .cross_primal_security
            .audit_logging
            .destinations
            .clear();
        assert!(c.validate_cross_primal_security_config().is_err());
    }

    #[test]
    fn validate_network_policies_default_succeeds() {
        let c = OrchestrationNetworkConfigurator::new();
        assert!(c.validate_network_policies_config().is_ok());
    }

    #[test]
    fn validate_network_policies_skips_when_disabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.network_policies.enabled = false;
        c.config.network_policies.default_policy = String::new();
        assert!(c.validate_network_policies_config().is_ok());
    }

    #[test]
    fn validate_network_policies_rejects_empty_default_policy_when_enabled() {
        let mut c = OrchestrationNetworkConfigurator::new();
        c.config.network_policies.enabled = true;
        c.config.network_policies.default_policy = String::new();
        assert!(c.validate_network_policies_config().is_err());
    }

    #[test]
    fn validate_network_policies_rejects_ingress_empty_name() {
        let mut base = OrchestrationNetworkConfigurator::new()
            .config
            .network_policies;
        base.ingress_rules.push(IngressRule {
            name: String::new(),
            from: vec![],
            ports: vec![],
            action: "allow".to_string(),
            priority: 1,
        });
        let c = configurator_with_network_policies(base);
        assert!(c.validate_network_policies_config().is_err());
    }

    #[test]
    fn validate_network_policies_rejects_ingress_empty_action() {
        let mut base = OrchestrationNetworkConfigurator::new()
            .config
            .network_policies;
        base.ingress_rules.push(IngressRule {
            name: "r1".to_string(),
            from: vec![],
            ports: vec![],
            action: "  ".to_string(),
            priority: 1,
        });
        let c = configurator_with_network_policies(base);
        assert!(c.validate_network_policies_config().is_err());
    }

    #[test]
    fn validate_network_policies_rejects_ingress_invalid_port_range() {
        let mut base = OrchestrationNetworkConfigurator::new()
            .config
            .network_policies;
        base.ingress_rules.push(IngressRule {
            name: "ports".to_string(),
            from: vec![],
            ports: vec![NetworkPort {
                port: 200,
                protocol: "tcp".to_string(),
                end_port: Some(100),
            }],
            action: "allow".to_string(),
            priority: 1,
        });
        let c = configurator_with_network_policies(base);
        assert!(c.validate_network_policies_config().is_err());
    }

    #[test]
    fn validate_network_policies_rejects_egress_invalid_port_range() {
        let mut base = OrchestrationNetworkConfigurator::new()
            .config
            .network_policies;
        base.egress_rules.push(EgressRule {
            name: "eg".to_string(),
            to: vec![NetworkSelector {
                selector_type: "cidr".to_string(),
                value: "0.0.0.0/0".to_string(),
            }],
            ports: vec![NetworkPort {
                port: 50,
                protocol: "udp".to_string(),
                end_port: Some(40),
            }],
            action: "allow".to_string(),
            priority: 1,
        });
        let c = configurator_with_network_policies(base);
        assert!(c.validate_network_policies_config().is_err());
    }

    #[test]
    fn validate_network_policies_rejects_service_mesh_policy_empty_name() {
        let mut base = OrchestrationNetworkConfigurator::new()
            .config
            .network_policies;
        base.service_mesh_policies.push(ServiceMeshPolicy {
            name: String::new(),
            policy_type: "traffic".to_string(),
            selector: HashMap::new(),
            config: HashMap::new(),
        });
        let c = configurator_with_network_policies(base);
        assert!(c.validate_network_policies_config().is_err());
    }

    #[test]
    fn validate_network_policies_rejects_service_mesh_policy_empty_type() {
        let mut base = OrchestrationNetworkConfigurator::new()
            .config
            .network_policies;
        base.service_mesh_policies.push(ServiceMeshPolicy {
            name: "p".to_string(),
            policy_type: "  ".to_string(),
            selector: HashMap::new(),
            config: HashMap::new(),
        });
        let c = configurator_with_network_policies(base);
        assert!(c.validate_network_policies_config().is_err());
    }
}
