// SPDX-License-Identifier: AGPL-3.0-only
//! DOS interface for 8086 systems
//!
//! This module provides DOS filesystem and interface support for 8086-based systems.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

/// DOS Interface for 8086 systems
#[derive(Debug)]
pub struct DOSInterface {
    /// DOS version
    dos_version: String,
    /// Current directory
    current_directory: PathBuf,
    /// Environment variables
    environment: HashMap<String, String>,
    /// File system
    file_system: DOSFileSystem,
}

/// DOS File System
#[derive(Debug)]
pub struct DOSFileSystem {
    /// Drive mappings
    drives: HashMap<char, PathBuf>,
    /// Current drive
    current_drive: char,
    /// File allocation table
    fat: FileAllocationTable,
}

/// File Allocation Table
#[derive(Debug)]
pub struct FileAllocationTable {
    /// FAT entries
    _entries: Vec<u16>,
    /// Cluster size
    cluster_size: u16,
    /// Root directory entries
    root_entries: Vec<DirectoryEntry>,
}

/// Directory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    /// File name
    pub name: String,
    /// File extension
    pub extension: String,
    /// File attributes
    pub attributes: u8,
    /// File size
    pub size: u32,
    /// Starting cluster
    pub start_cluster: u16,
    /// Last modified time
    #[serde(with = "toadstool_common::system_time_serde")]
    pub last_modified: SystemTime,
}

impl DOSInterface {
    /// Create a new DOS interface
    pub fn new() -> Self {
        Self {
            dos_version: "MS-DOS 6.22".to_string(),
            current_directory: PathBuf::from("C:\\"),
            environment: HashMap::new(),
            file_system: DOSFileSystem::new(),
        }
    }

    /// Get DOS version
    pub fn version(&self) -> &str {
        &self.dos_version
    }

    /// Get current directory
    pub const fn current_directory(&self) -> &PathBuf {
        &self.current_directory
    }

    /// Set current directory
    pub fn set_current_directory(&mut self, path: PathBuf) {
        self.current_directory = path;
    }

    /// Get environment variable
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.environment.get(key)
    }

    /// Set environment variable
    pub fn set_env(&mut self, key: String, value: String) {
        self.environment.insert(key, value);
    }

    /// Get file system
    pub const fn file_system(&self) -> &DOSFileSystem {
        &self.file_system
    }

    /// Get mutable file system
    pub const fn file_system_mut(&mut self) -> &mut DOSFileSystem {
        &mut self.file_system
    }
}

impl DOSFileSystem {
    /// Create a new DOS file system
    pub fn new() -> Self {
        Self {
            drives: HashMap::new(),
            current_drive: 'C',
            fat: FileAllocationTable::new(),
        }
    }

    /// Mount a drive
    pub fn mount_drive(&mut self, drive: char, path: PathBuf) {
        self.drives.insert(drive, path);
    }

    /// Unmount a drive
    pub fn unmount_drive(&mut self, drive: char) -> Option<PathBuf> {
        self.drives.remove(&drive)
    }

    /// Get current drive
    pub const fn current_drive(&self) -> char {
        self.current_drive
    }

    /// Set current drive
    pub const fn set_current_drive(&mut self, drive: char) {
        self.current_drive = drive;
    }

    /// Get FAT
    pub const fn fat(&self) -> &FileAllocationTable {
        &self.fat
    }

    /// Get mutable FAT
    pub const fn fat_mut(&mut self) -> &mut FileAllocationTable {
        &mut self.fat
    }
}

impl FileAllocationTable {
    /// Create a new file allocation table
    pub const fn new() -> Self {
        Self {
            _entries: Vec::new(),
            cluster_size: 512,
            root_entries: Vec::new(),
        }
    }

    /// Get cluster size
    pub const fn cluster_size(&self) -> u16 {
        self.cluster_size
    }

    /// Set cluster size
    pub const fn set_cluster_size(&mut self, size: u16) {
        self.cluster_size = size;
    }

    /// Add root entry
    pub fn add_root_entry(&mut self, entry: DirectoryEntry) {
        self.root_entries.push(entry);
    }

    /// Get root entries
    pub fn root_entries(&self) -> &[DirectoryEntry] {
        &self.root_entries
    }

    /// Clear root entries
    pub fn clear_root_entries(&mut self) {
        self.root_entries.clear();
    }
}

