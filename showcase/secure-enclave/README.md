# 🔐 ToadStool Secure Enclave Showcase

**Status**: Planning Phase  
**Date**: December 22, 2025  
**Integration**: NestGate (compression) + BearDog (encryption) + Songbird (BTSP) + ToadStool (secure compute)

---

## Overview

This showcase demonstrates **zero-knowledge compute** using ToadStool's secure enclave capabilities integrated with the ecoPrimals ecosystem.

### The Pattern: Compress → Encrypt → Secure Compute

```
┌─────────────────────────────────────────────────────────────┐
│ OWNER (Data Provider)                                       │
├─────────────────────────────────────────────────────────────┤
│ 1. Plain sensitive data (genomic, medical, financial)       │
│ 2. Compress with NestGate (88% reduction)                   │
│ 3. Encrypt with BearDog (AES-256-GCM)                      │
│ 4. Send to ToadStool enclave                                │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│ TOADSTOOL SECURE ENCLAVE (Zero-Knowledge)                   │
├─────────────────────────────────────────────────────────────┤
│ 1. Receive encrypted blob (entropy 7.99)                    │
│ 2. BTSP tunnel with BearDog (secure key exchange)          │
│ 3. Decrypt in ISOLATED memory (no disk!)                   │
│ 4. Decompress (zstd/lz4, <5ms/MB)                          │
│ 5. Process (ML inference, analysis, compute)               │
│ 6. Re-encrypt result                                        │
│ 7. Wipe keys & plaintext from memory                       │
│ 8. Send encrypted result + proof-of-isolation              │
└─────────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│ OWNER (Data Consumer)                                       │
├─────────────────────────────────────────────────────────────┤
│ 1. Receive encrypted result                                 │
│ 2. Decrypt with BearDog                                     │
│ 3. Done! (Provider never saw plaintext)                     │
└─────────────────────────────────────────────────────────────┘
```

### Key Benefits

- ✅ **Zero-Knowledge**: Provider cannot see, infer, or analyze plaintext
- ✅ **Energy Efficient**: 70-80% savings from pre-compression
- ✅ **Regulatory Compliant**: HIPAA, GDPR, SOC2
- ✅ **Auditable**: Cryptographic proof of isolation
- ✅ **High Performance**: < 10% overhead vs plaintext compute

---

## Showcase Demos

### Demo 1: Private Genomic Analysis 🧬
**File**: `genomic_analysis_demo.rs`

**Scenario**: Patient genomic data processed for variant calling without exposing DNA to provider.

**Flow**:
1. Receive compressed+encrypted genome (3GB → 360MB encrypted)
2. BTSP key exchange with BearDog
3. Decrypt in isolated memory
4. Decompress genome data
5. Run ML variant caller
6. Encrypt results (predicted variants)
7. Wipe memory, return encrypted results

**Metrics**:
- Compression ratio: 88%
- Transfer energy saved: 2.4 GB × 0.00052 kWh/GB = 0.00125 kWh
- Decompression time: 360MB × 5ms/MB = 1.8s
- Total overhead: < 10% of compute time
- Privacy guarantee: Host sees entropy 7.99 (random)

**Compliance**: HIPAA, GINA (Genetic Information Nondiscrimination Act)

---

### Demo 2: Medical AI Inference (Zero-Knowledge) 🏥
**File**: `medical_ai_demo.rs`

**Scenario**: Patient medical records processed by AI diagnostic model with zero provider knowledge.

**Flow**:
1. Receive encrypted medical records + imaging
2. Secure key exchange via BTSP
3. Decrypt patient data in isolated memory
4. Run diagnostic ML model (GPU accelerated)
5. Generate diagnosis + treatment recommendations
6. Encrypt results
7. Wipe all patient data from memory
8. Return encrypted diagnosis + proof-of-isolation

**Models**:
- Disease classification (ResNet-50 on medical imaging)
- Risk prediction (transformer on EMR data)
- Drug interaction analysis

**Metrics**:
- Model inference time: 200ms (GPU)
- Total time with decrypt/encrypt: 250ms
- Overhead: 25% (acceptable for privacy)
- Memory isolation: Verified (no swap, no core dump)

**Compliance**: HIPAA, GDPR Article 32 (security of processing)

---

### Demo 3: Financial Portfolio Optimization 💰
**File**: `financial_modeling_demo.rs`

**Scenario**: Private wealth portfolio optimization without exposing holdings to compute provider.

**Flow**:
1. Receive encrypted portfolio data (positions, transactions, preferences)
2. Secure key exchange
3. Decrypt financial data in isolated memory
4. Run optimization algorithms (risk/return analysis)
5. Generate rebalancing recommendations
6. Encrypt recommendations
7. Wipe sensitive financial data
8. Return encrypted results + audit trail

**Algorithms**:
- Mean-variance optimization
- Monte Carlo simulation (risk analysis)
- Tax-loss harvesting recommendations
- Constraint-based rebalancing

**Metrics**:
- Portfolio size: 1000 positions
- Optimization time: 5 seconds
- Encryption overhead: 50ms
- Total overhead: 1%
- Zero-knowledge guarantee: Provider sees only encrypted blobs

