# 🌿 ecoPrimals Integration Guide
**ToadStool + Songbird + Squirrel**

---

## 🎯 Overview

This document explains how ToadStool, Songbird, and Squirrel work together to create a universal, distributed, intelligent compute platform.

---

## 🏗️ The Three Primals

### 🍄 **ToadStool** - Universal Compute Orchestration
**Role**: Workload orchestration, resource management, execution

**Capabilities**:
- Multi-runtime execution (Native, Container, Python, GPU)
- Resource allocation and scheduling
- Workload lifecycle management
- Capability-based compute matching
- Security and isolation

**What it manages**:
- CPU/GPU resources
- Memory and storage
- Process execution
- Resource constraints

---

### 🐦 **Songbird** - Distributed Message Routing
**Role**: Inter-service communication, distributed coordination

**Capabilities**:
- Message routing between services
- Publish/subscribe patterns
- Request/response coordination
- Event streaming
- Load balancing

**What it routes**:
- Service-to-service messages
- AI agent communications
- Distributed task coordination
- Event notifications

---

### 🐿️ **Squirrel** - AI Model & API Management
**Role**: AI model lifecycle, API gateway, intelligent routing

**Capabilities**:
- Local model management (loading, caching, versioning)
- Cloud API integration (OpenAI, Anthropic, Google, etc.)
- Intelligent routing (local vs cloud)
- Cost tracking and optimization
- Privacy-aware decisions

**What it manages**:
- AI models (Llama, Mistral, CodeLlama, etc.)
- API credentials and rate limiting
- Model selection strategies
- Cost budgets

---

## 🔄 How They Work Together

### **Architecture Diagram**

```
┌──────────────────────────────────────────────────────────┐
│                    User/Application                      │
└────────────────────┬─────────────────────────────────────┘
                     │
                     ▼
           ┌─────────────────────┐
           │  🍄 ToadStool       │  ← Universal Orchestrator
           │  (Orchestration)    │     - Receives requests
           │                     │     - Analyzes requirements
           └──────────┬──────────┘     - Coordinates execution
                      │
        ┏━━━━━━━━━━━━┻━━━━━━━━━━━━┓
        ▼                          ▼
┌───────────────┐          ┌──────────────┐
│  🐦 Songbird  │          │ GPU Runtime  │
│  (Messaging)  │          │ CPU Runtime  │
└───────┬───────┘          └──────┬───────┘
        │                         │
        │   ┌─────────────────────┘
        │   │
        ▼   ▼
┌─────────────────┐
│  🐿️  Squirrel   │  ← AI Gateway
│  (AI Management)│     - Manages models
└────────┬────────┘     - Routes to APIs
         │
   ┏━━━━━┻━━━━━┓
   ▼           ▼
┌──────┐  ┌─────────┐
│Local │  │ Cloud   │
│AI    │  │ APIs    │
│Models│  │ (GPT-4, │
│      │  │ Claude) │
└──────┘  └─────────┘
```

---

## 📊 Integration Patterns

### **Pattern 1: Simple AI Request**

```
1. User → ToadStool: "Analyze this code"
2. ToadStool → Songbird: Route AI request
3. Songbird → Squirrel: Forward to AI gateway
4. Squirrel: Check requirements
   - Privacy: HIGH → Select local model
   - Load Llama 3 on GPU
5. Squirrel → ToadStool: Execute on GPU
6. ToadStool: Run inference
7. ToadStool → Songbird: Return results
8. Songbird → User: Deliver response
```

**Flow**:
```
User → ToadStool → Songbird → Squirrel → Local AI → Results
```

---

### **Pattern 2: Distributed Workload**

```
1. User → ToadStool: Large computation task
2. ToadStool: Split into 10 subtasks
3. ToadStool → Songbird: Distribute subtasks
4. Songbird: Route to multiple workers
   - Worker A: Subtasks 1-3
   - Worker B: Subtasks 4-6
   - Worker C: Subtasks 7-10
5. Workers → ToadStool: Execute in parallel
6. ToadStool: Aggregate results
7. ToadStool → User: Return final result
```

**Flow**:
```
                    ┌→ Worker A
User → ToadStool → Songbird ─→ Worker B → ToadStool → User
                    └→ Worker C
```

---

### **Pattern 3: Hybrid AI Pipeline**

```
1. User → ToadStool: "Research + Analyze + Report"

Stage 1 - Research (Cloud):
2. ToadStool → Songbird: Route research request
3. Songbird → Squirrel: Forward to AI gateway
4. Squirrel → Perplexity API: Web research
5. Perplexity → Squirrel: Research data
6. Squirrel → Songbird: Return data

Stage 2 - Analysis (Local):
7. Songbird → Squirrel: Route analysis
8. Squirrel: Privacy HIGH → Local model
9. Squirrel → ToadStool GPU: Execute Llama 3
10. ToadStool → Squirrel: Analysis results

Stage 3 - Report (Cloud):
11. Squirrel → Claude API: Generate report
12. Claude → Squirrel: Formatted report
13. Squirrel → Songbird: Final result
14. Songbird → User: Deliver report
```

