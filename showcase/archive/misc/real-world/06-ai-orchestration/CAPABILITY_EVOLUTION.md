# 🎯 Evolution to Capability-Based AI Routing

**From Vendor Lock-In to Universal Capabilities**

---

## 🔄 The Evolution

### **Before: Vendor-Hardcoded** ❌

```rust
// Old approach - hardcoded vendors
match model {
    "gpt-4" => call_openai(prompt),
    "claude-3" => call_anthropic(prompt),
    "llama-3" => call_local_model(prompt),
    _ => panic!("Unknown model"),
}
```

**Problems:**
- Hardcoded vendor names
- Code changes for new providers
- Vendor lock-in
- Difficult to optimize
- No automatic failover

---

### **After: Capability-Based** ✅

```rust
// New approach - capability-based
let requirements = WorkloadRequirements {
    service_type: "ai.text.generation",
    min_tokens: 4000,
    privacy_level: PrivacyLevel::Private,
    max_cost: Money::zero(),
};

// Query Songbird registry
let service = registry.find_best_match(requirements).await?;

// Route automatically
let result = service.execute(prompt).await?;
```

**Benefits:**
- No vendor names in code
- New providers auto-discovered
- No vendor lock-in
- Automatic optimization
- Graceful fallback

---

## 🏗️ Architecture

### **Service Registry (Songbird)**

```
┌────────────────────────────────────────────────────┐
│              Songbird Registry                     │
│                                                    │
│  Services register:                                │
│  - Capabilities (what they can do)                 │
│  - Performance (how fast)                          │
│  - Cost (how much)                                 │
│  - Privacy (where data goes)                       │
│  - Endpoint (how to reach)                         │
└────────────────────────────────────────────────────┘
         ↑                              ↑
         │ Register                     │ Query
         │                              │
    ┌────────┐                    ┌──────────┐
    │ Service│                    │ToadStool │
    │Provider│                    │Orchestr. │
    └────────┘                    └──────────┘
```

### **Capability Matching**

1. **Workload defines requirements**
   ```toml
   [requirements]
   service_type = "ai.text.generation"
   min_tokens = 4000
   privacy_level = "private"
   ```

2. **Services advertise capabilities**
   ```toml
   [capabilities]
   service_type = "ai.text.generation"
   max_tokens = 8192
   privacy.location = "on_premise"
   ```

3. **System matches automatically**
   ```
   Score = capability_match × 0.40 +
           cost_score × 0.25 +
           performance_score × 0.20 +
           privacy_score × 0.10 +
           reliability_score × 0.05
   ```

4. **Best match wins**

---

## 📊 Service Capabilities

### **Core Capability Types**

#### **Service Types** (What it does)
- `ai.text.generation` - Generate text
- `ai.text.completion` - Complete text
- `ai.code.generation` - Generate code
- `ai.vision.understanding` - Understand images
- `ai.research.web` - Web research
- `ai.audio.transcription` - Speech to text
- `ai.audio.synthesis` - Text to speech

#### **Modalities** (Input/Output types)
- `text` - Text in, text out
- `multimodal` - Text + images, etc.
- `audio` - Audio processing
- `video` - Video processing

#### **Performance Metrics**
- `avg_latency_ms` - Response time
- `tokens_per_second` - Throughput
- `reliability` - Uptime percentage
- `max_tokens` - Context length

#### **Cost Model**
- `free` - No cost
- `token` - Per token pricing
- `request` - Per request pricing
- `subscription` - Flat rate

#### **Privacy Levels**
- `on_premise` - Data never leaves
- `private` - E2E encrypted
- `internal` - Within organization
- `public` - OK for cloud

---

## 🔧 Adding New Services

### **Zero Code Changes Required!**

1. **Service Registers with Songbird**
   ```bash
   curl -X POST http://songbird:8080/registry/register \
     -d '{
       "service_id": "new-ai-provider-001",
       "service_type": "ai.text.generation",
       "capabilities": {
         "max_tokens": 8192,
         "languages": ["en", "es"],
         "streaming": true
       },
       "cost": {
         "input_cost_per_1k": 0.001,
         "output_cost_per_1k": 0.003
       },
       "endpoint": {
         "url": "https://new-provider.ai/v1/generate",
         "auth_type": "bearer"
       }
     }'
   ```

2. **ToadStool Auto-Discovers**
   ```
   [INFO] Songbird registry refresh
   [INFO] Discovered new service: new-ai-provider-001
   [INFO] Added to available services
   ```

3. **Immediately Available**
   ```
   Next workload matching capabilities
   → Will consider new service
   → May select it if best match
   ```

**That's it!** No code deployment, no configuration files, no restart.

---

## 🎯 Routing Examples

### **Example 1: Privacy-First**

```toml
[requirements]
privacy_level = "on_premise"  # Hard constraint

[constraints]
max_cost = 0.0  # Must be free
```

**Match:** Only local services
- Local Ollama
- Local GPU models
- Self-hosted services

**Reject:** All cloud APIs (even if cheaper/faster)

---

### **Example 2: Cost-Optimized**

```toml
[requirements]
service_type = "ai.text.generation"

[constraints]
max_cost_per_request = 0.05

[preferences]
optimize_for = "cost"
```

**Match:** Cheapest service meeting requirements
- GPT-3.5-turbo ($0.0015/1K)
- Claude Haiku ($0.00025/1K)
- Local models ($0.00)

**Selected:** Local if available, else cheapest cloud

---

### **Example 3: Performance-Optimized**