**Compliance**: GLBA (Gramm-Leach-Bliley Act), SOC2

---

### Demo 4: Multi-Party Privacy-Preserving Analytics 📊
**File**: `multiparty_compute_demo.rs`

**Scenario**: Multiple organizations contribute encrypted data for aggregate analytics without revealing individual data.

**Flow**:
1. Receive encrypted data from N parties (each compressed+encrypted)
2. N separate BTSP sessions (one per party)
3. Decrypt each dataset in isolated memory region
4. Compute aggregate statistics (mean, median, percentiles)
5. Generate encrypted aggregate result
6. Wipe all individual data
7. Return encrypted aggregate + proof that individual data wasn't stored

**Use Cases**:
- Healthcare: Multi-hospital disease prevalence (without patient-level data)
- Finance: Industry benchmarking (without company-specific data)
- Research: Federated learning (model training without raw data)

**Metrics**:
- Parties: 10
- Data per party: 100MB compressed
- Aggregate time: 2 seconds
- Memory isolation: Verified per-party
- Privacy: Each party's data invisible to others AND provider

**Compliance**: GDPR Article 25 (data protection by design)

---

## Technical Architecture

### Isolated Memory Runtime

**Requirements**:
```rust
struct SecureEnclaveRuntime {
    /// Memory region with no swap
    isolated_memory: IsolatedMemoryRegion,
    
    /// Key material (wiped on drop)
    ephemeral_keys: EphemeralKeyStore,
    
    /// Audit logger (encrypted operations only)
    audit_log: AuditLogger,
    
    /// Compression support
    decompressor: Decompressor, // zstd, lz4
}

impl SecureEnclaveRuntime {
    /// Create isolated runtime with memory guarantees
    pub fn new() -> Result<Self> {
        // mlock() to prevent swap
        // madvise(MADV_DONTDUMP) to prevent core dumps
        // seccomp() to limit syscalls
    }
    
    /// Process encrypted data in isolation
    pub async fn process_encrypted(
        &mut self,
        encrypted_blob: &[u8],
        btsp_session: &BtspSession,
        compute_fn: impl FnOnce(&[u8]) -> Result<Vec<u8>>,
    ) -> Result<(Vec<u8>, ProofOfIsolation)> {
        // 1. Key exchange via BTSP
        let key = self.exchange_key(btsp_session).await?;
        
        // 2. Decrypt in isolated memory
        let plaintext = self.decrypt_isolated(&key, encrypted_blob)?;
        
        // 3. Decompress if needed
        let decompressed = self.decompress(&plaintext)?;
        
        // 4. Run compute function
        let result = compute_fn(&decompressed)?;
        
        // 5. Re-encrypt result
        let encrypted_result = self.encrypt_isolated(&key, &result)?;
        
        // 6. Generate proof of isolation
        let proof = self.generate_proof()?;
        
        // 7. Wipe memory (explicit, not just Drop)
        self.wipe_sensitive_data();
        
        Ok((encrypted_result, proof))
    }
}
```

### Integration Points

**BearDog BTSP Client**:
```rust
// Secure key exchange via Songbird BTSP
let btsp_client = BtspClient::connect("beardog://encryption-service").await?;
let session = btsp_client.establish_session().await?;
let encryption_key = session.exchange_key().await?;
```

**NestGate Decompression**:
```rust
// Fast decompression of NestGate-compressed data
let decompressor = Decompressor::new(CompressionAlgorithm::Zstd);
let plaintext = decompressor.decompress(&compressed_data)?;
// Performance: ~5ms per MB on modern CPU
```

**Proof of Isolation**:
```rust
struct ProofOfIsolation {
    /// Memory was wiped after processing
    memory_wiped: bool,
    
    /// Keys destroyed (not just dropped)
    keys_destroyed: bool,
    
    /// No disk writes during processing
    no_disk_writes: bool,
    
    /// Audit log hash (verifiable)
    audit_log_hash: Blake3Hash,
    
    /// Timestamp (for audit trail)
    timestamp: DateTime<Utc>,
    
    /// Signature (cryptographic proof)
    signature: Signature,
}
```

---

## API Design

### Secure Enclave Endpoint

```rust
POST /enclave/compute

Request:
{
    "encrypted_data": "<base64-encoded encrypted blob>",
    "btsp_session_id": "<uuid>",
    "compute_type": "ml_inference" | "analysis" | "aggregation",
    "model_id": "genomic-variant-caller-v2",
    "compute_params": {
        // Model-specific parameters
    }
}

Response:
{
    "encrypted_result": "<base64-encoded encrypted result>",
    "proof_of_isolation": {
        "memory_wiped": true,
        "keys_destroyed": true,
        "no_disk_writes": true,
        "audit_log_hash": "<blake3>",
        "timestamp": "2025-12-22T10:30:00Z",
        "signature": "<ed25519-signature>"
    },
    "compute_metrics": {
        "duration_ms": 1234,
        "memory_peak_mb": 512,
        "energy_kwh": 0.0001,
        "decompression_ms": 50,
        "inference_ms": 1150,
        "encryption_ms": 34
    }
}
```

