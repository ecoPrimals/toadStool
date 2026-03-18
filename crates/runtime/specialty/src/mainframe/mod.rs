// SPDX-License-Identifier: AGPL-3.0-or-later
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

mod as400;
mod ibm;
mod types;
mod vax;

// Re-export public types
pub use as400::AS400Adapter;
pub use ibm::IBMMainframeAdapter;
pub use types::*;
pub use vax::VAXVMSAdapter;
