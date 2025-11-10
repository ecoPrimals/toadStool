# 🏭 ToadStool Specialty Hardware Runtime

**Status**: 🔧 Under Development  
**Purpose**: Support for specialty hardware platforms including mainframes, embedded systems, industrial controllers, and exotic architectures

---

## 🎯 Overview

The Specialty Hardware Runtime provides ToadStool compute capabilities for platforms beyond standard x86/ARM architectures. This is a **competitive advantage** that enables universal compute across:

- **Mainframes** (IBM z/OS, System/390, AS/400)
- **Embedded Systems** (Arduino, ESP32, Raspberry Pi Pico)
- **Industrial Controllers** (PLCs, SCADA systems, RT-Linux)
- **Exotic Architectures** (RISC-V, PowerPC, SPARC, MIPS)
- **Real-Time Systems** (VxWorks, QNX, FreeRTOS)

---

## 🚀 Features

### Mainframe Support
- IBM z/OS and z/VM virtualization
- COBOL and PL/I workload execution
- JCL job submission
- CICS and IMS integration

### Embedded Systems
- Arduino (AVR, ARM Cortex-M)
- ESP32 (Xtensa, RISC-V)
- STM32 (ARM Cortex-M)
- Raspberry Pi Pico (RP2040)

### Industrial Control
- Modbus/TCP and Modbus/RTU
- Profibus and Profinet
- EtherCAT and CAN bus
- OPC UA integration

### Real-Time Systems
- Deterministic scheduling
- Priority inversion handling
- Hard real-time guarantees
- Resource reservation

---

## 📊 Status

| Platform | Status | Notes |
|----------|--------|-------|
| **Mainframes** | 🔧 In Progress | Type system modernization needed |
| **Embedded** | 🔧 In Progress | Compilation errors to fix |
| **Industrial** | 🔧 In Progress | Protocol integration planned |
| **Real-Time** | 🔧 In Progress | Scheduling algorithms ready |

---

## 🔧 Current Work

### Phase 1: Modernization (In Progress)
- [x] Rename from "legacy" to "specialty"
- [ ] Fix compilation errors (83+ identified)
- [ ] Update type system to match core patterns
- [ ] Adopt base config patterns

### Phase 2: Re-enable (Planned)
- [ ] Full workspace integration
- [ ] Comprehensive testing
- [ ] Documentation completion

### Phase 3: Production (Future)
- [ ] Customer validation
- [ ] Performance optimization
- [ ] Extended platform support

---

## 🎯 Why "Specialty" Not "Legacy"?

**"Legacy" implies**:
- ❌ Deprecated code
- ❌ Scheduled for removal
- ❌ No new features

**"Specialty" implies**:
- ✅ Specialized expertise
- ✅ Niche competitive advantage
- ✅ Professional industrial support

This runtime is a **strategic differentiator** - no other universal compute platform supports these systems!

---

## 🏗️ Architecture

```
Specialty Runtime
├── Mainframe Module
│   ├── JCL Parser
│   ├── COBOL Integration
│   └── z/OS Abstraction
├── Embedded Module
│   ├── Arduino Support
│   ├── ESP32 Support
│   └── Device Drivers
├── Industrial Module
│   ├── Protocol Handlers
│   ├── PLC Integration
│   └── SCADA Bridges
└── Real-Time Module
    ├── Deterministic Scheduler
    ├── Priority Management
    └── Resource Reservation
```

---

## 📚 Documentation

- **Design**: See `../../../specs/` for architecture specs
- **API**: Run `cargo doc --no-deps` for API documentation
- **Examples**: See `../../../examples/specialty/` (planned)

---

## 🤝 Contributing

Specialty hardware support requires domain expertise. If you have experience with:
- Mainframe systems
- Embedded development
- Industrial control
- Real-time systems

...your contributions are especially welcome!

---

**ToadStool Specialty Hardware Runtime** - *Bringing Universal Compute to Every Platform* 🏭

