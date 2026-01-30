# Getting Started: Pure Rust Akida Integration

**Status**: Ready to implement  
**Hardware**: 2x Akida AKD1000 installed and operational  
**Driver**: Kernel module loaded and working  
**Next Step**: Direct Rust driver implementation

---

## Quick Win: The Driver is Simpler Than Expected!

Good news! After analyzing the C driver source, the Akida PCIe driver uses **simple read/write operations** - no complex ioctl needed!

```c
// From akida-pcie-core.c
static const struct file_operations akida_1000_fops = {
    .owner = THIS_MODULE,
    .write = akida_write,    // Transfer data to device
    .read = akida_read,      // Transfer data from device
    .open = akida_open,
    .release = akida_release,
};
```

This means our pure Rust implementation can be much simpler than expected!

---

## Implementation Strategy: Start Simple

### Phase 1: Basic I/O (This Week!)

**Goal**: Open device, read/write data

```rust
// crates/neuromorphic/akida-driver/src/device.rs

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;

pub struct AkidaDevice {
    file: File,
    index: usize,
}

impl AkidaDevice {
    pub fn open(index: usize) -> Result<Self, std::io::Error> {
        let path = format!("/dev/akida{}", index);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)?;
        
        Ok(Self { file, index })
    }
    
    pub fn write_data(&mut self, data: &[u8]) -> Result<usize, std::io::Error> {
        self.file.write(data)
    }
    
    pub fn read_data(&mut self, buffer: &mut [u8]) -> Result<usize, std::io::Error> {
        self.file.read(buffer)
    }
}

// Usage:
let mut device = AkidaDevice::open(0)?;
device.write_data(&model_data)?;
let mut output = vec![0u8; 1024];
device.read_data(&mut output)?;
```

**That's it!** Basic I/O is just standard file operations.

---

## Current Hardware Status

### Devices Detected

```bash
$ ls -l /dev/akida*
crw-rw-rw- 1 root root 10, 121 Jan 29 14:00 /dev/akida0
crw-rw-rw- 1 root root 10, 120 Jan 29 14:00 /dev/akida1
```

### Driver Loaded

```bash
$ lsmod | grep akida
akida_pcie             73728  0
```

### PCIe Info

```
a1:00.0 Co-processor [0b40]: Brainchip Inc AKD1000 [1e7c:bca1] (rev 01)
e2:00.0 Co-processor [0b40]: Brainchip Inc AKD1000 [1e7c:bca1] (rev 01)
```

---

## Week 1 Implementation Plan

### Day 1-2: Project Setup

**Create crate structure**:
```bash
cd /home/strandgate/Development/ecoPrimals/phase1/toadStool
mkdir -p crates/neuromorphic/akida-driver/src

cargo new --lib crates/neuromorphic/akida-driver
```

**Add to workspace** `Cargo.toml`:
```toml
[workspace.members]
# ... existing members ...
"crates/neuromorphic/akida-driver",
```

**Dependencies** `crates/neuromorphic/akida-driver/Cargo.toml`:
```toml
[package]
name = "akida-driver"
version = "0.1.0"
edition = "2021"

[dependencies]
thiserror = "2"
tracing = "0.1"
libc = "0.2"          # For Unix file descriptors

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
```

### Day 3-4: Basic Device I/O

**Implement** `crates/neuromorphic/akida-driver/src/lib.rs`:

```rust
//! Pure Rust Akida PCIe driver
//!
//! Provides direct access to Akida AKD1000/1500 neuromorphic processors
//! via the `/dev/akida*` character devices.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub mod device;

#[derive(Debug, Error)]
pub enum AkidaError {
    #[error("Device not found: {0}")]
    DeviceNotFound(PathBuf),
    
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Transfer failed: {0}")]
    TransferFailed(String),
}

pub type Result<T> = std::result::Result<T, AkidaError>;

/// Akida device handle
pub struct AkidaDevice {
    file: File,
    index: usize,
    path: PathBuf,
}

impl AkidaDevice {
    /// Open Akida device by index (0, 1, 2, ...)
    pub fn open(index: usize) -> Result<Self> {
        let path = PathBuf::from(format!("/dev/akida{}", index));
        
        if !path.exists() {
            return Err(AkidaError::DeviceNotFound(path));
        }
        
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)?;
        
        tracing::info!("Opened Akida device: {:?}", path);
        
        Ok(Self { file, index, path })
    }
    
    /// Get device index
    pub fn index(&self) -> usize {
        self.index
    }
    
    /// Get device path
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    /// Write data to device (DMA transfer to Akida)
    pub fn write_data(&mut self, data: &[u8]) -> Result<usize> {
        tracing::debug!("Writing {} bytes to device {}", data.len(), self.index);
        
        self.file.write(data)
            .map_err(|e| AkidaError::TransferFailed(e.to_string()))
    }
    
    /// Read data from device (DMA transfer from Akida)
    pub fn read_data(&mut self, buffer: &mut [u8]) -> Result<usize> {
        tracing::debug!("Reading up to {} bytes from device {}", buffer.len(), self.index);
        
        self.file.read(buffer)
            .map_err(|e| AkidaError::TransferFailed(e.to_string()))
    }
    
    /// Enumerate all available Akida devices
    pub fn enumerate() -> Vec<usize> {
        (0..16)
            .filter(|&i| Path::new(&format!("/dev/akida{}", i)).exists())
            .collect()
    }
}

impl Drop for AkidaDevice {
    fn drop(&mut self) {
        tracing::info!("Closed Akida device: {:?}", self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_enumerate_devices() {
        let devices = AkidaDevice::enumerate();
        println!("Found {} Akida device(s)", devices.len());
        
        for idx in &devices {
            println!("  - /dev/akida{}", idx);
        }
    }
    
    #[test]
    fn test_open_device() {
        let devices = AkidaDevice::enumerate();
        if devices.is_empty() {
            println!("No Akida devices found, skipping test");
            return;
        }
        
        let device = AkidaDevice::open(devices[0]);
        assert!(device.is_ok());
        
        let device = device.unwrap();
        println!("Opened: {:?}", device.path());
    }
}
```

### Day 5: Testing

**Create test example**:

```bash
cargo new --example test_basic_io crates/neuromorphic/akida-driver
```

**`crates/neuromorphic/akida-driver/examples/test_basic_io.rs`**:

```rust
use akida_driver::{AkidaDevice, Result};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🧠 Akida Basic I/O Test\n");
    
    // Enumerate devices
    let devices = AkidaDevice::enumerate();
    println!("Found {} device(s)", devices.len());
    
    if devices.is_empty() {
        println!("❌ No Akida devices found!");
        println!("   Check: lsmod | grep akida");
        println!("   Check: ls -l /dev/akida*");
        return Ok(());
    }
    
    // Open first device
    let mut device = AkidaDevice::open(devices[0])?;
    println!("✅ Opened: {}", device.path().display());
    
    // Test write
    let test_data = vec![0x42u8; 1024]; // 1KB test pattern
    println!("\n📤 Writing {} bytes...", test_data.len());
    let written = device.write_data(&test_data)?;
    println!("✅ Wrote {} bytes", written);
    
    // Test read
    let mut read_buffer = vec![0u8; 1024];
    println!("\n📥 Reading {} bytes...", read_buffer.len());
    let read_bytes = device.read_data(&mut read_buffer)?;
    println!("✅ Read {} bytes", read_bytes);
    
    println!("\n🎉 Success! Device is responding to Rust I/O");
    
    Ok(())
}
```

**Run test**:

```bash
cd crates/neuromorphic/akida-driver
cargo run --example test_basic_io
```

---

## Week 2: Understanding the Protocol

Now that we have basic I/O working, we need to understand **what to write and read**.

### Reverse Engineering Strategy

**1. Analyze Python SDK behavior**:

```bash
# Activate akida environment
conda activate akida_env

# Trace system calls
strace -e trace=read,write,open,close -o /tmp/akida_trace.log python3 << 'EOF'
import akida
devices = akida.devices()
print(f"Found {len(devices)} devices")
device = devices[0]
print(device.desc)
EOF

# Analyze the trace
cat /tmp/akida_trace.log
```

**2. Look for patterns**:
- What gets written first? (Model loading?)
- What gets read back? (Inference results?)
- Transfer sizes?
- Sequence of operations?

**3. Document the protocol**:

```rust
// Protocol discovered from analysis:
//
// 1. Device Initialization:
//    - Write: Device reset command (?)
//    - Read: Device status/version
//
// 2. Model Loading:
//    - Write: Model header (size, layer count, etc.)
//    - Write: Model weights (chunked, 1KB at a time)
//    - Read: Load status confirmation
//
// 3. Inference:
//    - Write: Input tensor data
//    - Write: Execute command
//    - Read: Output tensor data
```

