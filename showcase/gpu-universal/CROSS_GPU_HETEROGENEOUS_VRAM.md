# Cross-GPU Heterogeneous VRAM Computing

**Date**: January 8, 2026  
**Hardware**: NVIDIA RTX 3090 (24 GB) + AMD RX 6950 XT (16 GB)  
**Total VRAM**: 40 GB Heterogeneous Memory  
**Status**: 🚀 READY TO IMPLEMENT

---

## 🎯 Vision

**Leverage combined VRAM across vendor boundaries for AI workloads**

**Traditional Limitation**:
```
Single GPU → Max 24 GB → Large models impossible
```

**ToadStool Solution**:
```
NVIDIA (24 GB) + AMD (16 GB) = 40 GB → Large models possible! ✅
```

---

## 🖥️ Hardware Configuration

### Available Resources

**GPU 1: NVIDIA RTX 3090**
```
Memory:          24 GB GDDR6X
Bandwidth:       936 GB/s
Compute:         35.58 TFLOPS (FP32)
Backend:         OpenCL ✅
Status:          Production verified (17.3x)
```

**GPU 2: AMD RX 6950 XT**
```
Memory:          16 GB GDDR6
Bandwidth:       576 GB/s
Compute:         23.65 TFLOPS (FP32)
Backend:         Vulkan ✅
Status:          Discovered & accessible
```

**Combined Capacity**:
```
Total VRAM:      40 GB (heterogeneous)
Total Compute:   59.23 TFLOPS (FP32)
Total Bandwidth: 1,512 GB/s (aggregate)
Multi-Vendor:    ✅ Yes!
```

---

## 💡 Use Cases

### 1. Large Model Inference

**Problem**: Model requires >24 GB VRAM  
**Solution**: Split model across GPUs

**Example**: Large Language Model (LLM)
```
Model Size:      30 GB
Traditional:     Impossible on single GPU ❌
ToadStool:       Split across NVIDIA (20 GB) + AMD (10 GB) ✅

Layers 1-20  → NVIDIA (24 GB)  ← First 20 layers
Layers 21-40 → AMD (16 GB)     ← Remaining layers
Pipeline:     Input → NVIDIA → Transfer → AMD → Output
```

### 2. Parallel Batch Processing

**Problem**: Large batch size for training/inference  
**Solution**: Split batch across GPUs

**Example**: Image Classification
```
Batch Size:      2048 images
Traditional:     OOM on single GPU ❌
ToadStool:       1200 on NVIDIA, 848 on AMD ✅

NVIDIA (24 GB):  1200 images (60%)
AMD (16 GB):     848 images (40%)
Aggregate:       2048 images total
Speedup:         Near 2x throughput
```

### 3. Pipeline Parallelism

**Problem**: Complex multi-stage pipeline  
**Solution**: Different stages on different GPUs

**Example**: Image Processing Pipeline
```
Stage 1 (Preprocessing)  → NVIDIA  ← Fast OpenCL
Stage 2 (Inference)      → AMD     ← Parallel execution
Stage 3 (Postprocessing) → NVIDIA  ← Final output

Pipelined:     ~2x throughput
Latency:       Overlapped execution
Utilization:   Both GPUs active
```

### 4. Multi-Model Ensemble

**Problem**: Run multiple models simultaneously  
**Solution**: Different models on different GPUs

**Example**: Ensemble Classification
```
Model A (ResNet-50)    → NVIDIA  ← 11 GB
Model B (VGG-19)       → AMD     ← 9 GB
Model C (EfficientNet) → NVIDIA  ← 8 GB
Model D (MobileNet)    → AMD     ← 4 GB

Aggregate:     4 models simultaneously
Ensemble:      Vote/average results
Accuracy:      Improved via diversity
```

---

## 🏗️ Architecture Patterns

### Pattern 1: Model Parallelism (Layer Splitting)

**When**: Model > 24 GB VRAM

