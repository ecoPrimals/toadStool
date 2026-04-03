// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;

use super::super::types::{COBOLCompiler, RPGCompiler};

use crate::{COBOLSettings, ToadStoolResult};

/// PATH-based compiler lookup (no hardcoded `/usr/bin` paths).
pub(super) fn find_compiler_in_path(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .and_then(|path_var| {
            std::env::split_paths(&path_var)
                .map(|dir| dir.join(name))
                .find(|candidate| candidate.is_file())
        })
        .unwrap_or_else(|| PathBuf::from(name))
}

impl Default for COBOLCompiler {
    fn default() -> Self {
        Self {
            settings: COBOLSettings {
                compiler: "IGYCRCTL".to_string(),
                compile_options: vec![],
                link_options: vec![],
                runtime_options: vec![],
            },
            compiler_path: find_compiler_in_path("cobc"),
            library_paths: vec![],
        }
    }
}

impl COBOLCompiler {
    /// Creates a new COBOL compiler with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes the compiler with COBOL settings.
    ///
    /// # Errors
    ///
    /// Currently always returns `Ok`.
    pub async fn initialize(&mut self, settings: &COBOLSettings) -> ToadStoolResult<()> {
        self.settings = settings.clone();
        Ok(())
    }
}

impl Default for RPGCompiler {
    fn default() -> Self {
        Self {
            compiler_path: std::env::var_os("TOADSTOOL_RPG_COMPILER")
                .map_or_else(|| find_compiler_in_path("CRTRPGPGM"), PathBuf::from),
            compiler_options: vec![],
            source_library: "QRPGSRC".to_string(),
            object_library: "QRPGOBJ".to_string(),
        }
    }
}

impl RPGCompiler {
    /// Creates a new RPG compiler for AS/400.
    pub fn new() -> Self {
        Self::default()
    }
}
