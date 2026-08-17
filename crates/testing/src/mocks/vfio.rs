// SPDX-License-Identifier: AGPL-3.0-or-later
//! Mock VFIO BAR0 access for headless CI testing.
//!
//! Simulates a VFIO-attached GPU's BAR0 register space with configurable
//! register values, error injection, and access logging.
//!
//! All public methods that access internal mutexes will panic if a mutex is
//! poisoned (another thread panicked while holding it). This is acceptable
//! for test infrastructure — poisoned state means the test has already failed.
#![allow(clippy::expect_used)]

use hw_learn::applicator::RegisterAccess;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// Operation type for access log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessOp {
    /// Register read.
    Read,
    /// Register write.
    Write,
}

/// Log entry for a register access.
#[derive(Debug, Clone)]
pub struct RegisterAccessEntry {
    /// Operation (read or write).
    pub op: AccessOp,
    /// BAR0-relative offset.
    pub offset: u64,
    /// Value read or written.
    pub value: u32,
    /// Time of access.
    pub timestamp: Instant,
}

/// Injectable errors for testing error paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockVfioError {
    /// Read fault.
    ReadFault,
    /// Write fault.
    WriteFault,
    /// Timeout.
    Timeout,
    /// Device reset.
    DeviceReset,
}

const POISONED: &str = "mock mutex poisoned";

/// Mock VFIO device BAR0 register space for headless CI testing.
pub struct MockVfioDevice {
    bdf: String,
    registers: Mutex<HashMap<u64, u32>>,
    default_value: Mutex<u32>,
    access_log: Mutex<Vec<RegisterAccessEntry>>,
    error_at: Mutex<HashMap<u64, MockVfioError>>,
}

impl MockVfioDevice {
    /// Create a new mock VFIO device for the given BDF.
    #[must_use]
    pub fn new(bdf: &str) -> Self {
        Self {
            bdf: bdf.to_string(),
            registers: Mutex::new(HashMap::new()),
            default_value: Mutex::new(0),
            access_log: Mutex::new(Vec::new()),
            error_at: Mutex::new(HashMap::new()),
        }
    }

    /// PCI BDF address.
    #[must_use]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }

    /// Read a 32-bit register at the given offset.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned.
    #[must_use]
    pub fn read_register(&self, offset: u64) -> u32 {
        if let Some(err) = self.error_at.lock().expect(POISONED).get(&offset)
            && (*err == MockVfioError::ReadFault || *err == MockVfioError::DeviceReset)
        {
            return 0;
        }

        let value = self
            .registers
            .lock()
            .expect(POISONED)
            .get(&offset)
            .copied()
            .unwrap_or_else(|| *self.default_value.lock().expect(POISONED));

        self.access_log
            .lock()
            .expect(POISONED)
            .push(RegisterAccessEntry {
                op: AccessOp::Read,
                offset,
                value,
                timestamp: Instant::now(),
            });

        value
    }

    /// Write a 32-bit register at the given offset.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned.
    pub fn write_register(&self, offset: u64, value: u32) {
        if let Some(err) = self.error_at.lock().expect(POISONED).get(&offset)
            && (*err == MockVfioError::WriteFault || *err == MockVfioError::DeviceReset)
        {
            return;
        }

        self.registers.lock().expect(POISONED).insert(offset, value);
        self.access_log
            .lock()
            .expect(POISONED)
            .push(RegisterAccessEntry {
                op: AccessOp::Write,
                offset,
                value,
                timestamp: Instant::now(),
            });
    }

    /// Return all register accesses for verification.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned.
    #[must_use]
    pub fn access_log(&self) -> Vec<RegisterAccessEntry> {
        self.access_log.lock().expect(POISONED).clone()
    }

    /// Clear the access log.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned.
    pub fn clear_access_log(&self) {
        self.access_log.lock().expect(POISONED).clear();
    }

    /// Inject an error at the given offset for subsequent accesses.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned.
    pub fn inject_error_at(&self, offset: u64, error: MockVfioError) {
        self.error_at.lock().expect(POISONED).insert(offset, error);
    }

    /// Clear all injected errors.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned.
    pub fn clear_errors(&self) {
        self.error_at.lock().expect(POISONED).clear();
    }