```rust
struct SplitModel {
    part1: ModelPart,  // Runs on NVIDIA
    part2: ModelPart,  // Runs on AMD
    transfer: TransferLayer,
}

impl SplitModel {
    async fn forward(&self, input: Tensor) -> Result<Tensor> {
        // Part 1 on NVIDIA
        let intermediate = self.part1.forward_gpu(&nvidia_gpu, input).await?;
        
        // Transfer to AMD (PCIe or NVLink if available)
        let transferred = self.transfer.copy_to_gpu(&amd_gpu, intermediate).await?;
        
        // Part 2 on AMD
        let output = self.part2.forward_gpu(&amd_gpu, transferred).await?;
        
        Ok(output)
    }
}
```

**Characteristics**:
- ✅ Enables >24 GB models
- ⚠️ Transfer overhead (PCIe bandwidth)
- ✅ Full VRAM utilization
- Best for: Large models with sequential layers

### Pattern 2: Data Parallelism (Batch Splitting)

**When**: Large batch sizes

```rust
async fn parallel_inference(
    nvidia_gpu: &GpuInfo,
    amd_gpu: &GpuInfo,
    model: &Network,
    batch: &[Tensor],
) -> Result<Vec<Tensor>> {
    // Split batch proportionally to VRAM (60/40 split)
    let split_idx = (batch.len() * 60) / 100;
    let (nvidia_batch, amd_batch) = batch.split_at(split_idx);
    
    // Process in parallel
    let (nvidia_results, amd_results) = tokio::try_join!(
        async { model.forward_batch_gpu(nvidia_gpu, nvidia_batch).await },
        async { model.forward_batch_gpu(amd_gpu, amd_batch).await },
    )?;
    
    // Combine results
    Ok([nvidia_results, amd_results].concat())
}
```

**Characteristics**:
- ✅ Near-linear speedup (2x with 2 GPUs)
- ✅ Minimal transfer overhead
- ✅ Both GPUs fully utilized
- Best for: Training, large-batch inference

### Pattern 3: Pipeline Parallelism (Stage Assignment)

**When**: Multi-stage workloads

```rust
struct PipelineExecutor {
    stages: Vec<Stage>,
    gpu_assignment: HashMap<usize, GpuInfo>,
}

impl PipelineExecutor {
    async fn execute(&self, input: Tensor) -> Result<Tensor> {
        let mut current = input;
        
        for (stage_idx, stage) in self.stages.iter().enumerate() {
            let gpu = &self.gpu_assignment[&stage_idx];
            current = stage.execute_on_gpu(gpu, current).await?;
        }
        
        Ok(current)
    }
    
    async fn execute_pipelined(&self, inputs: Vec<Tensor>) -> Result<Vec<Tensor>> {
        // Overlapped execution for throughput
        let mut pipeline = Vec::new();
        
        for input in inputs {
            let stage_futures: Vec<_> = self.stages.iter()
                .enumerate()
                .map(|(idx, stage)| {
                    let gpu = &self.gpu_assignment[&idx];
                    stage.execute_on_gpu(gpu, input.clone())
                })
                .collect();
            
            pipeline.push(tokio::spawn(async move {
                // Execute stages sequentially but pipeline across inputs
                for fut in stage_futures {
                    fut.await?;
                }
                Ok(())
            }));
        }
        
        // Wait for pipeline to drain
        futures::future::try_join_all(pipeline).await?;
        Ok(vec![])  // Simplified
    }
}
```

**Characteristics**:
- ✅ High throughput (pipelined)
- ⚠️ More complex orchestration
- ✅ Good GPU utilization
- Best for: Multi-stage ML pipelines

### Pattern 4: Ensemble Parallelism (Multi-Model)

**When**: Running multiple models

```rust
struct EnsembleModel {
    models: Vec<(Network, GpuInfo)>,
}

impl EnsembleModel {
    async fn predict(&self, input: &Tensor) -> Result<Prediction> {
        // Run all models in parallel on their assigned GPUs
        let predictions: Vec<_> = self.models.iter()
            .map(|(model, gpu)| async move {
                model.forward_gpu(gpu, input).await
            })
            .collect();
        
        // Wait for all predictions
        let results = futures::future::try_join_all(predictions).await?;
        
        // Aggregate (vote, average, etc.)
        Ok(Self::aggregate_predictions(results))
    }
}
```

**Characteristics**:
- ✅ Perfect parallelism (independent models)
- ✅ Zero inter-GPU communication
- ✅ Best GPU utilization
- Best for: Ensemble methods, A/B testing

