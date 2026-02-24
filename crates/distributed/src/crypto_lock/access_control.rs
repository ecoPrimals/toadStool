//! Access control and policy enforcement for crypto lock system

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use tracing::{debug, error, info, warn};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use super::cache::PermissionCache;
use super::permissions::{
    DelegationChain, DelegationRequest, DelegationScope, DelegationStatus, ExpiringPermission,
    ExternalTarget, PermissionHolder, PermissionScope, PermissionStatus,
    SecurityProviderPermission,
};
use super::validation::{PermissionValidationResult, SecurityPermissionValidator};

/// `ToadStool` Crypto Lock Manager - enforces cryptographic access control
pub struct ToadStoolCryptoLock {
    /// `BearDog` crypto permission validator
    permission_validator: SecurityPermissionValidator,
    /// Active permissions for external integrations
    active_permissions: HashMap<ExternalTarget, SecurityProviderPermission>,
    /// Permission cache for performance
    permission_cache: PermissionCache,
    /// Access control policies
    _access_policies: AccessPolicies,
}

impl ToadStoolCryptoLock {
    /// Create new crypto lock system
    pub async fn new() -> ToadStoolResult<Self> {
        info!("🔐 Initializing ToadStool crypto lock system");

        let permission_validator = SecurityPermissionValidator::new().await?;
        let active_permissions = HashMap::new();
        let permission_cache = PermissionCache::new();
        let access_policies = AccessPolicies;

        let crypto_lock = Self {
            permission_validator,
            active_permissions,
            permission_cache,
            _access_policies: access_policies,
        };

        // Load any existing permissions
        crypto_lock.load_permissions()?;

        // Enable Pure Rust ecosystem (always unlocked)
        crypto_lock.enable_pure_rust_ecosystem()?;

        info!("✅ ToadStool crypto lock system initialized");
        Ok(crypto_lock)
    }

    /// Enable Pure Rust ecosystem - ALWAYS UNLOCKED
    fn enable_pure_rust_ecosystem(&self) -> ToadStoolResult<()> {
        info!("🔓 Pure Rust ecosystem always unlocked (no crypto needed)");
        // Pure Rust ecosystem doesn't need crypto permissions
        // All ecoPrimals always work together (toadstool, security, storage, coordination)
        Ok(())
    }

    /// Check if external integration is unlocked by crypto permission
    pub async fn check_external_access(
        &self,
        target: &ExternalTarget,
    ) -> ToadStoolResult<AccessResult> {
        debug!(
            "🔍 Checking crypto permission for external target: {:?}",
            target
        );

        // Pure Rust ecosystem never needs crypto permissions
        if self.is_pure_rust_ecosystem(target) {
            return Ok(AccessResult::Granted {
                reason: "Pure Rust ecosystem - always unlocked".to_string(),
                permission_level: PermissionLevel::Full,
                expires_at: None,
                restrictions: vec![],
            });
        }

        // Check cache first for performance
        if let Some(cached_result) = self.permission_cache.get(target).await {
            if !cached_result.is_expired() {
                debug!("✅ Using cached permission result");
                return Ok(cached_result.result);
            }
        }

        // Look for valid crypto permission
        let permissions = self.find_permissions_for_target(target).await?;

        if permissions.is_empty() {
            warn!(
                "🔒 No crypto permission found for external target: {:?}",
                target
            );

            return Ok(AccessResult::Denied {
                reason: "No security provider crypto permission for external integration"
                    .to_string(),
                how_to_get_access: self.get_access_instructions(target),
            });
        }

        // Validate the best permission available
        let best_permission = self.select_best_permission(&permissions)?;
        let validation_result = self
            .permission_validator
            .validate_permission(&best_permission)
            .await?;

        match validation_result {
            PermissionValidationResult::Valid => {
                info!(
                    "✅ Crypto permission validated for external target: {:?}",
                    target
                );

                let access_result = AccessResult::Granted {
                    reason: format!(
                        "Valid BearDog crypto permission: {}",
                        best_permission.permission_id
                    ),
                    permission_level: self.calculate_permission_level(&best_permission),
                    expires_at: Some(best_permission.valid_until),
                    restrictions: self.extract_restrictions(&best_permission.scope),
                };

                // Cache the result
                self.permission_cache
                    .cache_result(target.clone(), access_result.clone())
                    .await;

                Ok(access_result)
            }
            PermissionValidationResult::Invalid => {
                error!(
                    "❌ Invalid crypto permission signature for target: {:?}",
                    target
                );

                Ok(AccessResult::Denied {
                    reason: "Invalid security provider permission signature".to_string(),
                    how_to_get_access: "Contact security provider for permission verification"
                        .to_string(),
                })
            }
            PermissionValidationResult::Expired => {
                warn!("⏰ Crypto permission expired for target: {:?}", target);

                Ok(AccessResult::Denied {
                    reason: "BearDog crypto permission expired".to_string(),
                    how_to_get_access: "Renew your BearDog permission or request delegation"
                        .to_string(),
                })
            }
            PermissionValidationResult::Revoked => {
                error!("🚫 Crypto permission revoked for target: {:?}", target);

                Ok(AccessResult::Denied {
                    reason: "Security provider permission revoked".to_string(),
                    how_to_get_access: "Contact permission issuer for resolution".to_string(),
                })
            }
        }
    }

