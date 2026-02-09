# NestGate Integration API Design

**Version**: 1.0  
**Date**: December 19, 2025  
**Target**: Week 2 Integration

---

## 🎯 Overview

NestGate provides large-scale data storage (76TB cold storage on LAN) for ToadStool Deep Learning. This document defines the integration API for Week 2.

**Goals**:
- Store large datasets (ImageNet, COCO, custom datasets)
- Stream data efficiently for training
- Distributed access across towers
- Versioning and metadata management

---

## 📊 Current State (Week 1)

### Storage

| Dataset | Size | Location | Access |
|---------|------|----------|--------|
| CIFAR-10 | 170 MB | Local disk | Direct |
| Total | < 1 GB | Per-tower | Replicated |

**Limitations**:
- Small datasets only
- Manual download
- Tower-local storage
- No versioning

---

## 🚀 Target State (Week 2+)

### Storage

| Dataset | Size | Location | Access |
|---------|------|----------|--------|
| CIFAR-10 | 170 MB | NestGate | Streaming |
| ImageNet | 150 GB | NestGate | Streaming |
| COCO | 25 GB | NestGate | Streaming |
| Custom | 500 GB+ | NestGate | Streaming |
| **Total** | **76 TB** | **Cold storage** | **Distributed** |

**Benefits**:
- Large dataset support
- Automatic distribution
- Single source of truth
- Versioning + metadata

---

## 🏗️ Architecture

```
ToadStool Training
    ↓
┌───────────────────────┐
│   NestGate Client     │
│  (Rust library)       │
│  ┌─────────────────┐  │
│  │ • List datasets │  │
│  │ • Stream chunks │  │
│  │ • Cache locally │  │
│  └─────────────────┘  │
└──────────┬────────────┘
           │ gRPC/HTTP
           ↓
┌───────────────────────┐
│   NestGate Server     │
│   (76TB Storage)      │
│  ┌─────────────────┐  │
│  │ • Dataset store │  │
│  │ • Chunk serving │  │
│  │ • Metadata DB   │  │
│  └─────────────────┘  │
└───────────────────────┘
```

---

## 📡 API Design

### 1. Dataset Discovery

**List Available Datasets**:

```rust
pub struct NestGateClient {
    endpoint: String,
    http_client: reqwest::Client,
}

impl NestGateClient {
    /// List all datasets available in NestGate
    pub async fn list_datasets(&self) -> Result<Vec<DatasetInfo>> {
        let response = self.http_client
            .get(&format!("{}/api/v1/datasets", self.endpoint))
            .send()
            .await?;
        
        let datasets: Vec<DatasetInfo> = response.json().await?;
        Ok(datasets)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetInfo {
    pub name: String,
    pub version: String,
    pub size_bytes: u64,
    pub num_samples: usize,
    pub format: DatasetFormat,
    pub splits: Vec<String>, // ["train", "val", "test"]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DatasetFormat {
    ImageFolder,  // ImageNet-style
    COCO,         // COCO JSON + images
    TFRecord,     // TensorFlow format
    WebDataset,   // WebDataset shards
    Custom(String), // Custom format
}
```

**Example Response**:
```json
[
  {
    "name": "imagenet-1k",
    "version": "2012",
    "size_bytes": 150000000000,
    "num_samples": 1281167,
    "format": "ImageFolder",
    "splits": ["train", "val"],
    "metadata": {
      "classes": "1000",
      "resolution": "variable"
    }
  },
  {
    "name": "coco-2017",
    "version": "2017",
    "size_bytes": 25000000000,
    "num_samples": 118287,
    "format": "COCO",
    "splits": ["train", "val"],
    "metadata": {
      "tasks": "detection,segmentation,keypoints"
    }
  }
]
```

---

### 2. Dataset Streaming

**Stream Dataset Chunks**:

```rust
impl NestGateClient {
    /// Stream a dataset split (e.g., "train", "val", "test")
    pub async fn stream_dataset(
        &self,
        dataset: &str,
        split: &str,
        chunk_size: usize,
    ) -> Result<DatasetStream> {
        let url = format!("{}/api/v1/datasets/{}/splits/{}/stream", 
                         self.endpoint, dataset, split);
        
        let response = self.http_client
            .get(&url)
            .query(&[("chunk_size", chunk_size)])
            .send()
            .await?;
        
        Ok(DatasetStream::new(response))
    }
}

pub struct DatasetStream {
    response: Response,
    buffer: Vec<u8>,
    chunk_size: usize,
}

impl DatasetStream {
    /// Get next chunk of data
    pub async fn next_chunk(&mut self) -> Result<Option<DataChunk>> {
        // Read chunk from HTTP stream
        let data = self.read_chunk().await?;
        
        if data.is_empty() {
            return Ok(None);
        }
        
        // Deserialize chunk
        let chunk: DataChunk = bincode::deserialize(&data)?;
        Ok(Some(chunk))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataChunk {
    pub samples: Vec<Sample>,
    pub chunk_id: usize,
    pub total_chunks: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sample {
    pub data: Vec<u8>,  // Image bytes or tensor data
    pub label: i64,
    pub metadata: HashMap<String, String>,
}
```

**Example Usage**:
```rust
let client = NestGateClient::new("http://nestgate:8080");

// Stream ImageNet training data
let mut stream = client.stream_dataset("imagenet-1k", "train", 1000).await?;

while let Some(chunk) = stream.next_chunk().await? {
    println!("Chunk {}/{}: {} samples", 
             chunk.chunk_id, chunk.total_chunks, chunk.samples.len());
    
    // Process samples
    for sample in chunk.samples {
        let image = decode_image(&sample.data)?;
        let label = sample.label;
        // Train on (image, label)
    }
}
```

---

### 3. Local Caching

**Cache Frequently Accessed Data**:

```rust
impl NestGateClient {
    /// Download and cache a dataset locally
    pub async fn cache_dataset(
        &self,
        dataset: &str,
        split: &str,
        cache_dir: &Path,
    ) -> Result<CachedDataset> {
        let cache_path = cache_dir.join(format!("{}_{}", dataset, split));
        
        // Check if already cached
        if cache_path.exists() {
            return Ok(CachedDataset::load(&cache_path)?);
        }
        
        // Download and cache
        let mut stream = self.stream_dataset(dataset, split, 10000).await?;
        let mut samples = Vec::new();
        
        while let Some(chunk) = stream.next_chunk().await? {
            samples.extend(chunk.samples);
        }
        
        // Save to cache
        let cached = CachedDataset { samples };
        cached.save(&cache_path)?;
        
        Ok(cached)
    }
}

pub struct CachedDataset {
    pub samples: Vec<Sample>,
}

impl CachedDataset {
    /// Load cached dataset from disk
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let samples = bincode::deserialize_from(reader)?;
        Ok(CachedDataset { samples })
    }
    
    /// Save dataset to disk cache
    pub fn save(&self, path: &Path) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, &self.samples)?;
        Ok(())
    }
}
```

---

### 4. Dataset Metadata

**Query Dataset Information**:

```rust
impl NestGateClient {
    /// Get detailed metadata for a dataset
    pub async fn get_dataset_metadata(
        &self,
        dataset: &str,
    ) -> Result<DatasetMetadata> {
        let response = self.http_client
            .get(&format!("{}/api/v1/datasets/{}/metadata", 
                         self.endpoint, dataset))
            .send()
            .await?;
        
        let metadata: DatasetMetadata = response.json().await?;
        Ok(metadata)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub num_samples: HashMap<String, usize>, // split -> count
    pub format: DatasetFormat,
    pub schema: DatasetSchema,
    pub statistics: DatasetStatistics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetSchema {
    pub input_shape: Vec<i64>,
    pub label_type: String,
    pub num_classes: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetStatistics {
    pub mean: Vec<f64>,
    pub std: Vec<f64>,
    pub class_distribution: HashMap<i64, usize>,
}
```

---

## 🔄 Integration Workflow