---

## 🎯 Implementation Plan

### Phase 1: Data Parallel Inference (Simplest)

**Goal**: Split MNIST batch across both GPUs

**Implementation** (2-3 hours):
```rust
// File: src/bin/cross_gpu_inference.rs

async fn parallel_batch_inference(
    nvidia: &GpuInfo,
    amd: &GpuInfo,
    network: &SimpleNetwork,
    images: &Array2<f32>,
    labels: &Array1<u8>,
) -> Result<BenchmarkStats> {
    let total_samples = images.nrows();
    
    // Split 60/40 based on VRAM (24GB vs 16GB)
    let nvidia_samples = (total_samples * 60) / 100;
    let amd_samples = total_samples - nvidia_samples;
    
    let (nvidia_images, amd_images) = images.split_at(Axis(0), nvidia_samples);
    let (nvidia_labels, amd_labels) = labels.split_at(Axis(0), nvidia_samples);
    
    let start = Instant::now();
    
    // Process in parallel
    let (nvidia_correct, amd_correct) = tokio::try_join!(
        async { run_inference_gpu(nvidia, network, &nvidia_images, &nvidia_labels).await },
        async { run_inference_gpu(amd, network, &amd_images, &amd_labels).await },
    )?;
    
    let elapsed = start.elapsed();
    let total_correct = nvidia_correct + amd_correct;
    
    Ok(BenchmarkStats {
        samples: total_samples,
        correct: total_correct,
        accuracy: total_correct as f32 / total_samples as f32,
        total_time_ms: elapsed.as_millis() as f64,
        throughput_per_sec: total_samples as f64 / elapsed.as_secs_f64(),
    })
}
```

**Expected Results**:
- Speedup: 1.8-2.0x vs single GPU
- Utilization: Both GPUs active
- Accuracy: Same as single GPU

### Phase 2: Large Model Simulation (Medium)

**Goal**: Simulate model >24 GB split across GPUs

**Implementation** (4-5 hours):
```rust
// Simulate a 30 GB model split across GPUs
struct LargeModelSimulator {
    part1_size_gb: f32,  // 20 GB on NVIDIA
    part2_size_gb: f32,  // 10 GB on AMD
    layers_per_part: Vec<Vec<Layer>>,
}

impl LargeModelSimulator {
    fn new() -> Self {
        // Create a model structure that simulates 30 GB
        Self {
            part1_size_gb: 20.0,
            part2_size_gb: 10.0,
            layers_per_part: vec![
                vec![/* 20 layers for NVIDIA */],
                vec![/* 20 layers for AMD */],
            ],
        }
    }
    
    async fn forward(&self, nvidia: &GpuInfo, amd: &GpuInfo, input: Tensor) -> Result<Tensor> {
        // Part 1 on NVIDIA
        let mut current = input;
        for layer in &self.layers_per_part[0] {
            current = layer.forward_gpu(nvidia, current).await?;
        }
        
        // Transfer to AMD (measure overhead)
        let transfer_start = Instant::now();
        let transferred = transfer_tensor_to_gpu(amd, current).await?;
        let transfer_time = transfer_start.elapsed();
        println!("Transfer time: {:?}", transfer_time);
        
        // Part 2 on AMD
        for layer in &self.layers_per_part[1] {
            transferred = layer.forward_gpu(amd, transferred).await?;
        }
        
        Ok(transferred)
    }
}
```

**Expected Results**:
- Model size: 30 GB (impossible on single GPU)
- Inference: Working across both GPUs
- Bottleneck: PCIe transfer overhead measured
- Proof: >24 GB models are viable

### Phase 3: Pipeline Parallelism (Advanced)

**Goal**: Overlapped execution for throughput

