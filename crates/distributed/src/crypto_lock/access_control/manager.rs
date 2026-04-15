// SPDX-License-Identifier: AGPL-3.0-or-later
//! ToadStool Crypto Lock Manager - policy enforcement and access control

use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use tracing::{debug, error, info, warn};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool_common::platform_paths::{PathEnv, PlatformPaths};

use super::super::cache::PermissionCache;
use super::super::permissions::{
    DelegationChain, DelegationRequest, DelegationScope, DelegationStatus, ExpiringPermission,
    ExternalTarget, PermissionHolder, PermissionScope, PermissionStatus,
    SecurityProviderPermission,
};
use super::super::validation::{PermissionValidationResult, SecurityPermissionValidator};
use super::types::{AccessPolicies, AccessResult, CryptoLockStatus, PermissionLevel};
use super::validation as delegation_validation;

/// `ToadStool` Crypto Lock Manager - enforces cryptographic access control
pub struct ToadStoolCryptoLock {
    permission_validator: SecurityPermissionValidator,
    active_permissions: HashMap<ExternalTarget, SecurityProviderPermission>,
    permission_cache: PermissionCache,
    _access_policies: AccessPolicies,
}

impl ToadStoolCryptoLock {
    /// Create new crypto lock system
    pub async fn new() -> ToadStoolResult<Self> {
        info!("🔐 Initializing ToadStool crypto lock system");

        let permission_validator = SecurityPermissionValidator::new().await?;
        let active_permissions = HashMap::new();
        let permission_cache = PermissionCache::new();
        let access_policies = AccessPolicies::default();

        let mut crypto_lock = Self {
            permission_validator,
            active_permissions,
            permission_cache,
            _access_policies: access_policies,
        };

        crypto_lock.load_permissions()?;
        crypto_lock.enable_pure_rust_ecosystem()?;

        info!("✅ ToadStool crypto lock system initialized");
        Ok(crypto_lock)
    }

    /// Enable Pure Rust ecosystem - ALWAYS UNLOCKED
    fn enable_pure_rust_ecosystem(&self) -> ToadStoolResult<()> {
        info!("🔓 Pure Rust ecosystem always unlocked (no crypto needed)");
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

        if self.is_pure_rust_ecosystem(target) {
            return Ok(AccessResult::Granted {
                reason: "Pure Rust ecosystem - always unlocked".to_string(),
                permission_level: PermissionLevel::Full,
                expires_at: None,
                restrictions: vec![],
            });
        }

        if let Some(cached_result) = self.permission_cache.get(target).await
            && !cached_result.is_expired()
        {
            debug!("✅ Using cached permission result");
            return Ok(cached_result.result);
        }

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
                    reason: format!("Valid crypto permission: {}", best_permission.permission_id),
                    permission_level: self.calculate_permission_level(&best_permission),
                    expires_at: Some(best_permission.valid_until),
                    restrictions: self.extract_restrictions(&best_permission.scope),
                };

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
                    reason: "Security provider crypto permission expired".to_string(),
                    how_to_get_access: "Renew your crypto permission or request delegation"
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

