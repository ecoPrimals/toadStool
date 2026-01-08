# Local AI Model Across GPUs - Research & Plan

**Date**: January 8, 2026  
**Hardware**: NVIDIA RTX 3090 (24 GB) + AMD RX 6950 XT (16 GB) = 40 GB Total  
**Goal**: Run large language models that require both GPUs  
**Status**: 🔬 RESEARCH PHASE

---

## 🎯 Mission

**Demonstrate real AI models running across vendor boundaries**:
1. **Small Model** (< 16 GB): Show it works on single GPU
2. **Split Model** (16-24 GB): Show splitting across both GPUs
3. **Large Model** (24-40 GB): Model that REQUIRES both GPUs

---

## 🖥️ Hardware Capacity Analysis

### Available Resources

**GPU 1: NVIDIA RTX 3090**
```
VRAM:           24 GB (24,576 MB)
Available:      ~22 GB (accounting for overhead)
Best for:       Large model layers
```

**GPU 2: AMD RX 6950 XT**
```
VRAM:           16 GB (16,384 MB)
Available:      ~14 GB (accounting for overhead)
Best for:       Smaller model layers or embeddings
```

**Combined**:
```
Total VRAM:     40 GB (40,960 MB)
Usable:         ~36 GB (accounting for overhead)
Advantage:      Can run models impossible on single GPU!
```

---

## 📊 Model Size Reference

### Precision Impact

**FP32 (Full Precision)**:
- Size: 4 bytes per parameter
- Example: 7B model = 28 GB
- Quality: Best
- Fit: Won't fit most models

**FP16 (Half Precision)**:
- Size: 2 bytes per parameter
- Example: 7B model = 14 GB
- Quality: Excellent
- Fit: Good for most models

**INT8 (8-bit Quantization)**:
- Size: 1 byte per parameter
- Example: 7B model = 7 GB
- Quality: Very good
- Fit: 2x more models

**INT4 (4-bit Quantization)**:
- Size: 0.5 bytes per parameter
- Example: 7B model = 3.5 GB
- Quality: Good for inference
- Fit: 4x more models

### Overhead Estimate

```
Model size = parameters × bytes_per_param × 1.2 (overhead)

Example (7B model in FP16):
7B × 2 bytes × 1.2 = 16.8 GB
```

---

## 🤖 Model Candidates

### Tier 1: Single GPU Models (< 16 GB)

**Perfect for testing infrastructure**

#### 1. Mistral 7B (Recommended Start)
```
Parameters:     7B
Size (FP16):    ~14 GB
Size (INT8):    ~7 GB
VRAM Needed:    16 GB (FP16) or 8 GB (INT8)
Fits on:        AMD RX 6950 XT ✅
Quality:        Excellent (SOTA for 7B)
Use case:       Chat, instruction following
HF Model:       mistralai/Mistral-7B-Instruct-v0.2
```

**Why Start Here**:
- Fits comfortably on AMD GPU (16 GB)
- Excellent quality for size
- Well-tested and documented
- Good baseline for comparison

#### 2. LLaMA-2 7B
```
Parameters:     7B
Size (FP16):    ~14 GB
Size (INT8):    ~7 GB
VRAM Needed:    16 GB (FP16) or 8 GB (INT8)
Fits on:        AMD RX 6950 XT ✅
Quality:        Excellent
Use case:       General purpose, chat
HF Model:       meta-llama/Llama-2-7b-chat-hf
```

#### 3. Phi-3 Mini
```
Parameters:     3.8B
Size (FP16):    ~8 GB
VRAM Needed:    10 GB
Fits on:        AMD RX 6950 XT ✅✅
Quality:        Excellent for size
Use case:       Chat, reasoning
HF Model:       microsoft/Phi-3-mini-4k-instruct
```

### Tier 2: Split Models (16-24 GB)

**Show splitting across GPUs**

#### 4. LLaMA-2 13B
```
Parameters:     13B
Size (FP16):    ~26 GB
Size (INT8):    ~13 GB
VRAM Needed:    28 GB (FP16) or 15 GB (INT8)
Fits on:        Single GPU? NO ❌ (FP16), YES ✅ (INT8 on NVIDIA)
Split config:   YES ✅ (FP16 split, or INT8 for demo)
Quality:        Excellent
Use case:       Better reasoning than 7B
HF Model:       meta-llama/Llama-2-13b-chat-hf
```

**Split Strategy** (FP16):
```
NVIDIA (24 GB):  Layers 1-25 (~16 GB)
AMD (16 GB):     Layers 26-40 (~10 GB)
Transfer:        Layer 25 → Layer 26
```