impl Default for DOSInterface {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DOSFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for FileAllocationTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn dos_interface_new_and_default_match() {
        let a = DOSInterface::new();
        let b = DOSInterface::default();
        assert_eq!(a.version(), b.version());
        assert_eq!(a.current_directory(), b.current_directory());
        assert_eq!(a.version(), "MS-DOS 6.22");
        assert_eq!(a.current_directory(), &PathBuf::from("C:\\"));
    }

    #[test]
    fn dos_interface_debug_format() {
        let s = format!("{:?}", DOSInterface::new());
        assert!(s.contains("DOSInterface"));
    }

    #[test]
    fn dos_interface_env_and_directory() {
        let mut iface = DOSInterface::new();
        assert!(iface.get_env("PATH").is_none());
        iface.set_env("PATH".to_string(), "C:\\DOS".to_string());
        assert_eq!(iface.get_env("PATH").map(String::as_str), Some("C:\\DOS"));

        let p = PathBuf::from("D:\\WORK");
        iface.set_current_directory(p.clone());
        assert_eq!(iface.current_directory(), &p);
    }

    #[test]
    fn dos_interface_file_system_accessors() {
        let mut iface = DOSInterface::new();
        assert_eq!(iface.file_system().current_drive(), 'C');
        iface.file_system_mut().set_current_drive('A');
        assert_eq!(iface.file_system().current_drive(), 'A');
    }

    #[test]
    fn dos_file_system_new_default_mount_and_fat() {
        let a = DOSFileSystem::new();
        let b = DOSFileSystem::default();
        assert_eq!(a.current_drive(), b.current_drive());

        let mut fs = DOSFileSystem::new();
        fs.mount_drive('D', PathBuf::from("/mnt/d"));
        assert_eq!(fs.unmount_drive('D'), Some(PathBuf::from("/mnt/d")));
        assert!(fs.unmount_drive('Z').is_none());

        assert_eq!(fs.fat().cluster_size(), 512);
        fs.fat_mut().set_cluster_size(1024);
        assert_eq!(fs.fat().cluster_size(), 1024);
    }

    #[test]
    fn dos_file_system_debug_format() {
        let s = format!("{:?}", DOSFileSystem::new());
        assert!(s.contains("DOSFileSystem"));
    }

    #[test]
    fn file_allocation_table_new_default_and_root_entries() {
        let a = FileAllocationTable::new();
        let b = FileAllocationTable::default();
        assert_eq!(a.cluster_size(), b.cluster_size());

        let mut fat = FileAllocationTable::new();
        assert!(fat.root_entries().is_empty());

        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let entry = DirectoryEntry {
            name: "README".to_string(),
            extension: "TXT".to_string(),
            attributes: 0x20,
            size: 1024,
            start_cluster: 2,
            last_modified: t,
        };
        fat.add_root_entry(entry);
        assert_eq!(fat.root_entries().len(), 1);
        fat.clear_root_entries();
        assert!(fat.root_entries().is_empty());
    }

    #[test]
    fn file_allocation_table_debug_format() {
        let s = format!("{:?}", FileAllocationTable::new());
        assert!(s.contains("FileAllocationTable"));
    }

    #[test]
    fn directory_entry_clone_debug_serde_roundtrip() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let entry = DirectoryEntry {
            name: "README".to_string(),
            extension: "TXT".to_string(),
            attributes: 0x20,
            size: 1024,
            start_cluster: 2,
            last_modified: t,
        };
        let c = entry.clone();
        assert_eq!(entry.name, c.name);
        assert_eq!(entry.extension, c.extension);
        assert_eq!(entry.attributes, c.attributes);
        assert_eq!(entry.size, c.size);
        assert_eq!(entry.start_cluster, c.start_cluster);
        assert_eq!(entry.last_modified, c.last_modified);

        let dbg = format!("{:?}", entry);
        assert!(dbg.contains("README"));

        let json = serde_json::to_string(&entry).unwrap();
        let back: DirectoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.name, back.name);
        assert_eq!(entry.extension, back.extension);
        assert_eq!(entry.attributes, back.attributes);
        assert_eq!(entry.size, back.size);
        assert_eq!(entry.start_cluster, back.start_cluster);
        assert_eq!(entry.last_modified, back.last_modified);
    }
}
