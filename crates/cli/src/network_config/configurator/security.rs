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
