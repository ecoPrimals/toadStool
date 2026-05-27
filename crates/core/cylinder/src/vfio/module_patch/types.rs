// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};

/// How to patch a function's entry point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchStrategy {
    /// Skip 5-byte ftrace call at function entry, write `0xC3` (ret) at
    /// offset +5. Works when byte+5 has no kernel relocation entry.
    RetAfterFtrace,
    /// Write `0xC3` (ret) directly at offset+0 (the function entry point).
    /// Replaces the first byte of the ftrace call/NOP preamble. Safe from
    /// relocation rejection because byte+0 is the opcode, not the
    /// displacement target (which occupies bytes 1-4).
    /// Use for functions where `RetAfterFtrace` hits a relocation at +5.
    RetAtEntry,
    /// Like `RetAtEntry` but returns 1 instead of 0. Uses `xor eax,eax;
    /// inc eax; ret` (4 bytes: `31 c0 ff c0 c3`). For nvidia functions
    /// where 0 signals failure (e.g. `nv_cap_init` returns an opaque handle).
    Ret1AtEntry,
    /// NOP a `call` instruction at a fixed byte offset from function entry.
    /// Writes `xor eax,eax; 0f 1f 00` (5 bytes: 31 c0 0f 1f 00) to make
    /// the call a no-op that returns 0 in eax. Used to suppress specific
    /// kernel API calls (e.g., __register_chrdev) inside a function we
    /// cannot fully stub.
    NopCallAt(usize),
    /// Patch a single byte at a fixed offset from function entry. Used to
    /// change an immediate argument (e.g. chrdev major number) without
    /// disturbing relocations. The call itself remains intact so the
    /// kernel module loader fills in the target address normally.
    PatchByteAt {
        /// Byte offset from function entry.
        fn_offset: usize,
        /// Expected original byte (for verification).
        expected: u8,
        /// Replacement byte.
        replacement: u8,
    },
}

/// A single function to patch in a kernel module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchTarget {
    /// Function symbol name (e.g., `gf100_gr_fini`).
    pub symbol: String,
    /// Patching strategy.
    pub strategy: PatchStrategy,
}

/// A named collection of patch targets for a specific warm handoff strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchSet {
    /// Human-readable name (e.g., "volta_warm_handoff").
    pub name: String,
    /// Module name this patch set applies to (e.g., "nouveau").
    pub module_name: String,
    /// Functions to patch.
    pub targets: Vec<PatchTarget>,
    /// Minimum patches that must succeed for the module to be considered
    /// usable. Defaults to 1 — loading an entirely unpatched copy is
    /// almost certainly wrong.
    #[serde(default = "default_min_applied")]
    pub min_applied: usize,
}

pub(crate) fn default_min_applied() -> usize { 1 }

/// Errors from module patching operations.
#[derive(Debug, thiserror::Error)]
pub enum PatchError {
    #[error("failed to read module: {0}")]
    ReadFailed(std::io::Error),

    #[error("failed to write patched module to {path}: {source}")]
    WriteFailed {
        path: String,
        source: std::io::Error,
    },

    #[error("symbol '{symbol}' not found in {module}")]
    SymbolNotFound { symbol: String, module: String },

    #[error("nm failed on {path}: {detail}")]
    NmFailed { path: String, detail: String },

    #[error("ftrace call site not found at offset {offset:#x} for {symbol} (expected 0xe8/0x90/0x00/0x0f, got {found:#04x})")]
    NoFtraceCallSite {
        symbol: String,
        offset: usize,
        found: u8,
    },

    #[error("symbol offset {offset:#x} for {symbol} exceeds module size {module_size}")]
    OffsetOutOfBounds {
        symbol: String,
        offset: usize,
        module_size: usize,
    },

    #[error("insufficient patches applied: {applied}/{total} (minimum: {min_required})")]
    InsufficientPatches {
        applied: usize,
        total: usize,
        min_required: usize,
    },
}

/// Result of patching a single function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchResult {
    /// Function symbol name.
    pub symbol: String,
    /// Whether the patch was applied.
    pub applied: bool,
    /// File offset where the patch was written.
    pub offset: Option<usize>,
    /// Detail message.
    pub detail: String,
}

/// Result of patching an entire module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulePatchResult {
    /// Path to the patched `.ko` file.
    pub patched_path: String,
    /// Source module path.
    pub source_path: String,
    /// Patch set name.
    pub patch_set: String,
    /// Per-function results.
    pub patches: Vec<PatchResult>,
    /// Number of patches successfully applied.
    pub applied_count: usize,
    /// Total number of targets.
    pub total_count: usize,
}
