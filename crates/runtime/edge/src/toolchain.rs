//! # Cross-Compilation Toolchain
//!
//! Provides cross-compilation capabilities for different edge platforms and architectures.
//! Supports Arduino, ESP32, ARM, and other embedded targets.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
};

use crate::platforms::EdgePlatform;
use crate::EdgeRuntimeConfig;

/// Cross-Compilation Toolchain
pub struct CrossCompilationToolchain {
    config: EdgeRuntimeConfig,
    toolchains: Arc<RwLock<HashMap<String, ToolchainInfo>>>,
    cache: Arc<RwLock<HashMap<String, CompilationCache>>>,
}

/// Toolchain Information
#[derive(Debug, Clone)]
pub struct ToolchainInfo {
    pub name: String,
    pub target: String,
    pub compiler: String,
    pub linker: String,
    pub sysroot: Option<PathBuf>,
    pub flags: Vec<String>,
    pub environment: HashMap<String, String>,
    pub is_available: bool,
}

/// Compilation Cache Entry
#[derive(Debug, Clone)]
pub struct CompilationCache {
    pub source_hash: String,
    pub target: String,
    pub compiled_binary: Vec<u8>,
    pub compilation_time: std::time::Instant,
    pub metadata: HashMap<String, String>,
}

/// Cross-Compilation Target
#[derive(Debug, Clone)]
pub struct CompilationTarget {
    pub platform: EdgePlatform,
    pub architecture: String,
    pub toolchain: String,
    pub output_format: OutputFormat,
}

/// Output Format
#[derive(Debug, Clone)]
pub enum OutputFormat {
    Binary,
    Hex,
    Elf,
    Wasm,
    Custom(String),
}

impl CrossCompilationToolchain {
    /// Create a new cross-compilation toolchain
    pub async fn new(config: &EdgeRuntimeConfig) -> ToadStoolResult<Self> {
        info!("Initializing cross-compilation toolchain");
        
        let toolchain = Self {
            config: config.clone(),
            toolchains: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Initialize default toolchains
        toolchain.initialize_toolchains().await?;
        
        Ok(toolchain)
    }
    
    /// Initialize default toolchains
    async fn initialize_toolchains(&self) -> ToadStoolResult<()> {
        let mut toolchains = self.toolchains.write().await;
        
        // Arduino toolchain
        if let Some(arduino_toolchain) = self.detect_arduino_toolchain().await? {
            toolchains.insert("arduino".to_string(), arduino_toolchain);
        }
        
        // ESP32 toolchain
        if let Some(esp32_toolchain) = self.detect_esp32_toolchain().await? {
            toolchains.insert("esp32".to_string(), esp32_toolchain);
        }
        
        // ARM toolchain
        if let Some(arm_toolchain) = self.detect_arm_toolchain().await? {
            toolchains.insert("arm".to_string(), arm_toolchain);
        }
        
        // RISC-V toolchain
        if let Some(riscv_toolchain) = self.detect_riscv_toolchain().await? {
            toolchains.insert("riscv".to_string(), riscv_toolchain);
        }
        
        // AVR toolchain
        if let Some(avr_toolchain) = self.detect_avr_toolchain().await? {
            toolchains.insert("avr".to_string(), avr_toolchain);
        }
        
        info!("Initialized {} toolchains", toolchains.len());
        Ok(())
    }
    
    /// Cross-compile code for target platform
    pub async fn cross_compile(&self, code: &[u8], platform: &EdgePlatform) -> ToadStoolResult<Vec<u8>> {
        info!("Cross-compiling for platform: {:?}", platform);
        
        // Generate cache key
        let source_hash = format!("{:x}", md5::compute(code));
        let cache_key = format!("{}_{}", source_hash, self.get_platform_key(platform));
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                info!("Using cached compilation result");
                return Ok(cached.compiled_binary.clone());
            }
        }
        
        // Get compilation target
        let target = self.get_compilation_target(platform).await?;
        
        // Get toolchain
        let toolchain = self.get_toolchain(&target.toolchain).await?;
        
        // Compile code
        let compiled_binary = self.compile_with_toolchain(code, &target, &toolchain).await?;
        
