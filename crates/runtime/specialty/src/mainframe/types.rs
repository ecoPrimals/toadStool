//! Shared types for mainframe adapters

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::{
    AuthenticationSettings, COBOLSettings, CommunicationSettings, ConnectionSettings, 
    ConnectionType, DatasetConfig, JCLSettings, JobOutput, JobPriority, JobStatus, 
    SystemInfo, ToadStoolResult, ToadStoolError
};

/// Mainframe job representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainframeJob {
    /// Job ID
    pub job_id: Uuid,
    /// Job name
    pub job_name: String,
    /// Job class
    pub job_class: String,
    /// Job priority
    pub priority: JobPriority,
    /// JCL content
    pub jcl_content: String,
    /// Job status
    pub status: JobStatus,
    /// Start time
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub start_time: Option<SystemTime>,
    /// End time
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub end_time: Option<SystemTime>,
    /// Output datasets
    pub output_datasets: Vec<String>,
    /// Return code
    pub return_code: Option<i32>,
    /// Job log
    pub job_log: String,
}

/// JCL (Job Control Language) Generator
#[derive(Debug)]
pub struct JCLGenerator {
    /// Template library
    templates: HashMap<String, String>,
    /// JCL settings
    settings: JCLSettings,
}

/// COBOL Compiler Interface
#[derive(Debug)]
pub struct COBOLCompiler {
    /// Compiler settings
    settings: COBOLSettings,
    /// Compiler executable path
    compiler_path: PathBuf,
    /// Library paths
    library_paths: Vec<PathBuf>,
}

/// 3270 Terminal Emulator
#[derive(Debug)]
pub struct Terminal3270 {
    /// Connection settings
    connection: ConnectionSettings,
    /// Terminal session
    session: Option<Box<dyn Terminal3270Session>>,
    /// Screen buffer
    screen_buffer: Vec<Vec<char>>,
    /// Cursor position
    cursor_position: (u16, u16),
    /// Terminal attributes
    attributes: Terminal3270Attributes,
}

/// 3270 Terminal Session trait
// Native async trait - no macro needed
#[async_trait::async_trait]
pub trait Terminal3270Session: Send + Sync {
    /// Connect to mainframe
    async fn connect(&mut self, settings: &ConnectionSettings) -> ToadStoolResult<()>;
    
    /// Disconnect from mainframe
    async fn disconnect(&mut self) -> ToadStoolResult<()>;
    
    /// Send data to mainframe
    async fn send_data(&mut self, data: &[u8]) -> ToadStoolResult<()>;
    
    /// Receive data from mainframe
    async fn receive_data(&mut self, timeout: Duration) -> ToadStoolResult<Vec<u8>>;
    
    /// Send key sequence
    async fn send_key(&mut self, key: Terminal3270Key) -> ToadStoolResult<()>;
    
    /// Get screen contents
    async fn get_screen(&self) -> ToadStoolResult<String>;
    
    /// Wait for field
    async fn wait_for_field(&mut self, field_name: &str, timeout: Duration) -> ToadStoolResult<String>;
}

/// 3270 Terminal Attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Terminal3270Attributes {
    /// Screen width
    pub width: u16,
    /// Screen height
    pub height: u16,
    /// Color support
    pub color_support: bool,
    /// Extended attributes
    pub extended_attributes: bool,
}

/// 3270 Terminal Keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Terminal3270Key {
    /// Enter key
    Enter,
    /// Clear key
    Clear,
    /// Program function keys
    PF(u8),
    /// Program access keys
    PA(u8),
    /// Cursor movement
    CursorUp,
    CursorDown,
    CursorLeft,
    CursorRight,
    /// Tab keys
    Tab,
    BackTab,
    /// Other keys
    Insert,
    Delete,
    Home,
    End,
    /// String input
    String(String),
}

/// Dataset Manager
#[derive(Debug)]
pub struct DatasetManager {
    /// Dataset configurations
    datasets: HashMap<String, DatasetConfig>,
    /// Active dataset handles
    active_datasets: Arc<RwLock<HashMap<String, DatasetHandle>>>,
}

/// Dataset Handle
#[derive(Debug)]
pub struct DatasetHandle {
    /// Dataset name
    pub name: String,
    /// Dataset configuration
    pub config: DatasetConfig,
    /// File handle
    pub file_handle: Option<std::fs::File>,
    /// Record buffer
    pub record_buffer: Vec<u8>,
    /// Current record number
    pub current_record: u64,
}

/// DCL (Digital Command Language) Processor for VAX/VMS
#[derive(Debug)]
pub struct DCLProcessor {
    /// DCL command templates
    templates: HashMap<String, String>,
    /// Environment variables
    environment: HashMap<String, String>,
    /// Current directory
    current_directory: PathBuf,
}

/// VAX FORTRAN Compiler
#[derive(Debug)]
pub struct VAXFortranCompiler {
    /// Compiler path
    compiler_path: PathBuf,
    /// Compiler options
    compiler_options: Vec<String>,
    /// Library paths
    library_paths: Vec<PathBuf>,
}