```toml
[requirements]
service_type = "ai.text.generation"
min_tokens = 100000  # Long context

[constraints]
max_latency_ms = 2000

[preferences]
optimize_for = "performance"
quality = "high"
```

**Match:** Services with long context + fast response
- Claude Opus (200K tokens)
- GPT-4-turbo (128K tokens)

**Selected:** Best performance/quality match

---

### **Example 4: Specialized Task**

```toml
[requirements]
service_type = "ai.research.web"
web_search = true
citations = true
real_time_data = true
```

**Match:** Only research-capable services
- Perplexity AI
- You.com AI Search
- Custom research agents

**Reject:** Standard text generation (no web access)

---

## 🔄 Protocol Adapters

Services don't need to use a specific API format. We use **adapters**:

### **Supported Protocols**

1. **OpenAI-Compatible**
   - OpenAI GPT
   - Together AI
   - Many others

2. **Anthropic-Compatible**
   - Anthropic Claude
   - Future compatible services

3. **Ollama-Compatible**
   - Ollama
   - LocalAI
   - LM Studio

4. **Generic HTTP/JSON**
   - Any REST API
   - Custom protocols

**Adding new protocol:** Just add adapter definition, no code changes.

---

## 📈 Benefits

### **1. No Vendor Lock-In**
Switch providers without code changes:
- OpenAI → Claude → Llama
- Just update registry
- Workloads unaffected

### **2. Cost Optimization**
System automatically finds cheapest option:
- Compares all providers
- Considers performance/quality
- Balances cost vs requirements

### **3. Privacy Enforcement**
Hard constraints on data location:
- Private data → local only
- Public data → cloud OK
- Automatic compliance

### **4. Graceful Degradation**
If service unavailable:
- Try next best match
- Fallback to alternatives
- Never fails completely

### **5. Load Balancing**
Multiple services with same capabilities:
- Distribute requests
- Health checks
- Auto-failover

---

## 🔍 Discovery Flow

```
1. Workload submitted to ToadStool
   ↓
2. ToadStool queries Songbird registry
   "What services can do text generation?"
   ↓
3. Songbird returns matching services
   [service-1, service-2, service-3]
   ↓
4. ToadStool scores each service
   Capability: 95%, Cost: 80%, Privacy: 100%
   ↓
5. Select best match
   service-2 scores highest
   ↓
6. Resolve endpoint via Songbird
   https://service-2.example.com/api
   ↓
7. Squirrel executes request
   Uses appropriate protocol adapter
   ↓
8. Return result to user
```

---

## 🌟 Real-World Example

### **Scenario: Code Review**

**Old Way (Hardcoded):**
```rust
// Hardcoded vendor
let result = openai::chat_completion("gpt-4", prompt)?;
```

**Problem:** What if GPT-4 is down? Or too expensive? Or violates privacy policy?

**New Way (Capability-Based):**
```rust
let requirements = WorkloadRequirements {
    service_type: ServiceType::TextGeneration,
    privacy: PrivacyLevel::Private,  // Must be local
    min_tokens: 4000,
    code_understanding: true,
};

let result = orchestrator.execute(requirements, prompt).await?;
```

**What happens:**
1. Query registry: "text generation + private + code"
2. Find matches: [local-llama-3, local-codellama]
3. Score: local-codellama scores higher (specialized)
4. Execute: route to local-codellama
5. Result: Code review completed locally

**Benefits:**
- Privacy enforced (never touched cloud)
- Cost: $0.00 (local execution)
- Optimal: Used specialized code model
- Flexible: If local unavailable, policy decides next step

---

## 🚀 Migration Path

### **Phase 1: Add Registry** ✅
- Deploy Songbird
- Services register capabilities
- Keep existing code working

### **Phase 2: Dual Mode** ✅
- New workloads use capability-based
- Old workloads use legacy paths
- Both work simultaneously

### **Phase 3: Full Migration** 🎯
- All workloads capability-based
- Remove vendor-specific code
- Pure capability routing

### **Phase 4: Advanced** 🔮
- ML-based routing optimization
- Predictive service selection
- Cost forecasting
- Auto-negotiation

---

## 📝 Configuration Example

See `capability-registry.toml` for full example.

**Key Sections:**

1. **Service Definitions**
   - What services are available
   - Their capabilities
   - Cost/performance metrics

2. **Workload Profiles**
   - Common use cases
   - Requirements templates
   - Reusable configurations

3. **Routing Rules**
   - How to score matches
   - Fallback strategies
   - Load balancing

4. **Discovery Config**
   - Songbird endpoint
   - Refresh intervals
   - Health checks

---

## 🎯 Key Takeaways

1. **Think Capabilities, Not Vendors**
   - "I need X" not "Use Y"
   
2. **Services Self-Register**
   - No manual configuration
   - Auto-discovery
   
3. **Automatic Optimization**
   - Best match selected
   - Cost/performance balanced
   
4. **Zero Lock-In**
   - Swap providers anytime
   - No code changes
   
5. **Privacy by Design**
   - Hard constraints enforced
   - Automatic compliance

---

## 🌟 The Future

**This is just the beginning!**

Next evolution:
- **ML-based routing** - Learn optimal selections
- **Predictive scaling** - Pre-warm services
- **Auto-negotiation** - Dynamic pricing
- **Federated registry** - Multi-region discovery
- **Smart caching** - Reduce costs further

**The foundation is built. Now we iterate!**

---

*Last Updated: December 8, 2025*  
*Status: Architecture Complete ✅*  
*Ready for Implementation*