**Flow**:
```
User → ToadStool → Songbird → Squirrel → [Cloud API]
                                    ↓
                           [Local GPU Analysis]
                                    ↓
                            [Cloud API Report]
                                    ↓
                Songbird ← Squirrel ← Results → User
```

---

## 🎯 Real-World Example

### **Scenario: AI-Powered Code Review**

```python
# User submits code for review
code = """
def transfer_money(from_account, to_account, amount):
    from_account.balance -= amount  # BUG: No validation!
    to_account.balance += amount
"""

# 1. ToadStool receives request
request = {
    "task": "code_review",
    "code": code,
    "privacy": "high",  # User's proprietary code
}

# 2. ToadStool analyzes
toadstool.analyze(request)
→ Privacy: HIGH → Local AI required
→ Complexity: MEDIUM → Local model sufficient
→ Decision: Route to local Llama 3

# 3. Songbird routes message
songbird.route(
    from="toadstool",
    to="squirrel",
    message={"action": "analyze_code", "code": code}
)

# 4. Squirrel selects model
squirrel.select_model(
    task="code_review",
    privacy="high",
    complexity="medium"
)
→ Selected: codellama-7b (local model)

# 5. ToadStool executes on GPU
toadstool.gpu_runtime.execute(
    model="codellama-7b",
    input=code
)

# 6. Results returned via Songbird
songbird.return_result({
    "issues_found": [
        "Missing balance validation",
        "No error handling",
        "Race condition possible"
    ],
    "latency": "145ms",
    "cost": "$0.00",
    "privacy": "100% local"
})

# 7. User receives results
✅ Code analyzed privately, instantly, free!
```

---

## 🔧 Configuration

### **ToadStool Configuration**
```toml
# toadstool.toml
[orchestration]
enable_distributed = true
message_broker = "songbird"
ai_gateway = "squirrel"

[runtimes]
gpu = { enabled = true, device = "cuda:0" }
cpu = { enabled = true }
```

---

### **Songbird Configuration**
```toml
# songbird.toml
[routing]
mode = "intelligent"

[endpoints]
toadstool = "songbird://localhost:7878/toadstool"
squirrel = "songbird://localhost:8080/squirrel"
workers = [
    "songbird://worker1:7878/compute",
    "songbird://worker2:7878/compute",
]

[patterns]
ai_requests = { route_to = "squirrel", priority = "high" }
compute_tasks = { route_to = "workers", load_balance = true }
```

---

### **Squirrel Configuration**
```toml
# squirrel.toml
[local_models]
llama3 = { path = "/models/llama-3-8b", device = "cuda:0" }
codellama = { path = "/models/codellama-7b", device = "cuda:0" }

[cloud_apis]
openai = { models = ["gpt-4", "gpt-3.5-turbo"] }
anthropic = { models = ["claude-3-opus"] }

[routing]
privacy_high = "local"
complexity_high = "cloud"
```

---

## 📈 Benefits of Integration

### **1. Distributed Intelligence**
- ToadStool orchestrates
- Songbird connects
- Squirrel optimizes

### **2. Privacy-Preserving**
- Sensitive data → Local models (ToadStool GPU)
- Public data → Cloud APIs (Squirrel gateway)
- Routing automatic (Songbird + Squirrel)

### **3. Cost-Optimized**
- 90%+ savings with hybrid approach
- Automatic cost tracking
- Budget enforcement

### **4. Scalable**
- Add workers via Songbird
- Load balance automatically
- Fault tolerant

### **5. Universal**
- Any compute resource (ToadStool)
- Any AI model/API (Squirrel)
- Any message pattern (Songbird)

---

## 🚀 Getting Started

### **1. Install All Three Primals**
```bash
# ToadStool
cargo build --release

# Songbird
# (Follow Songbird installation guide)

# Squirrel
# (Follow Squirrel installation guide)
```

---

### **2. Run the Integration Demo**
```bash
cd showcase/real-world/06-ai-orchestration
./demo.sh hybrid
```

---

### **3. Try Your Own Workloads**
```bash
# Create a workload with AI
cat > my-ai-task.toml << EOF
[workload]
name = "my-task"
type = "ai-processing"

[ai]
task = "code_review"
privacy = "high"
complexity = "medium"
EOF

# Execute via ToadStool
toadstool-cli execute my-ai-task.toml
```

---

## 📚 Learn More

### **ToadStool**
- [README.md](../README.md) - Project overview
- [GPU Runtime](../crates/runtime/gpu/README.md) - GPU compute
- [Universal Compute](../docs/sessions/dec-8-2025/⭐_MASTER_SESSION_SUMMARY_DEC_8_2025.md) - Architecture

### **Songbird**
- Distributed message routing
- Pub/sub patterns
- Load balancing

### **Squirrel**
- AI model management
- API gateway
- Cost optimization

---

## 🌟 Summary

**The Power of Three Primals**:

| Primal | Role | Key Value |
|--------|------|-----------|
| 🍄 ToadStool | Orchestration | Universal compute |
| 🐦 Songbird | Communication | Distributed coordination |
| 🐿️ Squirrel | AI Management | Intelligent routing |

**Together**: A complete, universal, distributed, intelligent compute platform! 🚀

---

**🌿 Three Primals. One Ecosystem. Infinite Possibilities.** ✨

---

*Last Updated: December 8, 2025*