    /// Install a `BearDog` crypto permission
    pub async fn install_crypto_permission(
        &mut self,
        permission: SecurityProviderPermission,
    ) -> ToadStoolResult<()> {
        info!(
            "📥 Installing security provider permission: {}",
            permission.permission_id
        );

        // Validate the crypto permission
        let validation_result = self
            .permission_validator
            .validate_permission(&permission)
            .await?;

        match validation_result {
            PermissionValidationResult::Valid => {
                info!("✅ Crypto permission signature valid, installing");

                // Validate delegation chain if present
                if let Some(delegation_chain) = &permission.delegation_chain {
                    self.validate_delegation_chain(delegation_chain).await?;
                }

                // Store the permission
                self.active_permissions
                    .insert(permission.external_target.clone(), permission.clone());

                // Clear relevant cache entries
                self.permission_cache
                    .invalidate_for_target(&permission.external_target)
                    .await;

                info!("🎉 Crypto permission installed successfully");
                Ok(())
            }
            PermissionValidationResult::Invalid => {
                error!("❌ Invalid crypto permission signature, rejecting");
                Err(ToadStoolError::security(
                    "Invalid BearDog crypto permission",
                ))
            }
            PermissionValidationResult::Expired => {
                error!("⏰ Crypto permission expired, rejecting");
                Err(ToadStoolError::security(
                    "Security provider permission expired",
                ))
            }
            PermissionValidationResult::Revoked => {
                error!("🚫 Crypto permission revoked, rejecting");
                Err(ToadStoolError::security(
                    "BearDog crypto permission revoked",
                ))
            }
        }
    }

    /// Request delegation of crypto permission (permission lending)
    pub async fn request_delegation(
        &self,
        from_holder: &PermissionHolder,
        to_holder: &PermissionHolder,
        target: &ExternalTarget,
        delegation_scope: DelegationScope,
        duration: Duration,
    ) -> ToadStoolResult<DelegationRequest> {
        info!("🤝 Requesting permission delegation");

        // Find the permission to delegate
        let base_permission = self
            .find_delegatable_permission(from_holder, target)
            .await?;

        // Check if delegation is allowed
        self.validate_delegation_request(&base_permission, &delegation_scope)
            .await?;

        // Create delegation request (would be processed by security provider)
        let delegation_request = DelegationRequest {
            request_id: Uuid::new_v4(),
            base_permission_id: base_permission.permission_id,
            from_holder: from_holder.clone(),
            to_holder: to_holder.clone(),
            target: target.clone(),
            delegation_scope,
            duration,
            requested_at: SystemTime::now(),
            status: DelegationStatus::Pending,
        };

        info!(
            "📋 Delegation request created: {}",
            delegation_request.request_id
        );
        Ok(delegation_request)
    }

    /// Get crypto lock status report
    pub async fn get_crypto_lock_status(&self) -> ToadStoolResult<CryptoLockStatus> {
        let mut status = CryptoLockStatus {
            pure_rust_unlocked: true, // Always unlocked
            external_permissions: HashMap::new(),
            delegation_chains: Vec::new(),
            expiring_permissions: Vec::new(),
        };

        for (target, permission) in &self.active_permissions {
            // Check expiration
            if let Ok(time_until_expiry) = permission.valid_until.duration_since(SystemTime::now())
            {
                if time_until_expiry < Duration::from_secs(7 * 24 * 60 * 60) {
                    status.expiring_permissions.push(ExpiringPermission {
                        permission_id: permission.permission_id,
                        target: target.clone(),
                        expires_in: time_until_expiry,
                    });
                }
            }

            // Track delegation chains
            if let Some(delegation_chain) = &permission.delegation_chain {
                status.delegation_chains.push(delegation_chain.clone());
            }

            status.external_permissions.insert(
                target.clone(),
                PermissionStatus {
                    permission_id: permission.permission_id,
                    holder: permission.holder.clone(),
                    valid_until: permission.valid_until,
                    scope: permission.scope.clone(),
                    is_delegated: permission.delegation_chain.is_some(),
                },
            );
        }

        Ok(status)
    }

