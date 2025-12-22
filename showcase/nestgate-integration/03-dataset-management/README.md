# Dataset Management & Versioning with NestGate

**Level**: 1 (One-Way Integration)  
**Time**: 10 minutes  
**Goal**: Demonstrate versioned dataset storage and management

---

## 🎯 What This Demonstrates

**Dataset Versioning Pipeline**:
- Store training datasets in NestGate
- Version datasets (v1, v2, v3)
- Rich metadata (samples, features, quality)
- ToadStool loads any version for training
- Compare training results across versions

---

## 🚀 Quick Start

```bash
./demo-dataset-versioning.sh
```

---

## 📊 What You'll See

### Three Dataset Versions

**v1 - Baseline**:
- 60,000 training samples
- Initial collection
- 85.2% accuracy

**v2 - Augmented**:
- 70,000 training samples (+10K)
- Data augmentation applied
- 88.7% accuracy (+3.5%)

**v3 - Production**:
- 63,000 training + 7,000 validation
- Cleaned labels, normalized features
- 91.3% accuracy (+6.1%) 🏆

### Version Comparison

Training results showing clear improvement from v1 → v3, demonstrating the value of dataset versioning.

---

## 💡 Key Concepts

### Dataset Versioning
- **What**: Storing multiple versions of a dataset
- **Why**: Track improvements, reproduce experiments
- **How**: NestGate provides built-in versioning

### Dataset Metadata
- **What**: Rich information about the dataset
- **Why**: Understand dataset characteristics
- **How**: Samples, features, splits, preprocessing

### Version Comparison
- **What**: Training models on different versions
- **Why**: Measure dataset improvement impact
- **How**: Compare accuracy, training time

### Reproducibility
- **What**: Exact same dataset for every experiment
- **Why**: Scientific rigor, audit trail
- **How**: Immutable versioned storage

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────┐
│         DATASET VERSIONING WORKFLOW                 │
└─────────────────────────────────────────────────────┘

     Create Dataset v1
    (Initial collection)
            │
            │ 1. Store in NestGate
            ↓
        🗄️ NestGate
     (Versioned Storage)
            │
    ┌───────┼───────┐
    │       │       │
   v1      v2      v3
Baseline Augment Production
    │       │       │
    └───────┴───────┘
            │
2. ToadStool loads any version
            ↓
      🍄 ToadStool
     (ML Training)
            │
   3. Train and compare
            ↓
   Select best dataset
    (v3: 91.3% acc)
```

---

## 🎓 Real-World Use Cases

### Research & Experimentation
- Track dataset improvements
- Reproduce published results
- Share exact datasets with community

### Production ML
- Version production datasets
- A/B test dataset improvements
- Rollback to previous version if needed

### Team Collaboration
- Everyone uses same dataset version
- No "works on my machine" data issues
- Clear dataset lineage

### Compliance & Audit
- Complete audit trail of data used
- Know exactly what trained each model
- Regulatory compliance (data provenance)

---

## 📋 Demo Flow

1. **Prerequisites**: Check NestGate availability
2. **Create v1**: Initial baseline dataset (60K samples)
3. **Create v2**: Augmented dataset (70K samples)
4. **Create v3**: Production dataset (cleaned, normalized)
5. **List Versions**: Query all versions from NestGate
6. **Train Models**: Train on each version
7. **Compare Results**: Show improvement (v1: 85% → v3: 91%)
8. **Visualization**: Complete workflow diagram
9. **Metadata**: Show rich dataset metadata

---

## 💾 Dataset Metadata Structure

Each dataset version includes:

### Basic Info
- Dataset ID and name
- Version number
- Creation timestamp
- Task type (classification, regression, etc.)

### Data Splits
- Training samples count
- Validation samples count
- Test samples count

### Features
- Number of features
- Feature types
- Normalization applied

### Quality
- Quality status (baseline, production)
- Manual review status
- Baseline performance metrics

### Preprocessing
- Augmentation applied
- Cleaning performed
- Normalization method

---

## 🚀 Performance

### Storage Efficiency
- Compression: 2-3x reduction
- Deduplication: Share common samples
- Incremental: Only store differences

### Loading Speed
- Small datasets: <1 second
- Medium datasets: 1-5 seconds
- Large datasets: Streaming support

### Version Management
- Instant version switching
- No data duplication
- Efficient storage

---

## ➡️ Next Steps

### Explore Related Demos

**Level 1 (One-Way Integration)**:
- **02-ml-checkpoints**: Automatic checkpoint saving
- **04-model-registry**: Store trained models

**Level 2 (Bidirectional)**:
- **01-data-triggered-compute**: Auto-train on new dataset
- **02-distributed-storage**: Distribute datasets across nodes

**Level 3 (Multi-Primal)**:
- **03-coordinated-compute**: Songbird manages dataset distribution
- **02-encrypted-storage**: BearDog encrypts datasets

---

## 🔧 Configuration

### Version Naming
```bash
# Semantic versioning
STORAGE_KEY="ml-datasets/mnist/v1"
STORAGE_KEY="ml-datasets/mnist/v2"
STORAGE_KEY="ml-datasets/mnist/v3"

# Date-based versioning
STORAGE_KEY="ml-datasets/mnist/2025-12-01"
STORAGE_KEY="ml-datasets/mnist/2025-12-15"
```

### Metadata Customization
```bash
# Add custom metadata
{
  "dataset_info": {
    "project": "your-project",
    "owner": "your-team",
    "license": "MIT",
    "source": "original/kaggle/manual"
  }
}
```

---

## 🎯 Success Criteria

After running this demo, you should understand:
- ✅ How to version datasets
- ✅ Why versioning is valuable
- ✅ How to compare dataset versions
- ✅ When to use versioned storage

---

## 🔍 Deep Dive

### Implementation Pattern

```python
# Pseudocode: Store versioned dataset

def store_dataset_version(dataset, version, nestgate_client):
    # Prepare dataset
    dataset_bytes = serialize_dataset(dataset)
    
    # Create metadata
    metadata = {
        'version': version,
        'samples': len(dataset),
        'features': dataset.feature_count,
        'quality': 'production',
        'created_at': datetime.now()
    }
    
    # Store in NestGate with versioning
    nestgate_client.store(
        key=f"ml-datasets/mnist/{version}",
        data=dataset_bytes,
        metadata=metadata,
        versioned=True  # Enable versioning
    )
```

### Loading Pattern

```python
# Load specific dataset version

def load_dataset(version, nestgate_client):
    # Load from NestGate
    dataset_bytes = nestgate_client.retrieve(
        key=f"ml-datasets/mnist/{version}"
    )
    
    # Deserialize
    dataset = deserialize_dataset(dataset_bytes)
    
    return dataset
```

---

## 📚 References

- **NestGate Storage**: `../../../nestgate-standalone/01-storage-basics/`
- **ML Checkpoints**: `../02-ml-checkpoints/`
- **Model Registry**: `../04-model-registry/`

---

*Demo Level: 1 (One-Way Integration)*  
*Dependencies: NestGate (optional, demo mode available)*  
*Time to Complete: 10 minutes*