**Implementation** (6-8 hours):
```rust
struct PipelinedInference {
    preprocessor: Preprocessor,  // On NVIDIA
    model: Network,               // On AMD
    postprocessor: Postprocessor, // On NVIDIA
}

impl PipelinedInference {
    async fn process_stream(&self, inputs: Vec<Tensor>) -> Result<Vec<Tensor>> {
        let (tx_preprocess, rx_preprocess) = mpsc::channel(16);
        let (tx_inference, rx_inference) = mpsc::channel(16);
        
        // Stage 1: Preprocessing on NVIDIA
        let preprocess_task = tokio::spawn(async move {
            for input in inputs {
                let processed = self.preprocessor.process_gpu(&nvidia, input).await?;
                tx_preprocess.send(processed).await?;
            }
            Ok(())
        });
        
        // Stage 2: Inference on AMD (pipelined)
        let inference_task = tokio::spawn(async move {
            while let Some(preprocessed) = rx_preprocess.recv().await {
                let output = self.model.forward_gpu(&amd, preprocessed).await?;
                tx_inference.send(output).await?;
            }
            Ok(())
        });
        
        // Stage 3: Postprocessing on NVIDIA (pipelined)
        let postprocess_task = tokio::spawn(async move {
            let mut results = Vec::new();
            while let Some(inference_output) = rx_inference.recv().await {
                let final_output = self.postprocessor.process_gpu(&nvidia, inference_output).await?;
                results.push(final_output);
            }
            Ok(results)
        });
        
        // Wait for pipeline to complete
        tokio::try_join!(preprocess_task, inference_task, postprocess_task)?;
        
        Ok(vec![])  // Results from postprocess_task
    }
}
```

**Expected Results**:
- Throughput: Near 2x (overlapped execution)
- Latency: Slightly higher (pipeline depth)
- Utilization: Both GPUs active simultaneously
- Complexity: Higher (channel management)

---

## 📊 Expected Performance

### Data Parallelism (Batch Splitting)

**Single GPU Baseline** (NVIDIA):
```
Throughput:    84,552 img/s
Latency:       0.012 ms/img
GPU Usage:     ~80%
```

**Cross-GPU (NVIDIA + AMD)**:
```
Throughput:    ~150,000 img/s (1.8x)
Latency:       0.007 ms/img
GPU Usage:     NVIDIA ~80%, AMD ~70%
Speedup:       1.8x (not quite 2x due to AMD optimization pending)
```

**Why not 2x?**:
- AMD backend still being optimized
- NVIDIA OpenCL more mature (17.3x vs CPU)
- AMD Vulkan executor accuracy being debugged
- Expected to reach 1.9-2.0x after AMD optimization

### Model Parallelism (Large Models)

**Single GPU Limitation**:
```
Max Model Size:  24 GB (NVIDIA only)
Large Models:    Impossible ❌
```

**Cross-GPU (NVIDIA + AMD)**:
```
Max Model Size:  40 GB (combined) ✅
Overhead:        PCIe transfer (5-10%)
Viability:       YES for models 24-40 GB
Example:         LLaMA-2 70B (quantized to 35 GB)
```

### Pipeline Parallelism

**Sequential** (single GPU):
```
Stage 1:  10 ms  → NVIDIA
Stage 2:  20 ms  → NVIDIA  
Stage 3:  5 ms   → NVIDIA
Total:    35 ms/sample
```

**Pipelined** (multi-GPU):
```
Stage 1:  10 ms  → NVIDIA  ┐
Stage 2:  20 ms  → AMD     ├─ Overlapped
Stage 3:  5 ms   → NVIDIA  ┘
Total:    ~20 ms/sample (1.75x throughput for stream)
```

---

## 🔧 Technical Challenges

### Challenge 1: Inter-GPU Transfer

**Problem**: PCIe bandwidth limits

**Solution**:
```rust
// Minimize transfers
// - Use async/pipelined transfers
// - Batch transfers together
// - Consider NVLink if available (not on consumer GPUs)

async fn optimized_transfer(
    src_gpu: &GpuInfo,
    dst_gpu: &GpuInfo,
    data: &[f32],
) -> Result<Vec<f32>> {
    // Use pinned memory for faster DMA
    let pinned = allocate_pinned_memory(data.len())?;
    copy_to_pinned(&pinned, data)?;
    
    // Async transfer (non-blocking)
    let transferred = async_copy_to_gpu(dst_gpu, &pinned).await?;
    
    Ok(transferred)
}
```

### Challenge 2: Load Balancing

**Problem**: GPUs have different capabilities