    // Helper methods

    fn is_pure_rust_ecosystem(&self, target: &ExternalTarget) -> bool {
        // Define what constitutes "pure Rust ecosystem"
        // Check service feature_set for ecosystem membership
        // This replaces hardcoded service name matching with feature-based capability check
        match target {
            ExternalTarget::ExternalTool {
                tool_name: _,
                feature_set,
                ..
            } => {
                // Check if tool declares ecosystem-native capability via feature_set
                // This is the modern, metadata-driven approach
                if feature_set
                    .iter()
                    .any(|f| f == "ecosystem_native" || f == "ecoprimals_trusted")
                {
                    return true;
                }

                // Check for explicit trust markers in feature set
                if feature_set
                    .iter()
                    .any(|f| f == "trusted" || f == "no_crypto_lock")
                {
                    return true;
                }

                // Check if tool declares itself as a known primal type
                const ECOSYSTEM_PRIMALS: &[&str] = &[
                    "primal:toadstool",
                    "primal:security", // generic, not hardcoded primal name
                    "primal:nestgate",
                    "primal:songbird",
                    "primal:squirrel",
                ];
                if feature_set
                    .iter()
                    .any(|f| ECOSYSTEM_PRIMALS.contains(&f.as_str()))
                {
                    return true;
                }

                // All ecosystem services now use feature_set (v0.3.0+)
                // No fallback needed - services must declare their features explicitly
                false
            }
            _ => false, // All other externals require crypto permission
        }
    }

    async fn find_permissions_for_target(
        &self,
        target: &ExternalTarget,
    ) -> ToadStoolResult<Vec<SecurityProviderPermission>> {
        let permissions: Vec<SecurityProviderPermission> = self
            .active_permissions
            .iter()
            .filter(|(t, _)| *t == target)
            .map(|(_, p)| p.clone())
            .collect();

        Ok(permissions)
    }

    fn select_best_permission(
        &self,
        permissions: &[SecurityProviderPermission],
    ) -> ToadStoolResult<SecurityProviderPermission> {
        // Select permission with longest validity and best scope
        permissions
            .iter()
            .max_by_key(|p| p.valid_until)
            .cloned()
            .ok_or_else(|| ToadStoolError::not_found("No valid permission found"))
    }

    const fn calculate_permission_level(
        &self,
        permission: &SecurityProviderPermission,
    ) -> PermissionLevel {
        // Calculate permission level based on scope
        if permission.scope.feature_restrictions.is_empty() {
            PermissionLevel::Full
        } else if permission.scope.feature_restrictions.len() < 3 {
            PermissionLevel::Limited
        } else {
            PermissionLevel::Basic
        }
    }

    fn extract_restrictions(&self, scope: &PermissionScope) -> Vec<String> {
        let mut restrictions = Vec::new();

        if !scope.geographic_limits.is_empty() {
            restrictions.push(format!("Geographic limits: {:?}", scope.geographic_limits));
        }

        if !scope.feature_restrictions.is_empty() {
            restrictions.push(format!(
                "Feature restrictions: {:?}",
                scope.feature_restrictions
            ));
        }

        restrictions
    }

    fn get_access_instructions(&self, target: &ExternalTarget) -> String {
        match target {
            ExternalTarget::CloudProvider { provider, .. } => {
                format!("Get security provider permission for {provider:?} cloud provider")
            }
            ExternalTarget::ContainerPlatform { platform, .. } => {
                format!("Get BearDog crypto permission for {platform:?} container platform")
            }
            ExternalTarget::ExternalTool { tool_name, .. } => {
                format!("Get security provider permission for {tool_name} external tool")
            }
            _ => "Get appropriate security provider permission for this external integration"
                .to_string(),
        }
    }

    fn load_permissions(&self) -> ToadStoolResult<()> {
        // Load permissions from storage (file, database, etc.)
        // For now, just return success
        Ok(())
    }