    /// Set default value returned for unset registers.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned.
    pub fn set_default_value(&self, value: u32) {
        *self.default_value.lock().expect(POISONED) = value;
    }

    /// Bulk load register values from a dump.
    ///
    /// # Panics
    ///
    /// Panics if an internal mutex is poisoned.
    pub fn load_register_dump(&self, dump: &[(u64, u32)]) {
        let mut regs = self.registers.lock().expect(POISONED);
        for &(offset, value) in dump {
            regs.insert(offset, value);
        }
    }
}

impl RegisterAccess for MockVfioDevice {
    fn read_u32(&self, offset: u64) -> Result<u32, String> {
        if let Some(err) = self.error_at.lock().expect(POISONED).get(&offset) {
            match err {
                MockVfioError::ReadFault => return Err("read fault".to_string()),
                MockVfioError::Timeout => return Err("timeout".to_string()),
                MockVfioError::DeviceReset => return Err("device reset".to_string()),
                MockVfioError::WriteFault => {}
            }
        }

        let value = self.read_register(offset);
        Ok(value)
    }

    fn write_u32(&mut self, offset: u64, value: u32) -> Result<(), String> {
        if let Some(err) = self.error_at.lock().expect(POISONED).get(&offset) {
            match err {
                MockVfioError::WriteFault => return Err("write fault".to_string()),
                MockVfioError::Timeout => return Err("timeout".to_string()),
                MockVfioError::DeviceReset => return Err("device reset".to_string()),
                MockVfioError::ReadFault => {}
            }
        }

        self.write_register(offset, value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_read_write() {
        let dev = MockVfioDevice::new("0000:65:00.0");
        dev.write_register(0x100, 0xDEAD_BEEF);
        assert_eq!(dev.read_register(0x100), 0xDEAD_BEEF);
    }

    #[test]
    fn access_logging() {
        let dev = MockVfioDevice::new("0000:01:00.0");
        dev.write_register(0x200, 0x1111_1111);
        let _ = dev.read_register(0x200);
        let log = dev.access_log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].op, AccessOp::Write);
        assert_eq!(log[0].offset, 0x200);
        assert_eq!(log[0].value, 0x1111_1111);
        assert_eq!(log[1].op, AccessOp::Read);
        assert_eq!(log[1].value, 0x1111_1111);
    }

    #[test]
    fn error_injection_read_fault() {
        let dev = MockVfioDevice::new("0000:02:00.0");
        dev.load_register_dump(&[(0x300, 0x42)]);
        dev.inject_error_at(0x300, MockVfioError::ReadFault);
        let result = dev.read_u32(0x300);
        assert!(result.is_err());
        assert!(result.expect_err("should be error").contains("read fault"));
    }

    #[test]
    fn error_injection_write_fault() {
        let mut dev = MockVfioDevice::new("0000:03:00.0");
        dev.inject_error_at(0x400, MockVfioError::WriteFault);
        let result = dev.write_u32(0x400, 0x1234_5678);
        assert!(result.is_err());
    }

    #[test]
    fn bulk_load_and_default_value() {
        let dev = MockVfioDevice::new("0000:04:00.0");
        dev.set_default_value(0xAAAA_AAAA);
        dev.load_register_dump(&[(0x100, 0x1111_1111), (0x200, 0x2222_2222)]);

        assert_eq!(dev.read_register(0x100), 0x1111_1111);
        assert_eq!(dev.read_register(0x200), 0x2222_2222);
        assert_eq!(dev.read_register(0x999), 0xAAAA_AAAA);
    }

    #[test]
    fn register_access_trait() {
        let mut dev = MockVfioDevice::new("0000:05:00.0");
        dev.write_u32(0x500, 0x5555_5555).expect("write");
        let val = dev.read_u32(0x500).expect("read");
        assert_eq!(val, 0x5555_5555);
    }

    #[test]
    fn clear_access_log() {
        let dev = MockVfioDevice::new("0000:06:00.0");
        dev.write_register(0x100, 1);
        assert!(!dev.access_log().is_empty());
        dev.clear_access_log();
        assert!(dev.access_log().is_empty());
    }
}