---

## Week 3-4: Model Format Analysis

**Goal**: Parse `.fbz` model files

### Step 1: Extract Sample Model

```python
conda activate akida_env
python3 << 'EOF'
import akida
from akida_models import mnist_cnn

# Train simple MNIST model
model = mnist_cnn()
model.save("test_mnist.fbz")
print("Saved test_mnist.fbz")
EOF
```

### Step 2: Decompress

```bash
# .fbz = FlatBuffers + zlib
# Try decompression
python3 -c "
import zlib
import sys

with open('test_mnist.fbz', 'rb') as f:
    compressed = f.read()
    
try:
    decompressed = zlib.decompress(compressed)
    with open('test_mnist.fb', 'wb') as out:
        out.write(decompressed)
    print('Decompressed successfully')
except:
    print('Not zlib compressed, copying as-is')
    with open('test_mnist.fb', 'wb') as out:
        out.write(compressed)
"

hexdump -C test_mnist.fb | head -50
```

### Step 3: Rust Parser

```rust
// crates/neuromorphic/akida-models/src/fbz.rs

use flate2::read::ZlibDecoder;
use std::io::Read;

pub struct AkidaModel {
    pub layers: Vec<Layer>,
    pub weights: Vec<u8>,
}

impl AkidaModel {
    pub fn from_file(path: &str) -> Result<Self> {
        let compressed = std::fs::read(path)?;
        
        // Decompress
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        
        // Parse (to be implemented)
        Self::parse(&decompressed)
    }
    
    fn parse(data: &[u8]) -> Result<Self> {
        // TODO: Parse FlatBuffers schema
        todo!("Model parsing")
    }
}
```

---

## Integration with ToadStool

### Replace Mock Code

**1. Update detection** (`showcase/neuromorphic/01-akida-detection/Cargo.toml`):

```toml
[dependencies]
akida-driver = { path = "../../../crates/neuromorphic/akida-driver" }
# Remove: akida-detection-demo (old mock version)
```

**2. Update bioinformatics** (`showcase/neuromorphic/02-akida-bioinformatics/src/akida_filter.rs`):

```rust
use akida_driver::AkidaDevice;

impl AkidaFilter {
    pub async fn new() -> Result<Self> {
        // Real implementation!
        let indices = AkidaDevice::enumerate();
        if indices.is_empty() {
            anyhow::bail!("No Akida boards detected");
        }
        
        let devices: Vec<_> = indices
            .into_iter()
            .map(|i| AkidaDevice::open(i))
            .collect::<Result<_, _>>()?;
        
        Ok(Self { devices })
    }
}
```

---

## Success Milestones

### ✅ Week 1: Basic I/O Working
- Can open `/dev/akida0`
- Can write test data
- Can read test data
- No crashes, clean error handling

### 🎯 Week 2: Protocol Understood
- Documented read/write sequence
- Captured Python SDK behavior
- Know what commands/data to send

### 🎯 Week 3: Model Loading
- Parse `.fbz` files
- Extract layers and weights
- Successfully transfer to device

### 🎯 Week 4: First Inference
- Load MNIST model
- Submit test input
- Get prediction output
- Validate against Python SDK

---

## Next Steps

1. **This week**: Implement basic I/O (shown above)
2. **Test on hardware**: Run examples on both devices
3. **Analyze Python SDK**: Capture protocol with strace
4. **Document findings**: Update this guide with protocol

---

## Resources

### Hardware
- 2x Akida AKD1000 at `/dev/akida0`, `/dev/akida1`
- Driver source: `/home/strandgate/Development/ecoPrimals/akida_dw_edma/`

### Software
- Python SDK: `~/miniconda3/envs/akida_env/`
- Existing code: `phase1/toadStool/showcase/neuromorphic/`
- New crate: `crates/neuromorphic/akida-driver/`

### Documentation
- Migration plan: `./PURE_RUST_AKIDA_MIGRATION_PLAN.md`
- BrainChip docs: https://doc.brainchipinc.com/

---

## Let's Build This! 🚀

The hardware is ready. The driver is simpler than expected. Pure Rust Akida integration is within reach.

**Start today with Week 1 implementation** ✅

---

**Document Version**: 1.0  
**Date**: January 29, 2026  
**Status**: Ready to implement  
**Next**: Create `akida-driver` crate and run first test