#### 5. Mixtral 8x7B (MoE)
```
Parameters:     46.7B (sparse, ~12.9B active)
Size (FP16):    ~90 GB (but sparse!)
Size (INT8):    ~45 GB
Size (INT4):    ~23 GB
VRAM Needed:    Variable by implementation
Fits on:        Single GPU? NO ❌
Split config:   YES ✅ (with quantization)
Quality:        Excellent (SOTA MoE)
Use case:       High quality with manageable size
HF Model:       mistralai/Mixtral-8x7B-Instruct-v0.1
```

**Why Interesting**:
- Mixture of Experts (only activates subset)
- Can fit in ~24 GB with clever quantization
- Shows advanced model architecture

### Tier 3: Large Models (24-40 GB)

**REQUIRE both GPUs - impossible on single GPU!**

#### 6. LLaMA-2 70B (4-bit Quantized) ⭐ **PRIMARY TARGET**
```
Parameters:     70B
Size (FP16):    ~140 GB (too large!)
Size (INT8):    ~70 GB (too large!)
Size (INT4):    ~35 GB ✅✅✅
VRAM Needed:    ~38 GB (with overhead)
Fits on:        Single GPU? NO ❌❌
Split config:   REQUIRES BOTH GPUs ✅✅
Quality:        Excellent (competitive with GPT-3.5)
Use case:       High-quality chat, reasoning
HF Model:       meta-llama/Llama-2-70b-chat-hf (quantize to 4-bit)
```

**Split Strategy** (4-bit quantized):
```
Total Size:      ~35 GB (4-bit quantized)
NVIDIA (24 GB):  Layers 1-50 (~20 GB)
AMD (16 GB):     Layers 51-80 (~15 GB)
Transfer:        Layer 50 → Layer 51 (activations only)
Result:          70B parameter model running locally! 🚀
```

**Why This Is THE Goal**:
- 70B parameters (10x larger than 7B)
- Impossible on single consumer GPU
- Competitive with GPT-3.5 in quality
- Demonstrates real value of multi-GPU

#### 7. Falcon 40B (8-bit Quantized)
```
Parameters:     40B
Size (FP16):    ~80 GB
Size (INT8):    ~40 GB
VRAM Needed:    ~44 GB (with overhead - too large!)
Fits on:        Single GPU? NO ❌
Split config:   Tight fit, but possible
Quality:        Excellent
Use case:       Alternative to LLaMA-2 70B
HF Model:       tiiuae/falcon-40b-instruct
```

#### 8. CodeLLaMA 34B (4-bit)
```
Parameters:     34B
Size (INT4):    ~17 GB (fits single!)
Size (INT8):    ~34 GB
VRAM Needed:    ~38 GB (INT8 split)
Fits on:        Single GPU? YES ✅ (INT4)
Split config:   YES ✅ (INT8 for quality)
Quality:        Excellent for code
Use case:       Code generation, completion
HF Model:       codellama/CodeLlama-34b-Instruct-hf
```

---

## 🛠️ Technical Approaches

### Approach 1: PyTorch + Accelerate (Recommended)

**Library**: Hugging Face `accelerate` + `transformers`

**Advantages**:
- Official HuggingFace support
- Device map for automatic splitting
- Works with most models
- Well-documented

**Setup**:
```python
from transformers import AutoModelForCausalLM, AutoTokenizer
import torch

# Load model with device map
model = AutoModelForCausalLM.from_pretrained(
    "meta-llama/Llama-2-70b-chat-hf",
    load_in_4bit=True,  # 4-bit quantization
    device_map="auto",   # Automatic GPU splitting
    torch_dtype=torch.float16,
)

# Device map will look like:
# {
#   'model.embed_tokens': 0 (NVIDIA),
#   'model.layers.0-50': 0 (NVIDIA),
#   'model.layers.51-80': 1 (AMD),
#   'model.norm': 1 (AMD),
#   'lm_head': 1 (AMD),
# }
```

**Transfer Handling**:
- Automatic via `accelerate`
- Activations transferred between layers
- Minimal overhead

### Approach 2: DeepSpeed (Advanced)

**Library**: Microsoft DeepSpeed

**Advantages**:
- More control over splitting
- Better memory optimization
- Can use ZeRO-Offload

**Complexity**: Higher
**Recommendation**: Use if `accelerate` insufficient

### Approach 3: vLLM (Performance)

**Library**: vLLM (optimized inference)

**Advantages**:
- Extremely fast inference
- PagedAttention for memory efficiency
- Production-ready

**Note**: May require more work for AMD GPU

### Approach 4: Custom Rust Implementation (Future)

**Approach**: Use our ToadStool infrastructure

**Status**: Ambitious, but possible
**Libraries**:
- `candle` (HuggingFace Rust ML)
- `burn` (Rust ML framework)
- Our GPU abstractions

**Timeline**: Longer-term goal

---

## 📋 Recommended Implementation Plan