**Solution**:
```rust
// Dynamic load balancing based on actual performance
fn compute_split_ratio(nvidia_perf: f32, amd_perf: f32) -> (f32, f32) {
    let total = nvidia_perf + amd_perf;
    let nvidia_ratio = nvidia_perf / total;
    let amd_ratio = amd_perf / total;
    (nvidia_ratio, amd_ratio)
}

// Example: If NVIDIA is 2x faster than AMD
// Split: 67% NVIDIA, 33% AMD (not 60/40 by VRAM)
```

### Challenge 3: Synchronization

**Problem**: Coordinating execution across GPUs

**Solution**:
```rust
// Use async/await for natural synchronization
async fn synchronized_execution() -> Result<()> {
    // Both start simultaneously
    let (result1, result2) = tokio::try_join!(
        gpu1_task(),
        gpu2_task(),
    )?;
    
    // Automatically synchronized at this point
    combine_results(result1, result2)
}
```

### Challenge 4: Memory Management

**Problem**: Tracking memory across GPUs

**Solution**:
```rust
struct MultiGpuMemoryManager {
    allocations: HashMap<GpuInfo, Vec<Allocation>>,
}

impl MultiGpuMemoryManager {
    fn allocate(&mut self, gpu: &GpuInfo, size: usize) -> Result<Allocation> {
        let available = self.get_available_memory(gpu)?;
        if available < size {
            return Err(anyhow!("OOM on GPU: {}", gpu.name));
        }
        
        // Track allocation
        let alloc = gpu.allocate(size)?;
        self.allocations.get_mut(gpu).unwrap().push(alloc.clone());
        Ok(alloc)
    }
    
    fn total_available(&self) -> usize {
        self.allocations.iter()
            .map(|(gpu, allocs)| self.get_available_memory(gpu).unwrap_or(0))
            .sum()
    }
}
```

---

## 🎯 Success Criteria

### Must Have ✅

1. **Data Parallel Inference**
   - [ ] Split batch across NVIDIA and AMD
   - [ ] Both GPUs processing simultaneously
   - [ ] Speedup >1.5x vs single GPU
   - [ ] Same accuracy as single GPU

2. **Memory Utilization**
   - [ ] Track memory usage on both GPUs
   - [ ] Demonstrate >24 GB capacity
   - [ ] No OOM errors

3. **Performance Measurement**
   - [ ] Measure per-GPU throughput
   - [ ] Measure aggregate throughput
   - [ ] Measure transfer overhead
   - [ ] Document bottlenecks

### Nice to Have

4. **Large Model Simulation**
   - [ ] Simulate 30 GB model
   - [ ] Split across GPUs
   - [ ] Measure inference time
   - [ ] Quantify PCIe overhead

5. **Pipeline Parallelism**
   - [ ] Multi-stage pipeline
   - [ ] Overlapped execution
   - [ ] Stream processing

6. **Load Balancing**
   - [ ] Dynamic split ratio
   - [ ] Performance-based allocation
   - [ ] Adaptive scheduling

---

## 📝 Implementation Roadmap

### Week 1: Data Parallelism (Core)

**Day 1-2**: Basic Implementation
- [ ] Create `cross_gpu_inference.rs`
- [ ] Implement batch splitting logic
- [ ] Test on MNIST (5000 images)
- [ ] Measure baseline performance

**Day 3-4**: Optimization
- [ ] Tune split ratio (60/40 vs dynamic)
- [ ] Minimize synchronization overhead
- [ ] Optimize memory transfers
- [ ] Benchmark improvements

**Day 5**: Documentation
- [ ] Document results
- [ ] Create performance graphs
- [ ] Write usage guide

**Deliverable**: Working data-parallel inference with 1.5-2.0x speedup

### Week 2: Large Model Support (Advanced)

**Day 1-3**: Model Splitting
- [ ] Design layer assignment algorithm
- [ ] Implement transfer layer
- [ ] Create 30 GB model simulator
- [ ] Test end-to-end

**Day 4-5**: Performance Tuning
- [ ] Minimize transfer overhead
- [ ] Async/pipelined transfers
- [ ] Memory pooling
- [ ] Benchmark