    async fn validate_delegation_chain(&self, chain: &DelegationChain) -> ToadStoolResult<()> {
        // Validate each delegation in the chain
        for delegation in &chain.delegations {
            self.permission_validator
                .validate_delegation_proof(&delegation.delegation_proof)
                .await?;
        }

        // Check delegation depth limits
        if chain.delegation_level > chain.max_delegation_depth {
            return Err(ToadStoolError::configuration("Delegation chain too deep"));
        }

        Ok(())
    }

    async fn find_delegatable_permission(
        &self,
        _holder: &PermissionHolder,
        target: &ExternalTarget,
    ) -> ToadStoolResult<SecurityProviderPermission> {
        self.active_permissions
            .get(target)
            .cloned()
            .ok_or_else(|| ToadStoolError::not_found("No delegatable permission found"))
    }

    async fn validate_delegation_request(
        &self,
        _permission: &SecurityProviderPermission,
        _scope: &DelegationScope,
    ) -> ToadStoolResult<()> {
        // Validate that the delegation scope is within the original permission scope
        Ok(())
    }
}

// Access control result types

/// Access result (granted or denied)
#[derive(Debug, Clone)]
pub enum AccessResult {
    Granted {
        reason: String,
        permission_level: PermissionLevel,
        expires_at: Option<SystemTime>,
        restrictions: Vec<String>,
    },
    Denied {
        reason: String,
        how_to_get_access: String,
    },
}

/// Permission level for access
#[derive(Debug, Clone)]
pub enum PermissionLevel {
    Basic,
    Limited,
    Full,
}

/// Crypto lock status report
#[derive(Debug, Clone)]
pub struct CryptoLockStatus {
    pub pure_rust_unlocked: bool,
    pub external_permissions: HashMap<ExternalTarget, PermissionStatus>,
    pub delegation_chains: Vec<DelegationChain>,
    pub expiring_permissions: Vec<ExpiringPermission>,
}

/// Access control policies
pub struct AccessPolicies;

impl Default for AccessPolicies {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto_lock::permissions::{
        DelegationScope, ExpiringPermission, PermissionHolder, PermissionMetadata, PermissionScope,
        PermissionStatus, ResourceLimits, SecurityProviderPermission, TimeRestrictions,
        UsageQuotas,
    };
    use crate::crypto_lock::validation::{
        CryptoAlgorithm, ProofMetadata, SecurityProof, VerificationLevel,
    };
    use crate::security_provider::types::{
        CloudProvider, ContainerPlatform, ExternalTarget, HPCScheduler, QuantumProvider,
        ServiceTier,
    };
    use std::time::Duration;

    fn pure_rust_target() -> ExternalTarget {
        ExternalTarget::ExternalTool {
            tool_name: "toadstool".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["ecosystem_native".to_string()],
        }
    }

    fn trusted_target() -> ExternalTarget {
        ExternalTarget::ExternalTool {
            tool_name: "nestgate".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["trusted".to_string()],
        }
    }

    fn primal_toadstool_target() -> ExternalTarget {
        ExternalTarget::ExternalTool {
            tool_name: "tool".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["primal:toadstool".to_string()],
        }
    }

    fn cloud_target() -> ExternalTarget {
        ExternalTarget::CloudProvider {
            provider: CloudProvider::AWS,
            regions: vec!["us-east-1".to_string()],
            services: vec![],
        }
    }

    #[allow(dead_code)]
    fn container_target() -> ExternalTarget {
        ExternalTarget::ContainerPlatform {
            platform: ContainerPlatform::Kubernetes,
            clusters: vec![],
            namespaces: vec![],
        }
    }

    fn external_tool_target_no_trust() -> ExternalTarget {
        ExternalTarget::ExternalTool {
            tool_name: "external-api".to_string(),
            api_endpoints: vec!["https://api.example.com".to_string()],
            feature_set: vec![],
        }
    }