### Phase 1: Proof of Concept (2-3 hours)

**Goal**: Get Mistral 7B running on single GPU

**Steps**:
1. Install Python dependencies
   ```bash
   pip install transformers accelerate bitsandbytes torch
   ```

2. Download model (with HF token)
   ```python
   from huggingface_hub import login
   login(token="your_token_here")
   ```

3. Load and test on AMD GPU
   ```python
   model = AutoModelForCausalLM.from_pretrained(
       "mistralai/Mistral-7B-Instruct-v0.2",
       device_map={"": 1},  # Force AMD GPU (device 1)
       torch_dtype=torch.float16,
   )
   ```

4. Generate text
   ```python
   prompt = "Explain quantum computing to a 5 year old:"
   response = model.generate(...)
   ```

**Success Criteria**: Model loads and generates coherent text

### Phase 2: Split Model (4-6 hours)

**Goal**: LLaMA-2 13B split across both GPUs

**Steps**:
1. Load with automatic device map
   ```python
   model = AutoModelForCausalLM.from_pretrained(
       "meta-llama/Llama-2-13b-chat-hf",
       device_map="auto",  # Automatically split
       torch_dtype=torch.float16,
   )
   ```

2. Inspect device map
   ```python
   print(model.hf_device_map)
   ```

3. Benchmark performance
   - Single GPU baseline
   - Split across both
   - Measure transfer overhead

4. Verify correctness
   - Same outputs as single GPU
   - Quality maintained

**Success Criteria**: 13B model running, split verified

### Phase 3: Large Model (6-8 hours)

**Goal**: LLaMA-2 70B (4-bit) requiring both GPUs

**Steps**:
1. Install 4-bit quantization
   ```bash
   pip install bitsandbytes
   ```

2. Load 70B model
   ```python
   model = AutoModelForCausalLM.from_pretrained(
       "meta-llama/Llama-2-70b-chat-hf",
       load_in_4bit=True,
       device_map="auto",
       torch_dtype=torch.float16,
   )
   ```

3. Verify it requires both GPUs
   ```python
   # Should error if we try single GPU:
   model = AutoModelForCausalLM.from_pretrained(
       ...,
       device_map={"": 0},  # Force single GPU
   )
   # Expected: OOM error ✅
   ```

4. Test inference
   - Generate text
   - Measure latency
   - Compare to smaller models

5. Create demo
   - Interactive chat
   - Show device utilization
   - Demonstrate quality

**Success Criteria**: 70B model running on both GPUs, impossible on single

---

## 🎯 Success Metrics

### Must Have ✅

1. **Mistral 7B on Single GPU**
   - [ ] Model loads successfully
   - [ ] Generates coherent text
   - [ ] Runs on AMD GPU

2. **LLaMA-2 13B Split Across GPUs**
   - [ ] Automatic device map working
   - [ ] Both GPUs utilized
   - [ ] Correct outputs

3. **LLaMA-2 70B Requiring Both GPUs**
   - [ ] Loads successfully (4-bit)
   - [ ] Fails on single GPU (proof)
   - [ ] Works on both GPUs
   - [ ] Generates high-quality text

### Nice to Have

4. **Performance Benchmarking**
   - [ ] Tokens/second measurement
   - [ ] Latency per token
   - [ ] Compare to cloud (GPT-3.5)

5. **Interactive Demo**
   - [ ] Chat interface
   - [ ] Real-time generation
   - [ ] GPU utilization display

6. **Quality Comparison**
   - [ ] 7B vs 13B vs 70B quality
   - [ ] Same prompts
   - [ ] Demonstrate value of larger model

---

## 💡 Key Questions to Research

### 1. AMD GPU Support

**Question**: Does PyTorch/HuggingFace work well with AMD GPUs?

**Research Needed**:
- ROCm support in PyTorch
- Vulkan backend compatibility
- Any known issues

**Fallback**: If AMD problematic, use NVIDIA for primary, AMD for embeddings

### 2. Quantization Quality

**Question**: How much quality loss at 4-bit?

**Research**:
- GPTQ vs bitsandbytes
- Perplexity comparisons
- User experience

### 3. Transfer Overhead

**Question**: How much does PCIe transfer hurt?

**Expected**:
- ~5-10% overhead (based on layer activation size)
- Acceptable for 70B model access

### 4. Memory Headroom

**Question**: Exact memory usage per model?

**Need**:
- Measure actual VRAM with different batch sizes
- Determine safe limits
- Plan for worst case

---

## 🔧 Technical Challenges

### Challenge 1: AMD GPU PyTorch Support

**Problem**: PyTorch primarily targets CUDA (NVIDIA)

