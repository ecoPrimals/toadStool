# 💾 NestGate Storage Basics

**Level 0 - Standalone Capabilities**

Learn what NestGate provides as a distributed storage service.

---

## 🎯 What You'll Learn

1. **Simple Storage** - Store, retrieve, list, delete operations
2. **Large Files** - Efficient handling of ML models and datasets  
3. **Rich Metadata** - Organize and discover data with tags and attributes

---

## 🚀 Quick Start

### Run All Demos

```bash
./demo-simple-storage.sh
./demo-large-files.sh
./demo-metadata.sh
```

### Run Individual Demos

```bash
# Start with the basics
./demo-simple-storage.sh

# Then try large files
./demo-large-files.sh

# Finally, explore metadata
./demo-metadata.sh
```

---

## 📋 Demos

### 1. Simple Storage (`demo-simple-storage.sh`)

**Time**: 2 minutes  
**Purpose**: Learn basic CRUD operations

**What it shows**:
- ✅ Store files in NestGate
- ✅ Retrieve files
- ✅ List stored files
- ✅ Verify integrity with checksums
- ✅ Delete files

**Key takeaway**: NestGate provides simple, reliable storage with integrity guarantees.

---

### 2. Large File Handling (`demo-large-files.sh`)

**Time**: 3 minutes  
**Purpose**: Understand performance for large data

**What it shows**:
- ✅ Chunked uploads for reliability
- ✅ Throughput measurement
- ✅ Range queries (partial retrieval)
- ✅ Zero-copy operations
- ✅ Compression and deduplication

**Key takeaway**: NestGate efficiently handles ML models (100MB+) with high throughput and smart optimizations.

---

### 3. Rich Metadata (`demo-metadata.sh`)

**Time**: 4 minutes  
**Purpose**: Learn data organization and discovery

**What it shows**:
- ✅ Store files with custom metadata
- ✅ Query by tags
- ✅ Query by attributes (accuracy, version, etc.)
- ✅ Update metadata without re-uploading
- ✅ Fast metadata-only queries
- ✅ Version management

**Key takeaway**: Rich metadata turns storage into a queryable model/dataset registry.

---

## 💡 Key Concepts

### 1. **Storage Operations**

```bash
# Store
curl -X POST $NESTGATE/api/v1/storage/store \
  -H "X-Storage-Key: my-model" \
  --data-binary @model.bin

# Retrieve
curl $NESTGATE/api/v1/storage/retrieve/$STORAGE_ID \
  -o model.bin

# List
curl $NESTGATE/api/v1/storage/list?prefix=my-models/

# Delete
curl -X DELETE $NESTGATE/api/v1/storage/delete/$STORAGE_ID
```

### 2. **Metadata-Driven Discovery**

```json
{
  "model_name": "mnist_classifier",
  "version": "1.0.0",
  "accuracy": 0.95,
  "tags": ["production", "mnist"],
  "framework": "rust-native"
}
```

Query by any field:
```bash
# By tag
curl $NESTGATE/api/v1/storage/query?tag=production

# By attribute
curl $NESTGATE/api/v1/storage/query?filter=accuracy>0.9

# By name
curl $NESTGATE/api/v1/storage/query?filter=model_name=mnist_classifier
```

### 3. **Performance Optimizations**

- **Compression**: ~30% storage savings (LZ4)
- **Deduplication**: Share common data blocks (ZFS)
- **Zero-copy**: Direct disk-to-app, no memory copies
- **Range queries**: Fetch only what you need

---

## 🎓 Architecture

### Data Flow

```
Application
    ↓ (store)
NestGate API
    ↓
Storage Layer
    ├─ Data (ZFS)
    ├─ Metadata (indexed)
    └─ Checksums (integrity)
    ↓ (retrieve)
Application
```

### Storage Features

```
┌─────────────────────────────┐
│  NestGate Storage           │
│                             │
│  • ZFS backend              │
│  • Compression (LZ4)        │
│  • Deduplication            │
│  • Snapshots                │
│  • Checksumming             │
│  • Metadata indexing        │
│  • HTTP API                 │
└─────────────────────────────┘
```

---

## 📊 Demo Mode vs Live Mode

### Demo Mode (No NestGate running)

- ✅ All demos work
- ✅ Operations simulated
- ✅ Learn concepts
- ❌ No actual persistence

### Live Mode (NestGate running)

- ✅ Real storage operations
- ✅ Actual persistence
- ✅ Performance metrics
- ✅ Integration testing

**Tip**: Start with demo mode to learn, then try live mode to validate.

---

## 🔍 Real-World Use Cases

### 1. ML Model Registry

```
Store trained models with metadata:
• Model name and version
• Training accuracy and loss
• Framework and dependencies
• Production/staging tags

Query production models:
• Find all models > 95% accuracy
• List latest version by name
• Filter by framework
```

### 2. Dataset Management

```
Store training datasets with metadata:
• Dataset name and version
• Row count and schema
• Collection date
• Privacy tags (PII, sensitive)

Query datasets:
• Find datasets for specific task
• List by collection date
• Filter by size
```

### 3. Checkpoint Storage

```
Store training checkpoints:
• Model state at epoch N
• Optimizer state
• Training metrics
• Timestamp

Resume training:
• Find latest checkpoint
• Load model state
• Continue from epoch N+1
```

---

## 🆘 Troubleshooting

### "NestGate not responding"

✅ **Expected**: Demos work in demo mode
```bash
# Check if NestGate is running
curl http://localhost:8082/health

# Or set custom endpoint
export NESTGATE_ENDPOINT=http://your-server:8082
```

### "Curl not found"

✅ **Solution**: Install curl
```bash
sudo apt install curl  # Debian/Ubuntu
sudo dnf install curl  # Fedora
```

### "Permission denied"

✅ **Solution**: Check script permissions
```bash
chmod +x *.sh
```

---

## 🔗 Next Steps

### Continue Learning

1. **Performance** → `../02-performance/`
   Learn about throughput, concurrency, zero-copy

2. **Data Services** → `../03-data-services/`
   Explore ZFS features: dedup, compression, snapshots

3. **Integration** → `../../nestgate-integration/`
   See how ToadStool uses NestGate for compute results

### Extend These Demos

1. **Custom Metadata** - Add your own metadata fields
2. **Query Builder** - Create complex queries
3. **Batch Operations** - Store multiple files
4. **Monitoring** - Add metrics collection

---

## 📚 Additional Resources

- **NestGate Docs**: `/home/eastgate/Development/ecoPrimals/nestgate/docs/`
- **API Reference**: `/home/eastgate/Development/ecoPrimals/nestgate/API.md`
- **Full Showcase**: `/home/eastgate/Development/ecoPrimals/nestgate/showcase/`

---

**Status**: ✅ **Ready to Run**  
**Time**: 10 minutes for all 3 demos  
**Difficulty**: ⭐ Beginner friendly

💾 **Discover NestGate's storage capabilities!** 🚀

