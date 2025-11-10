# 🧪 **Test Template & Examples**

Quick reference for adding tests to ToadStool codebase.

---

## 📋 **Test File Template**

```rust
//! Tests for [module_name]
//!
//! This module provides comprehensive test coverage for [description].

use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    // Basic struct creation tests
    
    #[test]
    fn test_[struct_name]_creation() {
        let instance = [StructName]::new();
        assert!(instance.is_valid());
    }

    #[test]
    fn test_[struct_name]_with_values() {
        let instance = [StructName] {
            field1: value1,
            field2: value2,
        };
        assert_eq!(instance.field1, value1);
        assert_eq!(instance.field2, value2);
    }

    // Serialization tests
    
    #[test]
    fn test_[struct_name]_serialization() {
        let instance = [StructName]::default();
        let json = serde_json::to_string(&instance).expect("Failed to serialize");
        assert!(!json.is_empty());
    }

    #[test]
    fn test_[struct_name]_deserialization() {
        let json = r#"{"field1":"value1","field2":"value2"}"#;
        let instance: [StructName] = serde_json::from_str(json)
            .expect("Failed to deserialize");
        assert_eq!(instance.field1, "value1");
    }

    // Enum variant tests
    
    #[test]
    fn test_[enum_name]_variants() {
        let variant1 = [EnumName]::Variant1;
        let variant2 = [EnumName]::Variant2;
        assert_ne!(variant1, variant2);
    }

    // Default trait tests
    
    #[test]
    fn test_[struct_name]_default() {
        let instance = [StructName]::default();
        assert_eq!(instance.field1, default_value);
    }

    // Clone/Debug tests
    
    #[test]
    fn test_[struct_name]_clone() {
        let instance1 = [StructName]::new();
        let instance2 = instance1.clone();
        assert_eq!(instance1, instance2);
    }

    #[test]
    fn test_[struct_name]_debug() {
        let instance = [StructName]::new();
        let debug_str = format!("{:?}", instance);
        assert!(!debug_str.is_empty());
    }
}
```

---

## 📝 **Example: zero_config Tests**

Create `crates/cli/tests/zero_config_tests.rs`:

```rust
//! Tests for zero_config module
//!
//! Provides test coverage for system detection and configuration structs.

#[cfg(test)]
mod zero_config_tests {
    use toadstool_cli::zero_config::*;

    // CPU Info Tests
    
    #[test]
    fn test_cpu_info_creation() {
        let cpu = CPUInfo {
            architecture: "x86_64".to_string(),
            cores: 8,
            threads: 16,
            model: "AMD Ryzen 7".to_string(),
            speed_mhz: 3600,
        };
        
        assert_eq!(cpu.architecture, "x86_64");
        assert_eq!(cpu.cores, 8);
        assert_eq!(cpu.threads, 16);
    }

    #[test]
    fn test_cpu_info_serialization() {
        let cpu = CPUInfo {
            architecture: "arm64".to_string(),
            cores: 4,
            threads: 4,
            model: "Apple M1".to_string(),
            speed_mhz: 3200,
        };
        
        let json = serde_json::to_string(&cpu)
            .expect("Failed to serialize CPUInfo");
        assert!(json.contains("arm64"));
        assert!(json.contains("Apple M1"));
    }

    #[test]
    fn test_cpu_info_deserialization() {
        let json = r#"{
            "architecture": "x86_64",
            "cores": 16,
            "threads": 32,
            "model": "Intel Xeon",
            "speed_mhz": 2400
        }"#;
        
        let cpu: CPUInfo = serde_json::from_str(json)
            .expect("Failed to deserialize CPUInfo");
        
        assert_eq!(cpu.architecture, "x86_64");
        assert_eq!(cpu.cores, 16);
        assert_eq!(cpu.threads, 32);
        assert_eq!(cpu.model, "Intel Xeon");
        assert_eq!(cpu.speed_mhz, 2400);
    }

    // Memory Info Tests
    
    #[test]
    fn test_memory_info_creation() {
        let memory = MemoryInfo {
            total_bytes: 16_000_000_000,
            available_bytes: 8_000_000_000,
            swap_total_bytes: 4_000_000_000,
            swap_free_bytes: 4_000_000_000,
        };
        
        assert_eq!(memory.total_bytes, 16_000_000_000);
        assert!(memory.available_bytes <= memory.total_bytes);
    }

    #[test]
    fn test_memory_info_serialization() {
        let memory = MemoryInfo {
            total_bytes: 8_000_000_000,
            available_bytes: 4_000_000_000,
            swap_total_bytes: 2_000_000_000,
            swap_free_bytes: 2_000_000_000,
        };
        
        let json = serde_json::to_string(&memory)
            .expect("Failed to serialize MemoryInfo");
        assert!(json.contains("total_bytes"));
        assert!(json.contains("8000000000"));
    }

    // Storage Info Tests
    
    #[test]
    fn test_storage_info_creation() {
        let storage = StorageInfo {
            mount_point: "/".to_string(),
            device: "/dev/sda1".to_string(),
            filesystem: "ext4".to_string(),
            total_bytes: 500_000_000_000,
            available_bytes: 250_000_000_000,
        };
        
        assert_eq!(storage.filesystem, "ext4");
        assert!(storage.available_bytes <= storage.total_bytes);
    }

    // Network Info Tests
    
    #[test]
    fn test_network_info_creation() {
        let network = NetworkInfo {
            interface: "eth0".to_string(),
            ip_address: "192.168.1.100".to_string(),
            mac_address: "00:11:22:33:44:55".to_string(),
            speed_mbps: 1000,
        };
        
        assert_eq!(network.interface, "eth0");
        assert_eq!(network.speed_mbps, 1000);
    }

    // OS Info Tests
    
    #[test]
    fn test_os_info_creation() {
        let os = OSInfo {
            name: "Linux".to_string(),
            version: "5.15.0".to_string(),
            kernel: "5.15.0-generic".to_string(),
            distribution: Some("Ubuntu".to_string()),
        };
        
        assert_eq!(os.name, "Linux");
        assert_eq!(os.distribution, Some("Ubuntu".to_string()));
    }
}
```

