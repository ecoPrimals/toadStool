//! Service-specific integration modules
//!
//! Each module handles integration with a specific ecosystem service,
//! providing clean separation and making it easy to add new services.

pub mod beardog;
pub mod nestgate;
pub mod songbird;

// No wildcard re-exports - use explicit module paths for clarity
// e.g., services::songbird::register() instead of wildcard imports
