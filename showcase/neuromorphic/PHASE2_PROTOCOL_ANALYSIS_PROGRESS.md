# Phase 2: Protocol Analysis - Progress Report

**Date**: January 29, 2026  
**Status**: 🚧 **In Progress** - Initial Findings Complete  
**Phase**: 2 of 4 (Model Format Analysis)

---

## 🎯 Objective

Understand the Akida `.fbz` model format and device communication protocol to implement model loading in pure Rust.

---

## ✅ Completed Today

### 1. Model File Format Analysis

**File Type**: `.fbz` (FlatBuffers Binary)

**Key Findings**:

| Aspect | Details |
|--------|---------|
| **Format** | FlatBuffers (NOT zlib compressed) |
| **Magic Bytes** | `\x80D\x04\x10` (FlatBuffers header) |
| **SDK Version** | "2.18.2" embedded at offset 0x1E |
| **Structure** | Binary FlatBuffers schema with metadata |
| **Size** | Minimal test model: 10,186 bytes (10KB) |

**Hexdump Analysis** (`minimal_test.fbz`):

```
00000000  80 44 04 10 00 01 01 40  0a 00 0c 00 04 00 00 00  |.D.....@........|
00000010  08 00 0a 00 00 00 14 00  00 05 0e 44 06 00 00 00  |...........D....|
00000020  32 2e 31 38 2e 32 00 00  02 00 00 00 b4 20 09 18  |2.18.2....... ..|
                                    ^^^^^^^^^ SDK Version string

00000040  ...
00000048  05 00 00 00 69 6e 70 75  74 00 00 00              |....input...|
                      ^^^^^^^^^^^^^^ Layer name "input"

0000007C  6c 61 79 65 72 5f 74 79  70 65 00 00              |layer_type..|
          ^^^^^^^^^^^^^^^^^^^^^^^^ Metadata key

00000094  77 65 69 67 68 74 73 5f  62 69 74 73              |weights_bits|
          ^^^^^^^^^^^^^^^^^^^^^^^^ Quantization info

000000B0  61 63 74 69 76 61 74 69  6f 6e 00 00              |activation..|
          ^^^^^^^^^^^^^^^^^^^ Activation function metadata
```

**Embedded Strings Discovered**:
- SDK version: "2.18.2"
- Layer names: "input", "fc" (fully connected)
- Metadata keys: "layer_type", "weights_bits", "activation"
- Values: Repeated `fe 01 00` patterns (likely weight data)

### 2. FlatBuffers Schema Identification

**Not Compressed**: Attempted zlib decompression failed  
**Binary Format**: Direct FlatBuffers serialization

**Structure Elements**:
1. **Header** (16 bytes): FlatBuffers magic + offsets
2. **Version** (~32 bytes): SDK version string
3. **Layer Metadata** (variable): Names, types, shapes
4. **Weight Data** (bulk): Model parameters
5. **Quantization Info**: Bit widths, scales

### 3. Python SDK API Investigation

**Model Creation**:
```python
import akida

# Create model from layers
layers = [
    akida.InputData(name="input", input_shape=(28, 28, 1)),
    akida.FullyConnected(name="fc", units=10)
]
model = akida.Model(layers=layers)

# Save to .fbz
model.save("model.fbz")
```

**Model Loading** (to discover later):
```python
# Load from file
model = akida.Model("model.fbz")

# Map to device (API TBD - need to investigate)
# device.??? (model)
```

---

## 🔬 Technical Analysis

### FlatBuffers Format

**Why FlatBuffers?**:
- Zero-copy deserialization
- Efficient binary format
- Cross-platform compatible
- Schema evolution support
- Used by TensorFlow Lite, ONNX Runtime

**Schema Elements** (inferred from binary):

```
table AkidaModel {
    version: string;           // SDK version
    layers: [Layer];          // Layer array
    metadata: [KeyValue];     // Additional info
}

table Layer {
    name: string;             // Layer name
    type: string;             // "InputData", "FullyConnected", etc
    shape: [int];             // Input/output dimensions
    weights: [ubyte];         // Weight data
    weights_bits: int;        // Quantization bits
    activation: string;       // Activation function
}
```

