// SPDX-License-Identifier: AGPL-3.0-only
//! # Mainframe System Adapters
//!
//! Support for legacy mainframe systems including:
//! - IBM System/360, System/370, z/Series
//! - VAX/VMS systems  
//! - AS/400 systems
//! - Job Control Language (JCL) processing
//! - COBOL compilation and execution
//! - 3270 terminal emulation
//! - Dataset management
//! - TSO/ISPF interface support

mod types;
mod ibm;
mod vax;
mod as400;

// Re-export public types
pub use types::*;
pub use ibm::IBMMainframeAdapter;
pub use vax::VAXVMSAdapter;
pub use as400::AS400Adapter;