/// VAX Terminal Interface
#[derive(Debug)]
pub struct VAXTerminal {
    /// Terminal type
    terminal_type: String,
    /// Terminal attributes
    attributes: VAXTerminalAttributes,
    /// Session handle
    session: Option<Box<dyn VAXTerminalSession>>,
}

/// VAX Terminal Attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VAXTerminalAttributes {
    /// Terminal width
    pub width: u16,
    /// Terminal height
    pub height: u16,
    /// Terminal capabilities
    pub capabilities: Vec<String>,
}

/// VAX Terminal Session trait
// Native async trait - no macro needed
#[async_trait::async_trait]
pub trait VAXTerminalSession: Send + Sync {
    /// Connect to VAX system
    async fn connect(&mut self, settings: &ConnectionSettings) -> ToadStoolResult<()>;
    
    /// Disconnect from VAX system
    async fn disconnect(&mut self) -> ToadStoolResult<()>;
    
    /// Execute DCL command
    async fn execute_dcl(&mut self, command: &str) -> ToadStoolResult<String>;
    
    /// Get system information
    async fn get_system_info(&self) -> ToadStoolResult<SystemInfo>;
}

/// VMS File System Manager
#[derive(Debug)]
pub struct VMSFileSystem {
    /// File specifications
    file_specs: HashMap<String, VMSFileSpec>,
    /// Directory cache
    directory_cache: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

/// VMS File Specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMSFileSpec {
    /// Device name
    pub device: String,
    /// Directory path
    pub directory: Vec<String>,
    /// File name
    pub filename: String,
    /// File type
    pub file_type: String,
    /// Version number
    pub version: Option<u32>,
}

/// RPG Compiler for AS/400
#[derive(Debug)]
pub struct RPGCompiler {
    /// Compiler path
    compiler_path: PathBuf,
    /// Compiler options
    compiler_options: Vec<String>,
    /// Source member library
    source_library: String,
    /// Object library
    object_library: String,
}

/// 5250 Terminal Emulator for AS/400
#[derive(Debug)]
pub struct Terminal5250 {
    /// Connection settings
    connection: ConnectionSettings,
    /// Terminal session
    session: Option<Box<dyn Terminal5250Session>>,
    /// Screen buffer
    screen_buffer: Vec<Vec<char>>,
    /// Field definitions
    field_definitions: Vec<Field5250>,
}

/// 5250 Terminal Session trait
// Native async trait - no macro needed
#[async_trait::async_trait]
pub trait Terminal5250Session: Send + Sync {
    /// Connect to AS/400
    async fn connect(&mut self, settings: &ConnectionSettings) -> ToadStoolResult<()>;
    
    /// Disconnect from AS/400
    async fn disconnect(&mut self) -> ToadStoolResult<()>;
    
    /// Execute command
    async fn execute_command(&mut self, command: &str) -> ToadStoolResult<String>;
    
    /// Navigate to menu
    async fn navigate_menu(&mut self, menu_option: &str) -> ToadStoolResult<()>;
    
    /// Get screen fields
    async fn get_screen_fields(&self) -> ToadStoolResult<Vec<Field5250>>;
}

/// 5250 Field Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field5250 {
    /// Field name
    pub name: String,
    /// Field position
    pub position: (u16, u16),
    /// Field length
    pub length: u16,
    /// Field type
    pub field_type: Field5250Type,
    /// Field attributes
    pub attributes: Field5250Attributes,
    /// Field value
    pub value: String,
}

/// 5250 Field Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Field5250Type {
    /// Input field
    Input,
    /// Output field
    Output,
    /// Both input and output
    Both,
    /// Hidden field
    Hidden,
}

/// 5250 Field Attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field5250Attributes {
    /// Field color
    pub color: Option<String>,
    /// Field highlighting
    pub highlighting: Option<String>,
    /// Field protection
    pub protected: bool,
    /// Field intensity
    pub intensity: Option<String>,
}

/// IFS (Integrated File System) Manager for AS/400
#[derive(Debug)]
pub struct IFSManager {
    /// IFS root paths
    root_paths: Vec<PathBuf>,
    /// File system cache
    file_cache: Arc<RwLock<HashMap<String, IFSFile>>>,
}

/// IFS File representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IFSFile {
    /// File path
    pub path: PathBuf,
    /// File size
    pub size: u64,
    /// File type
    pub file_type: String,
    /// File attributes
    pub attributes: IFSFileAttributes,
    /// Last modified time
    #[serde(with = "toadstool_common::system_time_serde")]
    pub last_modified: SystemTime,
}

/// IFS File Attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IFSFileAttributes {
    /// File permissions
    pub permissions: String,
    /// Owner
    pub owner: String,
    /// Group
    pub group: String,
    /// CCSID (Character Set ID)
    pub ccsid: Option<u32>,
}

// Implementation for IBM Mainframe Adapter