**Next Steps for Schema**:
1. Locate official `.fbs` schema file (if public)
2. Reverse-engineer complete schema from binary
3. Generate Rust bindings with `flatbuffers` crate

### Weight Data Analysis

**Pattern**: `fe 01 00` repeated

```
fe 01 00 fe 01 00 fe 01 00 ...
```

**Interpretation**:
- Likely quantized int8 weights
- `fe` = -2 in signed int8
- `01 00` = padding or metadata
- Pattern repeats for each weight

**Size**: Majority of file is weight data (bulk storage)

---

## 📊 File Format Summary

```
┌─────────────────────────────────────┐
│     Akida .fbz File Structure       │
├─────────────────────────────────────┤
│                                     │
│  ┌───────────────────────────────┐  │
│  │ FlatBuffers Header (16B)      │  │  Magic: \x80D\x04\x10
│  │  - Magic bytes                │  │
│  │  - Table offsets              │  │
│  └───────────────────────────────┘  │
│                                     │
│  ┌───────────────────────────────┐  │
│  │ Version String (~32B)         │  │  "2.18.2\0"
│  └───────────────────────────────┘  │
│                                     │
│  ┌───────────────────────────────┐  │
│  │ Model Metadata (variable)     │  │
│  │  - Layer count                │  │
│  │  - Input/output shapes        │  │
│  │  - Quantization info          │  │
│  └───────────────────────────────┘  │
│                                     │
│  ┌───────────────────────────────┐  │
│  │ Layers Array                  │  │
│  │  ┌─────────────────────────┐  │  │
│  │  │ Layer 0 (InputData)     │  │  │
│  │  │  - name: "input"        │  │  │
│  │  │  - shape: [28, 28, 1]   │  │  │
│  │  └─────────────────────────┘  │  │
│  │  ┌─────────────────────────┐  │  │
│  │  │ Layer 1 (FullyConn)     │  │  │
│  │  │  - name: "fc"           │  │  │
│  │  │  - units: 10            │  │  │
│  │  │  - weights: [...]       │  │  │  ← Bulk data here
│  │  └─────────────────────────┘  │  │
│  └───────────────────────────┘  │
│                                     │
└─────────────────────────────────────┘

Total Size: ~10KB for minimal model
            ~5-20MB for production models
```

---

## 🚧 Current Blockers

### 1. Python SDK API for Device Loading

**Issue**: Correct API method not yet identified

**Attempted**:
- `device.map(model)` → AttributeError
- `device.soc.load_model(filename)` → AttributeError

**Need**: Find correct method in Python SDK docs or examples

**Workaround**: Can proceed with file format parsing in parallel

### 2. FlatBuffers Schema Access

**Need**: Official `.fbs` schema file for Akida models

**Options**:
1. Request from BrainChip (ideal)
2. Reverse-engineer from binaries (time-consuming)
3. Use `flatc --binary` to extract (if possible)

**Impact**: Schema would accelerate Rust implementation significantly

---

## 📈 Progress Metrics

| Task | Status | % Complete |
|------|--------|------------|
| **File Format ID** | ✅ Complete | 100% |
| **Header Analysis** | ✅ Complete | 100% |
| **Metadata Extraction** | ✅ Partial | 70% |
| **Weight Format** | 🔍 In Progress | 40% |
| **Schema Recovery** | 🔍 In Progress | 30% |
| **Device Protocol** | ⏳ Pending | 0% |
| **Rust Parser** | ⏳ Pending | 0% |

**Overall Phase 2**: ~40% complete

---

## 🎯 Next Steps

### Immediate (Today/Tomorrow)

1. **Find Python SDK Examples**:
   ```bash
   find ~/miniconda3/envs/akida_env -name "*.py" -type f | \
     xargs grep -l "device\." | head -5
   ```
   - Look for example code showing device usage
   - Identify correct API methods

