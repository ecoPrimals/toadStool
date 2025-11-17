//! DOS interface for 8086 systems
//!
//! This module provides DOS filesystem and interface support for 8086-based systems.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
    entries: Vec<u16>,
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
    pub last_modified: DateTime<Utc>,
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
    pub fn current_directory(&self) -> &PathBuf {
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
    pub fn file_system(&self) -> &DOSFileSystem {
        &self.file_system
    }

    /// Get mutable file system
    pub fn file_system_mut(&mut self) -> &mut DOSFileSystem {
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
    pub fn current_drive(&self) -> char {
        self.current_drive
    }

    /// Set current drive
    pub fn set_current_drive(&mut self, drive: char) {
        self.current_drive = drive;
    }

    /// Get FAT
    pub fn fat(&self) -> &FileAllocationTable {
        &self.fat
    }

    /// Get mutable FAT
    pub fn fat_mut(&mut self) -> &mut FileAllocationTable {
        &mut self.fat
    }
}

impl FileAllocationTable {
    /// Create a new file allocation table
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cluster_size: 512,
            root_entries: Vec::new(),
        }
    }

    /// Get cluster size
    pub fn cluster_size(&self) -> u16 {
        self.cluster_size
    }

    /// Set cluster size
    pub fn set_cluster_size(&mut self, size: u16) {
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