    /// Install a crypto permission from the security provider
    pub async fn install_crypto_permission(
        &mut self,
        permission: SecurityProviderPermission,
    ) -> ToadStoolResult<()> {
        info!(
            "📥 Installing security provider permission: {}",
            permission.permission_id
        );

        let validation_result = self
            .permission_validator
            .validate_permission(&permission)
            .await?;

        match validation_result {
            PermissionValidationResult::Valid => {
                info!("✅ Crypto permission signature valid, installing");

                if let Some(delegation_chain) = &permission.delegation_chain {
                    self.validate_delegation_chain(delegation_chain).await?;
                }

                self.active_permissions
                    .insert(permission.external_target.clone(), permission.clone());

                self.permission_cache
                    .invalidate_for_target(&permission.external_target)
                    .await;

                info!("🎉 Crypto permission installed successfully");
                Ok(())
            }
            PermissionValidationResult::Invalid => {
                error!("❌ Invalid crypto permission signature, rejecting");
                Err(ToadStoolError::security(
                    "Invalid crypto permission signature",
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
                    "Crypto permission revoked by issuer",
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

        let base_permission = self
            .find_delegatable_permission(from_holder, target)
            .await?;

        delegation_validation::validate_delegation_request(
            from_holder,
            &base_permission,
            &delegation_scope,
            duration,
        )?;

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
            pure_rust_unlocked: true,
            external_permissions: HashMap::new(),
            delegation_chains: Vec::new(),
            expiring_permissions: Vec::new(),
        };

        for (target, permission) in &self.active_permissions {
            if let Ok(time_until_expiry) = permission.valid_until.duration_since(SystemTime::now())
                && time_until_expiry < Duration::from_secs(7 * 24 * 60 * 60)
            {
                status.expiring_permissions.push(ExpiringPermission {
                    permission_id: permission.permission_id,
                    target: target.clone(),
                    expires_in: time_until_expiry,
                });
            }

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

    fn is_pure_rust_ecosystem(&self, target: &ExternalTarget) -> bool {
        match target {
            ExternalTarget::ExternalTool {
                tool_name: _,
                feature_set,
                ..
            } => {
                if feature_set
                    .iter()
                    .any(|f| f == "ecosystem_native" || f == "ecoprimals_trusted")
                {
                    return true;
                }

                if feature_set
                    .iter()
                    .any(|f| f == "trusted" || f == "no_crypto_lock")
                {
                    return true;
                }

                const ECOSYSTEM_CAPABILITIES: &[&str] = &[
                    "primal:toadstool",
                    "capability:security",
                    "capability:storage",
                    "capability:coordination",
                    "capability:ai",
                ];
                if feature_set
                    .iter()
                    .any(|f| ECOSYSTEM_CAPABILITIES.contains(&f.as_str()))
                {
                    return true;
                }

                false
            }
            _ => false,
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
                format!("Get crypto permission for {platform:?} container platform")
            }
            ExternalTarget::ExternalTool { tool_name, .. } => {
                format!("Get security provider permission for {tool_name} external tool")
            }
            _ => "Get appropriate security provider permission for this external integration"
                .to_string(),
        }
    }

    /// Load persisted permissions into [`Self::active_permissions`] (the in-memory permission store).
    ///
    /// Reads JSON array of [`SecurityProviderPermission`] from:
    /// - `TOADSTOOL_CRYPTO_PERMISSIONS_STORE` if set, otherwise
    /// - `{toadstool_data_dir}/crypto_permissions.json` (see [`PlatformPaths::toadstool_data_dir`]).
    ///
    /// Missing file or empty file is treated as an empty store (not an error).
    fn load_permissions(&mut self) -> ToadStoolResult<()> {
        let path = std::env::var("TOADSTOOL_CRYPTO_PERMISSIONS_STORE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let env = PathEnv::from_env();
                let paths = PlatformPaths::new(&env);
                paths.toadstool_data_dir().join("crypto_permissions.json")
            });

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                debug!(
                    path = %path.display(),
                    "No persisted crypto permissions file; starting with empty permission store"
                );
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        if contents.trim().is_empty() {
            return Ok(());
        }

        let loaded: Vec<SecurityProviderPermission> = serde_json::from_str(&contents)?;
        let count = loaded.len();
        for p in loaded {
            self.active_permissions.insert(p.external_target.clone(), p);
        }
        if count > 0 {
            info!(
                count,
                path = %path.display(),
                "Loaded persisted crypto permissions into permission store"
            );
        }
        Ok(())
    }

    async fn validate_delegation_chain(&self, chain: &DelegationChain) -> ToadStoolResult<()> {
        for delegation in &chain.delegations {
            self.permission_validator
                .validate_delegation_proof(&delegation.delegation_proof)
                .await?;
        }

        delegation_validation::validate_delegation_chain_depth(chain)
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
}