**Deliverable**: Proof-of-concept for >24 GB models

### Week 3: Pipeline Parallelism (Expert)

**Day 1-3**: Pipeline Architecture
- [ ] Design stage assignment
- [ ] Implement channel-based pipeline
- [ ] Test with multi-stage workload

**Day 4-5**: Optimization & Polish
- [ ] Tune pipeline depth
- [ ] Minimize latency
- [ ] Maximize throughput

**Deliverable**: Production-ready pipeline system

---

## 💡 Real-World Applications

### 1. Large Language Models (LLMs)

**Scenario**: Run LLaMA-2 70B (quantized to 35 GB)

**Approach**:
```
Layers 1-40:  NVIDIA (20 GB)
Layers 41-80: AMD (15 GB)
Transfer:     Once per forward pass
Inference:    Viable for 35 GB model!
```

**Value**: Enables models impossible on single consumer GPU

### 2. High-Throughput Inference

**Scenario**: Real-time video processing (30 FPS, 4K)

**Approach**:
```
Frames 1-18:  NVIDIA (60%)
Frames 19-30: AMD (40%)
Processing:   Parallel
Output:       Combined stream
```

**Value**: 2x throughput = higher resolution or frame rate

### 3. Ensemble Learning

**Scenario**: Medical image classification (high accuracy needed)

**Approach**:
```
Model A (ResNet):      NVIDIA
Model B (DenseNet):    AMD
Model C (EfficientNet): NVIDIA
Ensemble:              Vote/average
```

**Value**: Higher accuracy via model diversity, no single-GPU bottleneck

### 4. Research & Development

**Scenario**: Experiment with large architectures

**Approach**:
```
Baseline Model:     NVIDIA (fast iteration)
Large Variant:      NVIDIA + AMD (40 GB)
Comparison:         A/B testing
```

**Value**: Explore larger models without cloud costs

---

## 🚀 Quick Start

### Run Data-Parallel Inference (When Implemented)

```bash
# Build
cd showcase/gpu-universal/ml-inference
cargo build --release --features "opencl vulkan" --bin cross-gpu-inference

# Run
cargo run --release --features "opencl vulkan" --bin cross-gpu-inference

# Expected output:
# ✓ Found 2 GPUs (NVIDIA + AMD)
# ✓ Split: 60% NVIDIA (3000 images), 40% AMD (2000 images)
# ✓ Processing in parallel...
# ✓ Throughput: 150,000 img/s (1.8x speedup)
# ✓ Accuracy: 90.5% (same as single GPU)
```

### Simulate Large Model (When Implemented)

```bash
# Run large model simulation
cargo run --release --features "opencl vulkan" --bin large-model-sim

# Expected output:
# ✓ Model size: 30 GB (NVIDIA: 20 GB, AMD: 10 GB)
# ✓ Impossible on single GPU ❌
# ✓ Viable on cross-GPU setup ✅
# ✓ Inference time: 45 ms (incl. 5 ms transfer)
# ✓ Transfer overhead: 11%
```

---

## 💎 Bottom Line

**Opportunity**: 40 GB heterogeneous VRAM (NVIDIA + AMD)

**Capabilities Unlocked**:
1. ✅ Large models (24-40 GB) - impossible on single GPU
2. ✅ High throughput (1.8-2.0x) - parallel batch processing
3. ✅ Pipeline workloads - overlapped execution
4. ✅ Ensemble methods - independent model parallelism

**Value Proposition**:
- **Cost**: Use existing hardware (no new GPU needed)
- **Capability**: Enable workloads impossible on single GPU
- **Performance**: Near-linear speedup for parallelizable workloads
- **Flexibility**: Choose pattern based on workload

**Status**: Infrastructure ready, implementation needed ✅

**Next Step**: Implement data-parallel inference (simplest, highest value)

---

**Document Version**: 1.0  
**Last Updated**: January 8, 2026  
**Status**: Ready for Implementation  
**Estimated Effort**: 2-3 hours for data parallelism, 1-2 weeks for full suite

---

*ToadStool: Breaking Vendor Lock-in AND Single-GPU Limitations* 🚀

**"40 GB Heterogeneous VRAM - Because Boundaries Are Artificial"**