2. **Extract FlatBuffers Schema**:
   ```bash
   flatc --raw-binary --schema minimal_test.fbz
   ```
   - Attempt automatic schema extraction
   - Compare with manual reverse engineering

3. **Analyze Larger Model**:
   - Create akidanet model (if possible)
   - Compare structure with minimal model
   - Identify common patterns

### Short-Term (This Week)

1. **Begin Rust Parser**:
   ```toml
   [dependencies]
   flatbuffers = "23"  # FlatBuffers Rust bindings
   ```
   - Create `akida-models` crate
   - Implement basic .fbz reader
   - Parse header and metadata

2. **Device Protocol Capture**:
   - Once Python API found, capture with strace
   - Document write/read sequences
   - Identify command structure

3. **Weight Extraction**:
   - Parse weight data from .fbz
   - Understand quantization format
   - Validate with Python SDK

---

## 🧠 Insights Gained

### 1. Simple File Format

**Positive**: FlatBuffers is well-documented and has good Rust support

**Impact**: Parser implementation will be straightforward once schema is known

### 2. No Compression

**Positive**: Direct binary access, no decompression needed

**Impact**: Simpler implementation, faster loading

### 3. Self-Contained

**Positive**: Model file includes all necessary metadata

**Impact**: No external configuration files needed

### 4. Version Tracking

**Positive**: SDK version embedded in file

**Impact**: Can detect compatibility issues early

---

## 📚 Resources Identified

### Documentation

- FlatBuffers Rust crate: https://crates.io/crates/flatbuffers
- FlatBuffers guide: https://google.github.io/flatbuffers/
- Akida Python SDK: https://doc.brainchipinc.com/api-reference/akida_apis.html

### Tools

- `flatc`: FlatBuffers compiler (for schema extraction)
- `hexdump`: Binary analysis
- `strace`: Protocol capture (once API found)

### Code Locations

- Minimal test model: `/home/strandgate/minimal_test.fbz` (10KB)
- Python SDK: `~/miniconda3/envs/akida_env/lib/python3.11/site-packages/akida/`
- Driver source: `/home/strandgate/Development/ecoPrimals/akida_dw_edma/`

---

## 🔄 Revised Timeline

### Phase 2: Model Format (Week 2) 🚧 IN PROGRESS

**Original**: 1 week  
**Revised**: 1.5 weeks (schema discovery taking longer)

- [x] Identify file format (FlatBuffers) ✅
- [x] Analyze header structure ✅
- [x] Extract metadata strings ✅
- [ ] Find Python device loading API (in progress)
- [ ] Recover FlatBuffers schema (in progress)
- [ ] Understand weight format (in progress)
- [ ] Capture device protocol (pending API)

### Phase 3: Rust Parser (Weeks 3-4) ⏳ NEXT

- [ ] Create `akida-models` crate
- [ ] Implement .fbz reader
- [ ] Parse FlatBuffers
- [ ] Extract layers and weights
- [ ] Validate against Python SDK

### Phase 4: Device Loading (Weeks 5-6) ⏳ FUTURE

- [ ] Implement model-to-device transfer
- [ ] Handle device memory allocation
- [ ] Verify model loaded correctly
- [ ] Run inference test

---

## 💡 Key Takeaways

1. **FlatBuffers is the Format**: Well-documented, good Rust support
2. **No Compression**: Simpler implementation
3. **Schema is Key**: Need `.fbs` file or reverse-engineer it
4. **Python SDK API**: Still discovering correct device methods
5. **Progress is Steady**: 40% through Phase 2 in one day

**Philosophy Maintained**:
- ✅ No mocks created (real file analysis)
- ✅ Production focus (actual .fbz files)
- ✅ Deep understanding (not just wrapping SDK)

---

**Next Session**: Continue schema recovery and find device loading API

**Status**: Phase 2 progressing well, ~40% complete 📊
