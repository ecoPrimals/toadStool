// SPDX-License-Identifier: AGPL-3.0-or-later
//! VBIOS init script parsing and host-side interpreter.
//!
//! Reference: nouveau nvkm/subdev/bios/init.c (Ben Skeggs, Red Hat)

mod discovery;
mod interpreter;
mod scan;

// `discovery` is shared between the interpreter and the scanner but is not
// part of this module's outward surface; nothing outside needs to locate
// scripts independently, and exporting it would invite a third opinion.
pub use interpreter::{InterpreterStats, interpret_boot_scripts};
pub use scan::{ScriptRegWrite, extract_boot_script_writes, scan_init_script_writes};