        // Cache result
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, CompilationCache {
                source_hash,
                target: target.toolchain,
                compiled_binary: compiled_binary.clone(),
                compilation_time: std::time::Instant::now(),
                metadata: HashMap::new(),
            });
        }
        
        info!("Cross-compilation completed successfully");
        Ok(compiled_binary)
    }
    
    /// Get compilation target for platform
    async fn get_compilation_target(&self, platform: &EdgePlatform) -> ToadStoolResult<CompilationTarget> {
        match platform {
            EdgePlatform::Arduino { board, .. } => {
                Ok(CompilationTarget {
                    platform: platform.clone(),
                    architecture: "avr".to_string(),
                    toolchain: "arduino".to_string(),
                    output_format: OutputFormat::Hex,
                })
            }
            EdgePlatform::ESP32 { chip, framework } => {
                Ok(CompilationTarget {
                    platform: platform.clone(),
                    architecture: "xtensa".to_string(),
                    toolchain: "esp32".to_string(),
                    output_format: OutputFormat::Binary,
                })
            }
            EdgePlatform::RaspberryPi { model, .. } => {
                let architecture = match model {
                    crate::platforms::PiModel::Pi1 => "armv6",
                    crate::platforms::PiModel::Pi2 | crate::platforms::PiModel::Pi3 => "armv7",
                    crate::platforms::PiModel::Pi4 | crate::platforms::PiModel::Pi5 => "aarch64",
                    crate::platforms::PiModel::PiZero | crate::platforms::PiModel::PiZero2W => "armv6",
                    crate::platforms::PiModel::PiPico | crate::platforms::PiModel::PiPicoW => "armv6m",
                    _ => "armv7",
                };
                
                Ok(CompilationTarget {
                    platform: platform.clone(),
                    architecture: architecture.to_string(),
                    toolchain: "arm".to_string(),
                    output_format: OutputFormat::Elf,
                })
            }
            EdgePlatform::Microcontroller { architecture, .. } => {
                let toolchain = match architecture {
                    crate::platforms::MicrocontrollerArch::ARM => "arm",
                    crate::platforms::MicrocontrollerArch::AVR => "avr",
                    crate::platforms::MicrocontrollerArch::RISCV => "riscv",
                    _ => "generic",
                };
                
                Ok(CompilationTarget {
                    platform: platform.clone(),
                    architecture: format!("{:?}", architecture).to_lowercase(),
                    toolchain: toolchain.to_string(),
                    output_format: OutputFormat::Binary,
                })
            }
            _ => {
                Ok(CompilationTarget {
                    platform: platform.clone(),
                    architecture: "generic".to_string(),
                    toolchain: "generic".to_string(),
                    output_format: OutputFormat::Binary,
                })
            }
        }
    }
    
    /// Get toolchain by name
    async fn get_toolchain(&self, name: &str) -> ToadStoolResult<ToolchainInfo> {
        let toolchains = self.toolchains.read().await;
        toolchains.get(name)
            .cloned()
            .ok_or_else(|| ToadStoolError::not_found(
                format!("Toolchain '{}' not found", name)
            ))
    }
    
    /// Compile code with specific toolchain
    async fn compile_with_toolchain(
        &self,
        code: &[u8],
        target: &CompilationTarget,
        toolchain: &ToolchainInfo,
    ) -> ToadStoolResult<Vec<u8>> {
        if !toolchain.is_available {
            return Err(ToadStoolError::not_available(
                format!("Toolchain '{}' is not available", toolchain.name)
            ));
        }
        
        // Create temporary directory for compilation
        let temp_dir = std::env::temp_dir().join(format!("toadstool_compile_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir)?;
        
        // Write source code to file
        let source_file = temp_dir.join("source.c"); // Assume C for now
        std::fs::write(&source_file, code)?;
        
        // Determine output file name
        let output_file = match target.output_format {
            OutputFormat::Binary => temp_dir.join("output.bin"),
            OutputFormat::Hex => temp_dir.join("output.hex"),
            OutputFormat::Elf => temp_dir.join("output.elf"),
            OutputFormat::Wasm => temp_dir.join("output.wasm"),
            OutputFormat::Custom(ref ext) => temp_dir.join(format!("output.{}", ext)),
        };
        
        // Build compilation command
        let mut cmd = Command::new(&toolchain.compiler);
        
        // Add toolchain flags
        for flag in &toolchain.flags {
            cmd.arg(flag);
        }
        
        // Add target-specific flags
        match &target.platform {
            EdgePlatform::Arduino { board, .. } => {
                cmd.args(&[
                    "-mmcu=atmega328p", // Default to Uno
                    "-DF_CPU=16000000L",
                    "-Os",
                ]);
            }
            EdgePlatform::ESP32 { .. } => {
                cmd.args(&[
                    "-mlongcalls",
                    "-mtext-section-literals",
                    "-ffunction-sections",
                    "-fdata-sections",
                ]);
            }
            EdgePlatform::RaspberryPi { .. } => {
                cmd.args(&[
                    "-O2",
                    "-fPIC",
                ]);
            }
            _ => {}
        }
        
        // Add source and output files
        cmd.arg(&source_file);
        cmd.args(&["-o", output_file.to_str().unwrap()]);
        
        // Add sysroot if available
        if let Some(sysroot) = &toolchain.sysroot {
            cmd.arg("--sysroot").arg(sysroot);
        }
        
        // Set environment variables
        for (key, value) in &toolchain.environment {
            cmd.env(key, value);
        }
        
        // Execute compilation
        info!("Executing compilation command: {:?}", cmd);
        let output = cmd.output()
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to execute compiler: {}", e)
            ))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Compilation failed: {}", stderr);
            return Err(ToadStoolError::execution_error(
                format!("Compilation failed: {}", stderr)
            ));
        }
        
        // Read compiled binary
        let compiled_binary = std::fs::read(&output_file)
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to read compiled binary: {}", e)
            ))?;
        
        // Clean up temporary directory
        let _ = std::fs::remove_dir_all(&temp_dir);
        
        Ok(compiled_binary)
    }
    
    /// Detect Arduino toolchain
    async fn detect_arduino_toolchain(&self) -> ToadStoolResult<Option<ToolchainInfo>> {
        // Check for Arduino CLI
        if let Ok(output) = Command::new("arduino-cli").arg("version").output() {
            if output.status.success() {
                return Ok(Some(ToolchainInfo {
                    name: "Arduino".to_string(),
                    target: "avr".to_string(),
                    compiler: "avr-gcc".to_string(),
                    linker: "avr-ld".to_string(),
                    sysroot: None,
                    flags: vec![
                        "-c".to_string(),
                        "-g".to_string(),
                        "-Os".to_string(),
                        "-w".to_string(),
                        "-ffunction-sections".to_string(),
                        "-fdata-sections".to_string(),
                        "-MMD".to_string(),
                    ],
                    environment: HashMap::new(),
                    is_available: true,
                }));
            }
        }
        
        // Check for avr-gcc directly
        if let Ok(output) = Command::new("avr-gcc").arg("--version").output() {
            if output.status.success() {
                return Ok(Some(ToolchainInfo {
                    name: "AVR-GCC".to_string(),
                    target: "avr".to_string(),
                    compiler: "avr-gcc".to_string(),
                    linker: "avr-ld".to_string(),
                    sysroot: None,
                    flags: vec![
                        "-c".to_string(),
                        "-g".to_string(),
                        "-Os".to_string(),
                        "-w".to_string(),
                        "-ffunction-sections".to_string(),
                        "-fdata-sections".to_string(),
                    ],
                    environment: HashMap::new(),
                    is_available: true,
                }));
            }
        }
        
        debug!("Arduino toolchain not found");
        Ok(None)
    }
    
    /// Detect ESP32 toolchain
    async fn detect_esp32_toolchain(&self) -> ToadStoolResult<Option<ToolchainInfo>> {
        // Check for ESP-IDF
        if let Ok(output) = Command::new("idf.py").arg("--version").output() {
            if output.status.success() {
                return Ok(Some(ToolchainInfo {
                    name: "ESP-IDF".to_string(),
                    target: "xtensa-esp32".to_string(),
                    compiler: "xtensa-esp32-elf-gcc".to_string(),
                    linker: "xtensa-esp32-elf-ld".to_string(),
                    sysroot: None,
                    flags: vec![
                        "-c".to_string(),
                        "-g".to_string(),
                        "-O2".to_string(),
                        "-ffunction-sections".to_string(),
                        "-fdata-sections".to_string(),
                    ],
                    environment: HashMap::new(),
                    is_available: true,
                }));
            }
        }
        
        // Check for xtensa toolchain directly
        if let Ok(output) = Command::new("xtensa-esp32-elf-gcc").arg("--version").output() {
            if output.status.success() {
                return Ok(Some(ToolchainInfo {
                    name: "Xtensa ESP32".to_string(),
                    target: "xtensa-esp32".to_string(),
                    compiler: "xtensa-esp32-elf-gcc".to_string(),
                    linker: "xtensa-esp32-elf-ld".to_string(),
                    sysroot: None,
                    flags: vec![
                        "-c".to_string(),
                        "-g".to_string(),
                        "-O2".to_string(),
                        "-mlongcalls".to_string(),
                        "-mtext-section-literals".to_string(),
                    ],
                    environment: HashMap::new(),
                    is_available: true,
                }));
            }
        }
        
        debug!("ESP32 toolchain not found");
        Ok(None)
    }
    
    /// Detect ARM toolchain
    async fn detect_arm_toolchain(&self) -> ToadStoolResult<Option<ToolchainInfo>> {
        // Check for ARM GCC
        if let Ok(output) = Command::new("arm-linux-gnueabihf-gcc").arg("--version").output() {
            if output.status.success() {
                return Ok(Some(ToolchainInfo {
                    name: "ARM Linux".to_string(),
                    target: "arm-linux-gnueabihf".to_string(),
                    compiler: "arm-linux-gnueabihf-gcc".to_string(),
                    linker: "arm-linux-gnueabihf-ld".to_string(),
                    sysroot: None,
                    flags: vec![
                        "-c".to_string(),
                        "-g".to_string(),
                        "-O2".to_string(),
                        "-fPIC".to_string(),
                    ],
                    environment: HashMap::new(),
                    is_available: true,
                }));
            }
        }
        
        // Check for embedded ARM toolchain
        if let Ok(output) = Command::new("arm-none-eabi-gcc").arg("--version").output() {
            if output.status.success() {
                return Ok(Some(ToolchainInfo {
                    name: "ARM Embedded".to_string(),
                    target: "arm-none-eabi".to_string(),
                    compiler: "arm-none-eabi-gcc".to_string(),
                    linker: "arm-none-eabi-ld".to_string(),
                    sysroot: None,
                    flags: vec![
                        "-c".to_string(),
                        "-g".to_string(),
                        "-Os".to_string(),
                        "-ffunction-sections".to_string(),
                        "-fdata-sections".to_string(),
                    ],
                    environment: HashMap::new(),
                    is_available: true,
                }));
            }
        }
        
        debug!("ARM toolchain not found");
        Ok(None)
    }
    
    /// Detect RISC-V toolchain
    async fn detect_riscv_toolchain(&self) -> ToadStoolResult<Option<ToolchainInfo>> {
        if let Ok(output) = Command::new("riscv64-unknown-elf-gcc").arg("--version").output() {
            if output.status.success() {
                return Ok(Some(ToolchainInfo {
                    name: "RISC-V".to_string(),
                    target: "riscv64-unknown-elf".to_string(),
                    compiler: "riscv64-unknown-elf-gcc".to_string(),
                    linker: "riscv64-unknown-elf-ld".to_string(),
                    sysroot: None,
                    flags: vec![
                        "-c".to_string(),
                        "-g".to_string(),
                        "-Os".to_string(),
                        "-ffunction-sections".to_string(),
                        "-fdata-sections".to_string(),
                    ],
                    environment: HashMap::new(),
                    is_available: true,
                }));
            }
        }
        
        debug!("RISC-V toolchain not found");
        Ok(None)
    }
    
    /// Detect AVR toolchain
    async fn detect_avr_toolchain(&self) -> ToadStoolResult<Option<ToolchainInfo>> {
        if let Ok(output) = Command::new("avr-gcc").arg("--version").output() {
            if output.status.success() {
                return Ok(Some(ToolchainInfo {
                    name: "AVR".to_string(),
                    target: "avr".to_string(),
                    compiler: "avr-gcc".to_string(),
                    linker: "avr-ld".to_string(),
                    sysroot: None,
                    flags: vec![
                        "-c".to_string(),
                        "-g".to_string(),
                        "-Os".to_string(),
                        "-ffunction-sections".to_string(),
                        "-fdata-sections".to_string(),
                    ],
                    environment: HashMap::new(),
                    is_available: true,
                }));
            }
        }
        
        debug!("AVR toolchain not found");
        Ok(None)
    }
    
    /// Get platform key for caching
    fn get_platform_key(&self, platform: &EdgePlatform) -> String {
        match platform {
            EdgePlatform::Arduino { board, version } => {
                format!("arduino_{:?}_{}", board, version)
            }
            EdgePlatform::ESP32 { chip, framework } => {
                format!("esp32_{:?}_{:?}", chip, framework)
            }
            EdgePlatform::RaspberryPi { model, os } => {
                format!("raspberry_pi_{:?}_{:?}", model, os)
            }
            EdgePlatform::Microcontroller { architecture, vendor, model } => {
                format!("microcontroller_{:?}_{}_{}", architecture, vendor, model)
            }
            _ => "generic".to_string(),
        }
    }
    
    /// Get available toolchains
    pub async fn get_available_toolchains(&self) -> Vec<ToolchainInfo> {
        let toolchains = self.toolchains.read().await;
        toolchains.values().cloned().collect()
    }
    
    /// Clear compilation cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Compilation cache cleared");
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> HashMap<String, u64> {
        let cache = self.cache.read().await;
        let mut stats = HashMap::new();
        
        stats.insert("total_entries".to_string(), cache.len() as u64);
        
        let mut total_size = 0;
        for entry in cache.values() {
            total_size += entry.compiled_binary.len();
        }
        stats.insert("total_size_bytes".to_string(), total_size as u64);
        
        stats
    }
} 