### Week 2: Basic Integration

1. **Setup NestGate Client**
   ```rust
   let nestgate = NestGateClient::new("http://nestgate.local:8080");
   ```

2. **List Datasets**
   ```rust
   let datasets = nestgate.list_datasets().await?;
   for ds in datasets {
       println!("{}: {} samples", ds.name, ds.num_samples);
   }
   ```

3. **Stream Training Data**
   ```rust
   let mut stream = nestgate.stream_dataset("imagenet-1k", "train", 1000).await?;
   while let Some(chunk) = stream.next_chunk().await? {
       // Train on chunk
   }
   ```

### Week 3: Advanced Integration

1. **Local Caching**
   - Cache frequently-used datasets on tower SSD
   - Background refresh
   - LRU eviction

2. **Distributed Sharding**
   - Each tower caches its shard
   - Coordinated by ToadStool
   - Managed via Songbird

3. **Version Management**
   - Track dataset versions
   - Reproducible training
   - A/B testing

---

## 📊 Performance Targets

### Streaming

| Metric | Target | Notes |
|--------|--------|-------|
| **Throughput** | 500 MB/s | Per tower |
| **Latency** | < 100ms | First chunk |
| **Cache hit** | > 90% | After warmup |

### Storage

| Metric | Target | Notes |
|--------|--------|-------|
| **Capacity** | 76 TB | Cold storage |
| **Datasets** | 100+ | Various formats |
| **Versions** | 10 per dataset | Historical tracking |

---

## 🔐 Security

### Authentication

```rust
impl NestGateClient {
    pub fn with_auth(endpoint: String, token: String) -> Self {
        let http_client = reqwest::Client::builder()
            .default_headers({
                let mut headers = HeaderMap::new();
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {}", token)).unwrap()
                );
                headers
            })
            .build()
            .unwrap();
        
        NestGateClient { endpoint, http_client }
    }
}
```

### Encryption

- **In-transit**: TLS 1.3
- **At-rest**: BearDog encryption
- **Keys**: Genetic key hierarchies

---

## 📈 Scaling Plan

### Phase 1 (Week 2): Basic Streaming

- Single dataset (ImageNet)
- Simple HTTP streaming
- Local caching
- **Target**: 150 GB dataset working

### Phase 2 (Week 3): Multi-Dataset

- Multiple datasets (ImageNet, COCO, etc.)
- Concurrent streaming
- Distributed caching
- **Target**: 500 GB+ working

### Phase 3 (Week 4): Production

- All datasets (76 TB)
- Optimized protocols (gRPC)
- Advanced caching strategies
- **Target**: Full capacity utilized

---

## 🧪 Testing

### Unit Tests

- Client API methods
- Chunk deserialization
- Cache operations

### Integration Tests

- End-to-end streaming
- Multi-tower coordination
- Cache hit rates

### Load Tests

- 10 concurrent towers
- 76 TB dataset
- Sustained throughput

---

## 📝 Implementation Checklist

### Week 2

- [ ] NestGate client library (`crates/nestgate-client/`)
- [ ] HTTP streaming implementation
- [ ] Local cache system
- [ ] ImageNet integration
- [ ] Performance benchmarks
- [ ] Documentation

### Week 3

- [ ] gRPC protocol (faster)
- [ ] Multi-dataset support
- [ ] Distributed caching
- [ ] Version management
- [ ] Monitoring dashboard

### Week 4

- [ ] Production hardening
- [ ] Fault tolerance
- [ ] Auto-scaling
- [ ] Full 76TB utilization

---

## 🚀 Success Criteria

Week 2 NestGate integration is **complete** when:

1. ✅ Client library implemented
2. ✅ ImageNet streaming working
3. ✅ Local caching functional
4. ✅ Performance targets met (500 MB/s)
5. ✅ Documentation complete
6. ✅ Integration tests passing

---

**Status**: API Designed  
**Next**: Week 2 Implementation  
**Ready For**: NestGate team handoff

🧠🦀💾 **Large-Scale Data Storage** 💾🦀🧠

