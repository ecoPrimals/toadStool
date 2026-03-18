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
