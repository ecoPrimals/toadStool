//! # Primal Integration Framework
//!
//! Integration framework for coordinating all 5 Primals in biomeOS:
//! - ToadStool (Universal Compute Orchestrator)
//! - Songbird (Service Mesh & Discovery)
//! - BearDog (Security & Authentication)
//! - NestGate (Storage & Data Management)
//! - Squirrel (AI Agents & MCP)

// Core modules
pub mod client;
pub mod error;
pub mod manifest;
pub mod orchestrator;
pub mod services;
pub mod types;

// Re-export main types and functionality
pub use client::*;
pub use error::*;
pub use manifest::*;
pub use orchestrator::*;
pub use services::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primal_type_conversion() {
        assert_eq!(PrimalType::ToadStool.as_str(), "toadstool");
        assert_eq!(PrimalType::Songbird.as_str(), "songbird");
        assert_eq!(PrimalType::BearDog.as_str(), "beardog");
        assert_eq!(PrimalType::NestGate.as_str(), "nestgate");
        assert_eq!(PrimalType::Squirrel.as_str(), "squirrel");
    }

    #[test]
    fn test_error_types() {
        let config_error = PrimalError::Configuration {
            message: "Test configuration error".to_string(),
        };
        assert!(config_error.to_string().contains("Configuration error"));

        let auth_error = PrimalError::Authentication {
            message: "Test authentication error".to_string(),
        };
        assert!(auth_error.to_string().contains("Authentication error"));
    }
}
