//! Universal Substrate Discovery and Management
//!
//! Deep Debt: ToadStool discovers and manages all compute substrates
//! at runtime without external tools or elevated privileges

pub mod discovery;

pub use discovery::{
    BackendType, DiscoveredSubstrate, HardwareChange, HardwareDiscovery, SubstrateCapabilities,
    SubstrateType,
};
