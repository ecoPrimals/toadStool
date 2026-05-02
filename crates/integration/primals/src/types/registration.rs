// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};

/// Registration request for a primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalRegistration {
    pub primal_id: String,
    pub primal_type: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub metadata: std::collections::HashMap<String, String>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub registered_at: std::time::SystemTime,
}

/// Registration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResponse {
    pub success: bool,
    pub primal_id: String,
    pub token: Option<String>,
    pub error: Option<String>,
}

/// Registry for managing primal registrations
pub struct PrimalRegistry {
    registrations: std::collections::HashMap<String, PrimalRegistration>,
}

impl Default for PrimalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PrimalRegistry {
    pub fn new() -> Self {
        Self {
            registrations: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, registration: PrimalRegistration) -> RegistrationResponse {
        let primal_id = registration.primal_id.clone();
        self.registrations.insert(primal_id.clone(), registration);

        RegistrationResponse {
            success: true,
            primal_id,
            token: None,
            error: None,
        }
    }

    pub fn get_registration(&self, primal_id: &str) -> Option<&PrimalRegistration> {
        self.registrations.get(primal_id)
    }

    pub fn unregister(&mut self, primal_id: &str) -> bool {
        self.registrations.remove(primal_id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::SystemTime;

    fn sample_registration(id: &str) -> PrimalRegistration {
        PrimalRegistration {
            primal_id: id.to_string(),
            primal_type: "compute".to_string(),
            endpoint: "unix:///tmp/test.sock".to_string(),
            capabilities: vec!["compute.dispatch".to_string()],
            metadata: HashMap::new(),
            registered_at: SystemTime::now(),
        }
    }

    #[test]
    fn registry_default_is_empty() {
        let reg = PrimalRegistry::default();
        assert!(reg.get_registration("any").is_none());
    }

    #[test]
    fn register_and_get() {
        let mut reg = PrimalRegistry::new();
        let resp = reg.register(sample_registration("primal-1"));
        assert!(resp.success);
        assert_eq!(resp.primal_id, "primal-1");
        assert!(resp.error.is_none());

        let entry = reg.get_registration("primal-1").unwrap();
        assert_eq!(entry.primal_type, "compute");
        assert_eq!(entry.capabilities, vec!["compute.dispatch"]);
    }

    #[test]
    fn unregister_returns_true_when_present() {
        let mut reg = PrimalRegistry::new();
        reg.register(sample_registration("primal-1"));
        assert!(reg.unregister("primal-1"));
        assert!(reg.get_registration("primal-1").is_none());
    }

    #[test]
    fn unregister_returns_false_when_absent() {
        let mut reg = PrimalRegistry::new();
        assert!(!reg.unregister("nonexistent"));
    }

    #[test]
    fn register_overwrites_existing() {
        let mut reg = PrimalRegistry::new();
        reg.register(sample_registration("primal-1"));
        let mut updated = sample_registration("primal-1");
        updated.primal_type = "storage".to_string();
        reg.register(updated);
        assert_eq!(
            reg.get_registration("primal-1").unwrap().primal_type,
            "storage"
        );
    }

    #[test]
    fn registration_roundtrips_serde() {
        let reg = sample_registration("serde-test");
        let json = serde_json::to_string(&reg).unwrap();
        let back: PrimalRegistration = serde_json::from_str(&json).unwrap();
        assert_eq!(back.primal_id, "serde-test");
        assert_eq!(back.capabilities, vec!["compute.dispatch"]);
    }

    #[test]
    fn response_roundtrips_serde() {
        let resp = RegistrationResponse {
            success: true,
            primal_id: "test".to_string(),
            token: Some("tok-123".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: RegistrationResponse = serde_json::from_str(&json).unwrap();
        assert!(back.success);
        assert_eq!(back.token.as_deref(), Some("tok-123"));
    }
}