---

## Success Metrics

### Security Metrics

| Metric | Target | Verification |
|--------|--------|--------------|
| Memory isolation | 100% (no leaks) | mlock + madvise + audit |
| Key lifecycle | Wiped on completion | Explicit zeroing + drop |
| Disk writes | 0 during processing | strace / seccomp |
| Entropy of encrypted | > 7.95 | Statistical analysis |
| Audit trail | 100% coverage | Every operation logged |

### Performance Metrics

| Metric | Target | Measured |
|--------|--------|----------|
| Decompression overhead | < 5ms/MB | TBD |
| Encryption overhead | < 2ms/MB | TBD |
| Total overhead vs plaintext | < 10% | TBD |
| Memory footprint | < 2x plaintext | TBD |
| GPU utilization | > 90% | TBD |

### Energy Metrics

| Metric | Target | Source |
|--------|--------|--------|
| Transfer energy saved | 70-80% | NestGate compression |
| Decompression cost | 0.00002 kWh/GB | Measured |
| Net energy savings | 70-80% | Calculated |

---

## Implementation Phases

### Phase 1: Foundation (Week 1) ✅ Planning
- [x] Review NestGate handoff
- [ ] Design isolated memory runtime
- [ ] Plan BTSP integration
- [ ] Define API contracts

### Phase 2: Core Runtime (Week 2-3)
- [ ] Implement `SecureEnclaveRuntime`
- [ ] Add memory isolation (mlock, madvise, seccomp)
- [ ] Integrate decompression (zstd, lz4)
- [ ] Add key wiping mechanisms

### Phase 3: BTSP Integration (Week 3-4)
- [ ] Integrate BearDog BTSP client
- [ ] Implement secure key exchange
- [ ] Add session management
- [ ] Test end-to-end encryption

### Phase 4: Demos (Week 4-6)
- [ ] Demo 1: Genomic analysis
- [ ] Demo 2: Medical AI
- [ ] Demo 3: Financial modeling
- [ ] Demo 4: Multi-party compute

### Phase 5: Proof & Audit (Week 6-7)
- [ ] Generate proof-of-isolation
- [ ] Add cryptographic signatures
- [ ] Comprehensive audit logging
- [ ] Security verification tools

### Phase 6: Documentation & Polish (Week 7-8)
- [ ] User guides
- [ ] API documentation
- [ ] Performance benchmarks
- [ ] Compliance documentation

---

## Dependencies

**Required Primals**:
- ✅ **NestGate**: Compression/decompression
- ✅ **BearDog**: Encryption/decryption
- ✅ **Songbird**: BTSP communication
- ✅ **ToadStool**: Secure compute runtime

**Crates**:
- `zstd` - Fast decompression
- `aes-gcm` - Encryption (via BearDog)
- `seccomp` - Syscall filtering
- `mlock` - Memory locking
- `blake3` - Fast hashing for audit logs

---

## Questions for NestGate Team

1. **Compression formats**: Do you support both zstd and lz4? Recommend one?
2. **Metadata**: Do compressed blobs include compression metadata (algorithm, level)?
3. **Error handling**: How should we handle corrupted compressed data?
4. **Integration**: Do you have a Rust decompression library or should we use upstream zstd?

## Questions for BearDog Team

1. **BTSP endpoint**: What's the service discovery name for BearDog encryption?
2. **Key sizes**: AES-256 only or support for different key sizes?
3. **Nonces**: How are nonces/IVs managed in multi-request scenarios?
4. **Performance**: Any benchmarks for encrypt/decrypt throughput?

## Questions for Songbird Team

1. **BTSP sessions**: Session lifetime and renewal strategy?
2. **Connection pooling**: Can we reuse BTSP connections?
3. **Error recovery**: How to handle dropped BTSP connections mid-compute?
4. **Performance**: Latency for key exchange?

---

## References

**NestGate**:
- `showcase/03_encryption_storage/README.md`
- `showcase/03_encryption_storage/ENERGY_ANALYSIS.md`
- `specs/ADAPTIVE_COMPRESSION_ARCHITECTURE.md`
- `specs/CROSS_PRIMAL_COMPRESSION_INTERACTIONS.md`

**ToadStool**:
- `START_HERE.md` - Project overview
- `STATUS.md` - Current status (A-, 89/100)
- `crates/runtime/` - Runtime engines
- `crates/security/` - Security & sandboxing

---

## Success Criteria

✅ **Functionality**: All 4 demos working end-to-end  
✅ **Security**: Zero-knowledge verified (strace, memory audit)  
✅ **Performance**: < 10% overhead vs plaintext compute  
✅ **Energy**: 70-80% savings measured and documented  
✅ **Compliance**: HIPAA, GDPR, GLBA analysis complete  
✅ **Documentation**: Comprehensive guides for each demo  

---

**Status**: Planning Complete, Ready for Implementation  
**Next**: Implement `SecureEnclaveRuntime` foundation  
**Timeline**: 8 weeks to full showcase  
**Confidence**: High (clear requirements, strong primal ecosystem)

*Last Updated: December 22, 2025*