**Solutions**:
1. Use ROCm (AMD's CUDA equivalent)
2. Try Vulkan backend (experimental)
3. Fallback to CPU for AMD portion if needed

**Recommendation**: Research first, have fallback

### Challenge 2: Model Download Size

**Problem**: 70B model is ~35 GB to download

**Solutions**:
1. Download overnight
2. Use streaming (load shards incrementally)
3. Cache properly

### Challenge 3: Quantization Setup

**Problem**: 4-bit quantization has dependencies

**Solutions**:
1. Install `bitsandbytes` (may need GPU-specific build)
2. Alternative: GPTQ quantization
3. Fallback: Use 8-bit if 4-bit problematic

### Challenge 4: Generation Speed

**Problem**: Large models slow (especially split)

**Expectation**: 
- 70B model: ~2-5 tokens/sec (acceptable for local)
- 13B model: ~10-20 tokens/sec
- 7B model: ~30-50 tokens/sec

**Not a Problem**: Still useful for local inference!

---

## 📚 Resources Needed

### Software

**Python Environment**:
```bash
conda create -n toadstool-llm python=3.10
conda activate toadstool-llm

# Core dependencies
pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/rocm5.6
pip install transformers accelerate
pip install bitsandbytes
pip install huggingface_hub

# Optional
pip install gradio  # For web UI
pip install flask   # For API
```

**Rust Bindings** (Future):
```bash
# For eventual Rust integration
cargo add candle-core
cargo add tokenizers
```

### Hardware Requirements

**Disk Space**:
- Mistral 7B: ~15 GB
- LLaMA-2 13B: ~30 GB
- LLaMA-2 70B: ~40 GB (4-bit)
- Total: ~85 GB (keep all models)

**RAM**:
- Minimum: 32 GB
- Recommended: 64 GB
- Critical for model loading

**Internet**:
- Fast connection for model download
- HuggingFace hub access

---

## 🎯 Value Proposition

### Why This Matters

**Problem**: Large language models require expensive hardware
- GPT-3.5 quality: Typically needs A100 (80 GB, $10k+)
- Cloud inference: $0.002 per 1k tokens (adds up)
- Privacy: Data sent to cloud

**Our Solution**: Use consumer multi-GPU
- Hardware: RTX 3090 + RX 6950 XT (~$2k total)
- Cost per token: $0 (after hardware)
- Privacy: Everything local
- Capability: 70B model competitive with GPT-3.5

**Impact**: Democratize access to large language models

### Use Cases

**1. Privacy-Sensitive Applications**
- Medical records analysis
- Legal document review
- Personal assistant
- No data leaves premises

**2. Offline Operation**
- No internet required (after download)
- Air-gapped environments
- Reliability

**3. Cost Savings**
- High-volume usage
- Development/testing
- Research

**4. Customization**
- Fine-tune on private data
- Domain-specific models
- Full control

---

## 🚀 Next Steps

### Immediate (Now)

1. **Research ROCm/PyTorch on AMD**
   - Check compatibility
   - Test basic PyTorch operations
   - Verify multi-GPU support

2. **Install Dependencies**
   - Set up Python environment
   - Install PyTorch with ROCm
   - Test GPU detection

3. **Download Mistral 7B**
   - Use HuggingFace token
   - Cache model
   - Test loading

### Short-Term (Today)

4. **Test Single GPU (Mistral 7B)**
   - Load on AMD GPU
   - Generate text
   - Verify quality

5. **Test Split (LLaMA-2 13B)**
   - Automatic device map
   - Verify both GPUs used
   - Measure performance

### Medium-Term (This Week)

6. **Implement LLaMA-2 70B**
   - Download model
   - Load with 4-bit quantization
   - Verify requires both GPUs
   - Create interactive demo

7. **Documentation**
   - Setup guide
   - Performance benchmarks
   - Quality comparisons

---

## 💎 Expected Outcomes

### Technical

**Proof Points**:
- ✅ 70B model running locally
- ✅ Impossible on single consumer GPU
- ✅ Quality competitive with GPT-3.5
- ✅ Vendor-agnostic infrastructure

**Performance**:
- 70B model: 2-5 tokens/sec (acceptable)
- 13B model: 10-20 tokens/sec (good)
- 7B model: 30-50 tokens/sec (excellent)

### Value

**For Users**:
- Access to large models
- Privacy preserved
- Cost savings vs cloud
- Full control

**For ToadStool**:
- Demonstrates multi-GPU value
- Real-world use case
- Vendor freedom proven
- Production-ready example

---

**Document Version**: 1.0  
**Last Updated**: January 8, 2026  
**Status**: Research Complete - Ready to Implement  
**Next**: Install dependencies and test Mistral 7B

---

*ToadStool: Making 70B Models Accessible on Consumer Hardware* 🚀

**"40 GB Heterogeneous VRAM + Smart Software = GPT-3.5 Quality Locally"**