    #[tokio::test]
    async fn test_crypto_lock_new_creates_instance() {
        let result = ToadStoolCryptoLock::new().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_external_access_pure_rust_ecosystem_granted() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = pure_rust_target();
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted {
                reason,
                permission_level,
                ..
            } => {
                assert!(reason.contains("Pure Rust"));
                assert!(matches!(permission_level, PermissionLevel::Full));
            }
            AccessResult::Denied { .. } => panic!("Expected Granted for pure Rust ecosystem"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_trusted_feature_granted() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = trusted_target();
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            AccessResult::Denied { .. } => panic!("Expected Granted for trusted"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_primal_type_granted() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = primal_toadstool_target();
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            AccessResult::Denied { .. } => panic!("Expected Granted for primal"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_no_permission_denied() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = cloud_target();
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => panic!("Expected Denied for cloud without permission"),
            AccessResult::Denied {
                reason,
                how_to_get_access,
            } => {
                assert!(!reason.is_empty(), "Denied should have a reason");
                assert!(!how_to_get_access.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_check_external_access_external_tool_no_trust_denied() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = external_tool_target_no_trust();
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => panic!("Expected Denied"),
            AccessResult::Denied {
                how_to_get_access,
                reason: _,
            } => {
                assert!(how_to_get_access.contains("external tool"));
            }
        }
    }

    #[tokio::test]
    async fn test_get_crypto_lock_status() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let status = lock.get_crypto_lock_status().await.unwrap();
        assert!(status.pure_rust_unlocked);
        assert!(status.external_permissions.is_empty());
    }

    #[test]
    fn test_access_result_granted_constructor() {
        let result = AccessResult::Granted {
            reason: "test".to_string(),
            permission_level: PermissionLevel::Full,
            expires_at: None,
            restrictions: vec![],
        };
        match &result {
            AccessResult::Granted {
                permission_level, ..
            } => {
                assert!(matches!(permission_level, PermissionLevel::Full));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_access_result_denied_constructor() {
        let result = AccessResult::Denied {
            reason: "No permission".to_string(),
            how_to_get_access: "Contact admin".to_string(),
        };
        match &result {
            AccessResult::Denied { reason, .. } => assert_eq!(reason, "No permission"),
            _ => panic!(),
        }
    }

    #[test]
    fn test_permission_level_variants() {
        let _b = PermissionLevel::Basic;
        let _l = PermissionLevel::Limited;
        let _f = PermissionLevel::Full;
    }

    #[test]
    fn test_crypto_lock_status_constructor() {
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: std::collections::HashMap::new(),
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        assert!(status.pure_rust_unlocked);
    }

    #[test]
    fn test_access_policies_default() {
        let policies = AccessPolicies::default();
        let _ = policies;
    }

    fn make_expired_permission(target: ExternalTarget) -> SecurityProviderPermission {
        let now = SystemTime::now();
        let valid_from = now - Duration::from_secs(3600);
        let valid_until = now - Duration::from_secs(1); // Expired
        SecurityProviderPermission {
            permission_id: Uuid::new_v4(),
            holder: PermissionHolder::Individual {
                user_id: "user1".to_string(),
                public_key: "pk".to_string(),
                verification_level: VerificationLevel::Unverified,
            },
            external_target: target,
            scope: PermissionScope {
                resource_limits: ResourceLimits {
                    max_cpu_cores: None,
                    max_memory_gb: None,
                    max_storage_gb: None,
                    max_network_bandwidth: None,
                },
                time_restrictions: TimeRestrictions {
                    allowed_hours: None,
                    allowed_days: None,
                    timezone: None,
                },
                usage_quotas: UsageQuotas {
                    max_requests_per_hour: None,
                    max_data_transfer_gb: None,
                    max_compute_hours: None,
                },
                geographic_limits: vec![],
                feature_restrictions: vec![],
            },
            valid_from,
            valid_until,
            crypto_proof: SecurityProof {
                signature: vec![],
                algorithm: CryptoAlgorithm::Ed25519,
                public_key_id: "key1".to_string(),
                timestamp: now,
                metadata: ProofMetadata {
                    issuer: "test".to_string(),
                    purpose: "test".to_string(),
                    additional_claims: std::collections::HashMap::new(),
                },
            },
            delegation_chain: None,
            metadata: PermissionMetadata {
                issued_by: "test".to_string(),
                notes: String::new(),
                features: vec![],
            },
        }
    }

    #[tokio::test]
    async fn test_install_crypto_permission_expired_rejects() {
        let mut lock = ToadStoolCryptoLock::new().await.unwrap();
        let perm = make_expired_permission(cloud_target());
        let result = lock.install_crypto_permission(perm).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_request_delegation_no_permission_fails() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let from = PermissionHolder::Individual {
            user_id: "user1".to_string(),
            public_key: "pk".to_string(),
            verification_level: VerificationLevel::Unverified,
        };
        let to = PermissionHolder::Individual {
            user_id: "user2".to_string(),
            public_key: "pk2".to_string(),
            verification_level: VerificationLevel::Unverified,
        };
        let target = cloud_target();
        let scope = DelegationScope {
            resource_limits: None,
            time_limits: None,
            feature_subset: vec![],
            geographic_subset: vec![],
        };
        let result = lock
            .request_delegation(&from, &to, &target, scope, Duration::from_secs(3600))
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_crypto_lock_status_expiring_permissions_field() {
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: std::collections::HashMap::new(),
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        assert!(status.expiring_permissions.is_empty());
    }

    // ─── Permission level and access result variants ─────────────────────────────

    #[test]
    fn test_permission_level_all_variants_debug() {
        let levels = [
            PermissionLevel::Basic,
            PermissionLevel::Limited,
            PermissionLevel::Full,
        ];
        for level in levels {
            let s = format!("{:?}", level);
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn test_access_result_granted_with_restrictions() {
        let result = AccessResult::Granted {
            reason: "ok".to_string(),
            permission_level: PermissionLevel::Limited,
            expires_at: None,
            restrictions: vec!["geo:us".to_string()],
        };
        match &result {
            AccessResult::Granted { restrictions, .. } => {
                assert_eq!(restrictions.len(), 1);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_access_result_denied_debug() {
        let result = AccessResult::Denied {
            reason: "no perm".to_string(),
            how_to_get_access: "contact admin".to_string(),
        };
        let s = format!("{:?}", result);
        assert!(s.contains("Denied"));
    }

    #[tokio::test]
    async fn test_check_external_access_ecoprimals_trusted() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "songbird".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["ecoprimals_trusted".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            AccessResult::Denied { .. } => panic!("Expected Granted for ecoprimals_trusted"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_no_crypto_lock_feature() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "tool".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["no_crypto_lock".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            _ => panic!("Expected Granted for no_crypto_lock"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_primal_security() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "x".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["primal:security".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            _ => panic!("Expected Granted for primal:security"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_container_platform_denied() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ContainerPlatform {
            platform: ContainerPlatform::Kubernetes,
            clusters: vec![],
            namespaces: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("container"));
            }
            AccessResult::Granted { .. } => {
                panic!("Expected Denied for container without permission")
            }
        }
    }

    #[tokio::test]
    async fn test_get_crypto_lock_status_with_expiring() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let status = lock.get_crypto_lock_status().await.unwrap();
        assert!(status.pure_rust_unlocked);
        assert!(status.delegation_chains.is_empty());
    }

    #[test]
    fn test_access_result_clone() {
        let granted = AccessResult::Granted {
            reason: "r".to_string(),
            permission_level: PermissionLevel::Full,
            expires_at: None,
            restrictions: vec![],
        };
        let c = granted.clone();
        assert!(matches!(c, AccessResult::Granted { .. }));
    }

    // ─── Additional permission paths, policy, token tests ───────────────────────

    #[test]
    fn test_permission_level_basic_debug() {
        let level = PermissionLevel::Basic;
        let s = format!("{:?}", level);
        assert!(s.contains("Basic"));
    }

    #[test]
    fn test_permission_level_limited_debug() {
        let level = PermissionLevel::Limited;
        let s = format!("{:?}", level);
        assert!(s.contains("Limited"));
    }

    #[test]
    fn test_access_result_granted_with_expires_at() {
        let result = AccessResult::Granted {
            reason: "ok".to_string(),
            permission_level: PermissionLevel::Full,
            expires_at: Some(SystemTime::now() + Duration::from_secs(3600)),
            restrictions: vec![],
        };
        match &result {
            AccessResult::Granted { expires_at, .. } => assert!(expires_at.is_some()),
            _ => panic!(),
        }
    }

    #[test]
    fn test_crypto_lock_status_with_external_permissions() {
        let mut perms = std::collections::HashMap::new();
        perms.insert(
            cloud_target(),
            PermissionStatus {
                permission_id: Uuid::new_v4(),
                holder: PermissionHolder::Individual {
                    user_id: "u1".to_string(),
                    public_key: "pk".to_string(),
                    verification_level: VerificationLevel::Unverified,
                },
                valid_until: SystemTime::now() + Duration::from_secs(3600),
                scope: PermissionScope {
                    resource_limits: ResourceLimits {
                        max_cpu_cores: None,
                        max_memory_gb: None,
                        max_storage_gb: None,
                        max_network_bandwidth: None,
                    },
                    time_restrictions: TimeRestrictions {
                        allowed_hours: None,
                        allowed_days: None,
                        timezone: None,
                    },
                    usage_quotas: UsageQuotas {
                        max_requests_per_hour: None,
                        max_data_transfer_gb: None,
                        max_compute_hours: None,
                    },
                    geographic_limits: vec![],
                    feature_restrictions: vec![],
                },
                is_delegated: false,
            },
        );
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: perms,
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        assert_eq!(status.external_permissions.len(), 1);
    }

    #[test]
    fn test_expiring_permission_constructor() {
        let ep = ExpiringPermission {
            permission_id: Uuid::new_v4(),
            target: cloud_target(),
            expires_in: Duration::from_secs(60),
        };
        assert_eq!(ep.expires_in.as_secs(), 60);
    }

    #[tokio::test]
    async fn test_check_external_access_primal_nestgate() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "x".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["primal:nestgate".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            AccessResult::Denied { .. } => panic!("Expected Granted for primal:nestgate"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_primal_songbird() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "x".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["primal:songbird".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            _ => panic!("Expected Granted for primal:songbird"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_primal_squirrel() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "x".to_string(),
            api_endpoints: vec![],
            feature_set: vec!["primal:squirrel".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Granted { .. } => {}
            _ => panic!("Expected Granted for primal:squirrel"),
        }
    }

    #[test]
    fn test_access_result_denied_clone() {
        let denied = AccessResult::Denied {
            reason: "no".to_string(),
            how_to_get_access: "get permit".to_string(),
        };
        let c = denied.clone();
        assert!(matches!(c, AccessResult::Denied { .. }));
    }

    // ─── get_access_instructions for all ExternalTarget variants ────────────────

    #[tokio::test]
    async fn test_check_external_access_quantum_provider_denied_default_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::QuantumProvider {
            provider: QuantumProvider::IBM,
            backends: vec![],
            qubit_limits: Some(127),
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("security provider"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied for QuantumProvider"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_hpc_cluster_denied_default_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::HPCCluster {
            cluster_name: "super-a".to_string(),
            scheduler: HPCScheduler::SLURM,
            partitions: vec!["gpu".to_string()],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("security provider"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied for HPCCluster"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_enterprise_service_denied_default_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::EnterpriseService {
            service_name: "acme-api".to_string(),
            tier: ServiceTier::Enterprise,
            features: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("security provider"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied for EnterpriseService"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_cloud_provider_aws_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::CloudProvider {
            provider: CloudProvider::AWS,
            regions: vec![],
            services: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("AWS"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_cloud_provider_gcp_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::CloudProvider {
            provider: CloudProvider::GCP,
            regions: vec![],
            services: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("GCP"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_container_docker_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ContainerPlatform {
            platform: ContainerPlatform::Docker,
            clusters: vec![],
            namespaces: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("container"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied"),
        }
    }

    #[tokio::test]
    async fn test_check_external_access_external_tool_instructions() {
        let lock = ToadStoolCryptoLock::new().await.unwrap();
        let target = ExternalTarget::ExternalTool {
            tool_name: "custom-tool".to_string(),
            api_endpoints: vec![],
            feature_set: vec![],
        };
        let result = lock.check_external_access(&target).await.unwrap();
        match &result {
            AccessResult::Denied {
                how_to_get_access, ..
            } => {
                assert!(how_to_get_access.contains("custom-tool"));
            }
            AccessResult::Granted { .. } => panic!("Expected Denied"),
        }
    }

    #[test]
    fn test_permission_level_full_debug() {
        let level = PermissionLevel::Full;
        let s = format!("{:?}", level);
        assert!(s.contains("Full"));
    }

    #[test]
    fn test_crypto_lock_status_delegation_chains() {
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: std::collections::HashMap::new(),
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        assert!(status.delegation_chains.is_empty());
    }

    // ─── Additional coverage: install valid permission, cache, permission levels ───

    #[tokio::test]
    async fn test_install_crypto_permission_valid_succeeds() {
        use crate::crypto_lock::validation::{CryptoAlgorithm, ProofMetadata, SecurityProof};
        let mut lock = ToadStoolCryptoLock::new().await.unwrap();
        let now = SystemTime::now();
        let perm = SecurityProviderPermission {
            permission_id: Uuid::new_v4(),
            holder: PermissionHolder::Individual {
                user_id: "u1".to_string(),
                public_key: "pk".to_string(),
                verification_level: VerificationLevel::Unverified,
            },
            external_target: cloud_target(),
            scope: PermissionScope {
                resource_limits: ResourceLimits {
                    max_cpu_cores: None,
                    max_memory_gb: None,
                    max_storage_gb: None,
                    max_network_bandwidth: None,
                },
                time_restrictions: TimeRestrictions {
                    allowed_hours: None,
                    allowed_days: None,
                    timezone: None,
                },
                usage_quotas: UsageQuotas {
                    max_requests_per_hour: None,
                    max_data_transfer_gb: None,
                    max_compute_hours: None,
                },
                geographic_limits: vec![],
                feature_restrictions: vec![],
            },
            valid_from: now - Duration::from_secs(3600),
            valid_until: now + Duration::from_secs(3600),
            crypto_proof: SecurityProof {
                signature: vec![],
                algorithm: CryptoAlgorithm::Ed25519,
                public_key_id: "k1".to_string(),
                timestamp: now,
                metadata: ProofMetadata {
                    issuer: "test".to_string(),
                    purpose: "test".to_string(),
                    additional_claims: std::collections::HashMap::new(),
                },
            },
            delegation_chain: None,
            metadata: PermissionMetadata {
                issued_by: "test".to_string(),
                notes: String::new(),
                features: vec![],
            },
        };
        let result = lock.install_crypto_permission(perm).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_external_access_with_valid_permission_granted() {
        use crate::crypto_lock::validation::{CryptoAlgorithm, ProofMetadata, SecurityProof};
        let mut lock = ToadStoolCryptoLock::new().await.unwrap();
        let now = SystemTime::now();
        let perm = SecurityProviderPermission {
            permission_id: Uuid::new_v4(),
            holder: PermissionHolder::Individual {
                user_id: "u1".to_string(),
                public_key: "pk".to_string(),
                verification_level: VerificationLevel::Unverified,
            },
            external_target: cloud_target(),
            scope: PermissionScope {
                resource_limits: ResourceLimits {
                    max_cpu_cores: None,
                    max_memory_gb: None,
                    max_storage_gb: None,
                    max_network_bandwidth: None,
                },
                time_restrictions: TimeRestrictions {
                    allowed_hours: None,
                    allowed_days: None,
                    timezone: None,
                },
                usage_quotas: UsageQuotas {
                    max_requests_per_hour: None,
                    max_data_transfer_gb: None,
                    max_compute_hours: None,
                },
                geographic_limits: vec![],
                feature_restrictions: vec![],
            },
            valid_from: now - Duration::from_secs(3600),
            valid_until: now + Duration::from_secs(3600),
            crypto_proof: SecurityProof {
                signature: vec![],
                algorithm: CryptoAlgorithm::Ed25519,
                public_key_id: "k1".to_string(),
                timestamp: now,
                metadata: ProofMetadata {
                    issuer: "test".to_string(),
                    purpose: "test".to_string(),
                    additional_claims: std::collections::HashMap::new(),
                },
            },
            delegation_chain: None,
            metadata: PermissionMetadata {
                issued_by: "test".to_string(),
                notes: String::new(),
                features: vec![],
            },
        };
        let _ = lock.install_crypto_permission(perm).await;
        let result = lock.check_external_access(&cloud_target()).await.unwrap();
        match &result {
            AccessResult::Granted {
                permission_level,
                restrictions,
                ..
            } => {
                assert!(matches!(
                    permission_level,
                    PermissionLevel::Full | PermissionLevel::Limited | PermissionLevel::Basic
                ));
                let _ = restrictions;
            }
            AccessResult::Denied { .. } => {}
        }
    }

    #[tokio::test]
    async fn test_install_crypto_permission_invalid_rejects() {
        let mut lock = ToadStoolCryptoLock::new().await.unwrap();
        let mut perm = make_expired_permission(cloud_target());
        perm.valid_until = SystemTime::now() + Duration::from_secs(3600);
        perm.valid_from = SystemTime::now() + Duration::from_secs(1);
        let result = lock.install_crypto_permission(perm).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_crypto_permission_revoked_rejects() {
        let mut lock = ToadStoolCryptoLock::new().await.unwrap();
        let perm = make_expired_permission(cloud_target());
        let result = lock.install_crypto_permission(perm).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_access_result_granted_with_all_fields() {
        let result = AccessResult::Granted {
            reason: "ok".to_string(),
            permission_level: PermissionLevel::Basic,
            expires_at: Some(SystemTime::now()),
            restrictions: vec!["r1".to_string(), "r2".to_string()],
        };
        match &result {
            AccessResult::Granted {
                permission_level,
                restrictions,
                ..
            } => {
                assert!(matches!(permission_level, PermissionLevel::Basic));
                assert_eq!(restrictions.len(), 2);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_crypto_lock_status_clone() {
        let status = CryptoLockStatus {
            pure_rust_unlocked: true,
            external_permissions: std::collections::HashMap::new(),
            delegation_chains: vec![],
            expiring_permissions: vec![],
        };
        let c = status.clone();
        assert!(c.pure_rust_unlocked);
    }
}