---

## 🎯 **Testing Best Practices**

### **1. Test Naming**
```rust
// Good: Descriptive, clear intent
#[test]
fn test_cpu_info_with_zero_cores_returns_error()

// Bad: Vague
#[test]
fn test_cpu()
```

### **2. Test Organization**
```rust
mod tests {
    use super::*;

    // Group by functionality
    mod creation_tests { /* ... */ }
    mod serialization_tests { /* ... */ }
    mod validation_tests { /* ... */ }
    mod integration_tests { /* ... */ }
}
```

### **3. Assertions**
```rust
// Good: Specific assertions
assert_eq!(result.status, Status::Success);
assert!(result.data.is_some());

// Bad: Generic assertions
assert!(result.is_ok());
```

### **4. Test Data**
```rust
// Good: Use helpers for common data
fn create_test_cpu_info() -> CPUInfo {
    CPUInfo {
        architecture: "test_arch".to_string(),
        cores: 4,
        threads: 8,
        model: "Test CPU".to_string(),
        speed_mhz: 2000,
    }
}

#[test]
fn test_something() {
    let cpu = create_test_cpu_info();
    // Use cpu...
}
```

### **5. Error Testing**
```rust
#[test]
#[should_panic(expected = "Invalid configuration")]
fn test_invalid_config_panics() {
    let _ = Config::new_invalid();
}

#[test]
fn test_invalid_config_returns_error() {
    let result = Config::try_new_invalid();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidConfig);
}
```

---

## 📊 **Running Tests**

```bash
# Run all tests
cargo test --workspace

# Run specific module tests
cargo test --package toadstool-cli

# Run specific test
cargo test test_cpu_info_creation

# Run with output
cargo test -- --nocapture

# Run and update coverage
cargo tarpaulin --out Html --output-dir coverage
```

---

## ✅ **Test Coverage Checklist**

For each struct:
- [ ] Creation test
- [ ] Serialization test
- [ ] Deserialization test
- [ ] Default trait test (if applicable)
- [ ] Clone test (if applicable)
- [ ] Debug test
- [ ] Validation test (if applicable)

For each function:
- [ ] Happy path test
- [ ] Error path test
- [ ] Edge case test
- [ ] Boundary condition test

For each enum:
- [ ] All variant tests
- [ ] Conversion tests
- [ ] Serialization tests

---

## 🚀 **Quick Start**

1. **Copy template** to new test file
2. **Replace placeholders** with actual types
3. **Add 5-10 tests** initially
4. **Run tests**: `cargo test`
5. **Check coverage**: `cargo tarpaulin`
6. **Commit**: "Add tests for [module]"

---

**Created**: October 12, 2025  
**Purpose**: Speed up test development  
**Goal**: Make adding tests easy and consistent

