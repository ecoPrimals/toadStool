// SPDX-License-Identifier: AGPL-3.0-only
//! # Legacy System Emulation
//!
//! Support for legacy system emulators:
//! - PDP-11 emulator
//! - Apple II emulator
//! - Commodore 64 emulator
//! - Atari 8-bit emulator
//! - CP/M emulator

use std::path::Path;
use tracing::info;

use crate::types::emulation::{EmulationConfig, EmulationStatus};
use crate::{LegacyEmulator, LegacySystemType, ToadStoolResult};

/// PDP-11 Emulator
#[derive(Debug)]
pub struct PDP11Emulator {
    name: &'static str,
    config: Option<EmulationConfig>,
    running: bool,
}

/// Apple II Emulator
#[derive(Debug)]
pub struct Apple2Emulator {
    name: &'static str,
    config: Option<EmulationConfig>,
    running: bool,
}

impl Default for PDP11Emulator {
    fn default() -> Self {
        Self {
            name: "PDP-11 Emulator",
            config: None,
            running: false,
        }
    }
}

impl PDP11Emulator {
    /// Creates a new PDP-11 emulator instance.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Apple2Emulator {
    fn default() -> Self {
        Self {
            name: "Apple II Emulator",
            config: None,
            running: false,
        }
    }
}

impl Apple2Emulator {
    /// Creates a new Apple II emulator instance.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl LegacyEmulator for PDP11Emulator {
    fn name(&self) -> &'static str {
        self.name
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![LegacySystemType::PDP11]
    }

    async fn initialize(&mut self, config: &EmulationConfig) -> ToadStoolResult<()> {
        info!("Initializing PDP-11 emulator");
        self.config = Some(config.clone());
        Ok(())
    }

    async fn start(&mut self) -> ToadStoolResult<()> {
        info!("Starting PDP-11 emulation");
        self.running = true;
        Ok(())
    }

    async fn stop(&mut self) -> ToadStoolResult<()> {
        info!("Stopping PDP-11 emulation");
        self.running = false;
        Ok(())
    }

    async fn reset(&mut self) -> ToadStoolResult<()> {
        info!("Resetting PDP-11 emulation");
        Ok(())
    }

    async fn load_image(&mut self, image: &Path) -> ToadStoolResult<()> {
        info!("Loading image into PDP-11 emulator: {:?}", image);
        Ok(())
    }

    async fn save_state(&mut self, path: &Path) -> ToadStoolResult<()> {
        info!("Saving PDP-11 emulator state to: {:?}", path);
        Ok(())
    }

    async fn load_state(&mut self, path: &Path) -> ToadStoolResult<()> {
        info!("Loading PDP-11 emulator state from: {:?}", path);
        Ok(())
    }

    async fn get_status(&self) -> ToadStoolResult<EmulationStatus> {
        if self.running {
            Ok(EmulationStatus::Running)
        } else {
            Ok(EmulationStatus::Stopped)
        }
    }
}

#[async_trait::async_trait]
impl LegacyEmulator for Apple2Emulator {
    fn name(&self) -> &'static str {
        self.name
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![LegacySystemType::AppleIi]
    }

    async fn initialize(&mut self, config: &EmulationConfig) -> ToadStoolResult<()> {
        info!("Initializing Apple II emulator");
        self.config = Some(config.clone());
        Ok(())
    }

    async fn start(&mut self) -> ToadStoolResult<()> {
        info!("Starting Apple II emulation");
        self.running = true;
        Ok(())
    }

    async fn stop(&mut self) -> ToadStoolResult<()> {
        info!("Stopping Apple II emulation");
        self.running = false;
        Ok(())
    }

    async fn reset(&mut self) -> ToadStoolResult<()> {
        info!("Resetting Apple II emulation");
        Ok(())
    }

    async fn load_image(&mut self, image: &Path) -> ToadStoolResult<()> {
        info!("Loading disk image into Apple II emulator: {:?}", image);
        Ok(())
    }

    async fn save_state(&mut self, path: &Path) -> ToadStoolResult<()> {
        info!("Saving Apple II emulator state to: {:?}", path);
        Ok(())
    }

    async fn load_state(&mut self, path: &Path) -> ToadStoolResult<()> {
        info!("Loading Apple II emulator state from: {:?}", path);
        Ok(())
    }

    async fn get_status(&self) -> ToadStoolResult<EmulationStatus> {
        if self.running {
            Ok(EmulationStatus::Running)
        } else {
            Ok(EmulationStatus::Stopped)
        }
    }
}
