// SPDX-License-Identifier: AGPL-3.0-or-later
pub mod relocations;
pub mod sections;
pub mod symbols;

pub use relocations::{normalize_relocations, nullify_relocations_at};
pub use sections::{strip_ksymtab, strip_ksymtab_sections};
pub use symbols::{NmResolver, SymbolResolver};

pub(crate) use symbols::resolve_symbol_file_offsets;
