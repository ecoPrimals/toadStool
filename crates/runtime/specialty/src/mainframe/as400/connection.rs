// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::super::types::{DatasetManager, IFSManager};

use crate::{DatasetConfig, ToadStoolResult};

impl Default for DatasetManager {
    fn default() -> Self {
        Self {
            datasets: HashMap::new(),
            active_datasets: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl DatasetManager {
    /// Creates a new dataset manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Initializes with the given dataset configurations.
    ///
    /// # Errors
    ///
    /// Currently always returns `Ok`.
    pub async fn initialize(
        &mut self,
        datasets: &HashMap<String, DatasetConfig>,
    ) -> ToadStoolResult<()> {
        self.datasets.clone_from(datasets);
        Ok(())
    }
}

impl Default for IFSManager {
    fn default() -> Self {
        Self {
            root_paths: vec![PathBuf::from("/")],
            file_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl IFSManager {
    /// Creates a new IFS (Integrated File System) manager for AS/400.
    pub fn new() -> Self {
        Self::default()
    }
}
