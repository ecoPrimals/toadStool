# 🧠 AI Orchestration Showcase
**Scenario**: Capability-based AI routing with zero vendor lock-in

---

## 🌟 Key Innovation: Capability-Based Routing

**No more hardcoded vendors!** Services register capabilities with Songbird, workloads specify requirements:

```toml
# ❌ Old Way: Hardcoded vendors
if task == "code_review":
    use_openai_gpt4()

# ✅ New Way: Capability-based
requirements = {
    service_type: "ai.text.generation",
    privacy_level: "private",
    cost: "free"
}
service = registry.find_best_match(requirements)
```

**Benefits:**
- ✅ No vendor lock-in
- ✅ New providers without code changes
- ✅ Automatic optimization
- ✅ Privacy enforcement

**See:** `CAPABILITY_EVOLUTION.md` for full architecture.

---

## 🎯 What This Demonstrates

This showcase demonstrates ToadStool orchestrating AI workloads across:
- **ToadStool**: Universal compute orchestration
- **Songbird**: Distributed message routing between AI agents
- **Squirrel**: AI model management and API gateway
- **Local AI**: Running models locally (e.g., Llama, Mistral)
- **Cloud AI**: Connecting to external APIs (e.g., OpenAI, Anthropic)

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      ToadStool Orchestrator                 │
│         (Universal Compute + Workflow Management)           │
└─────────────────────┬───────────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┐
        │             │             │
        ▼             ▼             ▼
┌──────────────┐ ┌──────────┐ ┌────────────────┐
│   Songbird   │ │ Squirrel │ │  GPU Runtime   │
│   (Routing)  │ │ (AI Mgmt)│ │ (Local Models) │
└──────┬───────┘ └────┬─────┘ └────────┬───────┘
       │              │                 │
       │     ┌────────┴────────┐       │
       │     │                 │       │
       ▼     ▼                 ▼       ▼
   ┌─────────────┐      ┌──────────────────┐
   │  Local AI   │      │   Cloud AI APIs  │
   │  (Llama 3)  │      │ (OpenAI/Claude)  │
   └─────────────┘      └──────────────────┘
```

---

## 🎬 Demo Scenario

### Part 1: Local AI Processing
1. ToadStool receives request: "Analyze this code for bugs"
2. Routes to local Llama 3 model via GPU runtime
3. Model processes on local GPU (private, fast, free)
4. Returns results via Songbird message bus

### Part 2: Cloud AI Fallback
1. ToadStool receives complex request: "Write a novel chapter"
2. Local model determines it's too complex
3. Routes to Squirrel AI gateway
4. Squirrel forwards to Claude API
5. Results streamed back via Songbird

### Part 3: Hybrid AI Pipeline
1. Request: "Research topic, summarize, and create presentation"
2. **Step 1**: Squirrel routes web search to API
3. **Step 2**: Local Llama summarizes results (private data)
4. **Step 3**: Cloud API creates slides (formatting expertise)
5. All coordinated via ToadStool + Songbird

---

## 🚀 What Makes This Powerful

### **Privacy + Power**
- Sensitive data → Local AI (stays on your machine)
- Complex tasks → Cloud AI (leverage cutting-edge models)
- Automatic routing based on requirements

### **Cost Optimization**
- Simple queries → Free local models
- Complex queries → Paid APIs (only when needed)
- **Savings**: ~90% vs using cloud APIs for everything

### **Performance**
- Local: ~50-200ms latency
- Cloud: ~1-5s latency
- Smart routing uses local whenever possible

---

## 📊 Demo Flow

```
User Request: "Analyze this code for security issues"
     │
     ▼
ToadStool: Analyze request requirements
     │
     ├──> Requires: Code analysis
     ├──> Privacy: HIGH (user's code)
     ├──> Complexity: MEDIUM
     └──> Decision: Route to LOCAL AI
     │
     ▼
Songbird: Route message to local AI agent
     │
     ▼
GPU Runtime: Execute Llama 3 on local GPU
     │
     ├──> Load model (cached, instant)
     ├──> Tokenize input
     ├──> Run inference
     └──> Generate response
     │
     ▼
Songbird: Return results to user
     │
     ▼
User receives: Security analysis (private, fast, free!)


User Request: "Write a business plan for a startup"
     │
     ▼
ToadStool: Analyze request requirements
     │
     ├──> Requires: Long-form generation
     ├──> Privacy: LOW (general business info)
     ├──> Complexity: HIGH
     └──> Decision: Route to CLOUD AI
     │
     ▼
Squirrel: Select best API for task
     │
     ├──> Task type: Long-form writing
     ├──> Available APIs: OpenAI, Anthropic, Google
     └──> Selected: Claude (best for writing)
     │
     ▼
Songbird: Forward request to Squirrel gateway
     │
     ▼
Squirrel: Call Claude API
     │
     ├──> Format request
     ├──> Make API call
     ├──> Stream response
     └──> Track costs
     │
     ▼
Songbird: Stream results back to user
     │
     ▼
User receives: Complete business plan
     │
ToadStool logs: API cost $0.23


User Request: "Hybrid pipeline - Research + Analyze + Report"
     │
     ▼
ToadStool: Create multi-stage workflow
     │
     ├──> Stage 1: Research (Cloud - Perplexity API)
     ├──> Stage 2: Analysis (Local - Llama 3)
     └──> Stage 3: Report (Cloud - Claude)
     │
     ▼
Songbird: Orchestrate multi-agent workflow
     │
     ├──> Agent 1: Research assistant (Squirrel→Perplexity)
     │   └──> Returns: Raw research data
     │
     ├──> Agent 2: Analyzer (Local Llama)
     │   └──> Returns: Key insights (private analysis)
     │
     └──> Agent 3: Writer (Squirrel→Claude)
         └──> Returns: Final report
     │
     ▼
ToadStool: Aggregate results and deliver
     │
User receives: Complete research report
     │
Cost: $0.15 (hybrid: 60% local, 40% cloud)
vs $2.50 (100% cloud) = 94% savings!
```

---

## 🎯 Key Features Demonstrated

### **1. Intelligent Routing**
```rust
match request {
    Simple & Private => LocalAI,
    Complex & Public => CloudAI,
    Hybrid => Pipeline(Local, Cloud),
}
```

### **2. Message-Based Coordination**
```
ToadStool → Songbird → AI Agents
   ↓          ↓           ↓
  Orchestrate Route    Execute
```

### **3. GPU Resource Management**
```
Local AI Models:
  - Llama 3 8B:  4GB VRAM, ~200 tokens/sec
  - Mistral 7B:  6GB VRAM, ~150 tokens/sec
  - CodeLlama:   4GB VRAM, ~180 tokens/sec

ToadStool manages:
  - Model loading/unloading
  - GPU memory allocation
  - Concurrent requests
  - Automatic batching
```

### **4. Cost Tracking**
```
Local Requests:  1,247 requests, $0.00
Cloud Requests:    153 requests, $12.45
Total Saved: $298.55 (vs 100% cloud)
```

---

## 🛠️ Technical Implementation

### **Message Flow (Songbird)**
```toml
# Songbird configuration
[routing]
local_ai_agent = "toadstool://localhost:7878/ai/local"
cloud_ai_agent = "toadstool://squirrel:8080/ai/cloud"

[patterns]
# Route based on content
sensitive_data = { destination = "local_ai_agent", priority = "high" }
complex_generation = { destination = "cloud_ai_agent", priority = "normal" }
```

### **AI Models (Squirrel)**
```toml
# Squirrel AI gateway config
[local_models]
llama3 = { path = "/models/llama-3-8b", device = "cuda:0", max_tokens = 4096 }
mistral = { path = "/models/mistral-7b", device = "cuda:0", max_tokens = 8192 }

[cloud_apis]
openai = { api_key = "sk-...", models = ["gpt-4", "gpt-3.5-turbo"] }
anthropic = { api_key = "sk-ant-...", models = ["claude-3-opus", "claude-3-sonnet"] }
google = { api_key = "AIza...", models = ["gemini-pro"] }

[routing_strategy]
simple_qa = "local"  # Use Llama for Q&A
code_review = "local"  # Use CodeLlama for code
creative_writing = "cloud"  # Use Claude for writing
research = "cloud"  # Use Perplexity for research
```

### **Workload Definition (ToadStool)**
```toml
# ai-orchestration.toml
[workload]
name = "ai-orchestration-demo"
type = "ai-pipeline"
privacy_level = "mixed"

[stages]
[stages.1]
name = "intake"
description = "Receive and classify request"
agent = "toadstool-classifier"
routes_to = ["stage.2.local", "stage.2.cloud"]

[stages.2.local]
name = "local-ai"
description = "Process with local model"
agent = "llama3"
conditions = ["privacy == high", "complexity <= medium"]
resources.gpu_memory = "4GB"
resources.max_latency = "500ms"

[stages.2.cloud]
name = "cloud-ai"
description = "Process with cloud API"
agent = "squirrel-gateway"
conditions = ["complexity >= high", "privacy == low"]
resources.max_cost = "$0.50"

[stages.3]
name = "response"
description = "Format and return results"
agent = "toadstool-formatter"
```

---

## 💡 Real-World Use Cases

### **1. Software Development**
- Code review → Local AI (private code)
- Documentation → Local AI (free)
- Complex refactoring → Cloud AI (expertise)

### **2. Content Creation**
- Draft writing → Local AI (private)
- Research → Cloud AI (web access)
- Final editing → Local AI (style)

### **3. Customer Support**
- FAQ answers → Local AI (instant, free)
- Complex issues → Cloud AI (better reasoning)
- Response routing → ToadStool (intelligent)

### **4. Data Analysis**
- Simple analytics → Local AI (fast)
- Report generation → Local AI (private data)
- Advanced insights → Cloud AI (complex reasoning)

---

## 📈 Performance Metrics

### **Latency Comparison**
```
Local AI (Llama 3 8B):
  - Cold start: 2-5s (model loading)
  - Warm: 50-200ms
  - Throughput: ~200 tokens/sec

Cloud AI (Claude API):
  - Network latency: 50-150ms
  - Processing: 1-5s
  - Throughput: ~100 tokens/sec (varies)

Hybrid (ToadStool orchestrated):
  - Simple queries: 50-200ms (local)
  - Complex queries: 1-5s (cloud)
  - Best of both worlds!
```

### **Cost Comparison**
```
Scenario: 10,000 AI requests/month

100% Cloud:
  - Cost: $250/month
  - All requests → API

100% Local:
  - Cost: $0/month (hardware amortized)
  - Limited by local GPU capability

Hybrid (ToadStool):
  - Cost: $15-30/month
  - 85-90% local, 10-15% cloud
  - Savings: 88-94%!
```

---

## 🔒 Privacy & Security

### **Data Classification**
```rust
enum DataPrivacy {
    Public,     // OK for cloud APIs
    Internal,   // Prefer local, cloud if anonymized
    Private,    // Local only, never cloud
    Sensitive,  // Local + encrypted storage
}

// ToadStool automatically routes based on classification
```

### **Privacy Features**
- ✅ Local AI for sensitive data
- ✅ Automatic PII detection
- ✅ Cloud requests can be anonymized
- ✅ Audit logs for all AI requests
- ✅ User control over routing decisions

---

## 🎮 Running The Demo

### **Prerequisites**
```bash
# All three primals should be in ecoPrimals directory:
# - /home/eastgate/Development/ecoPrimals/toadstool
# - /home/eastgate/Development/ecoPrimals/songbird
# - /home/eastgate/Development/ecoPrimals/squirrel

# API keys should be at:
# - /home/eastgate/Development/ecoPrimals/testing-secrets/api-keys.toml
```

### **Run Integrated Demo** (Recommended)
```bash
cd 06-ai-orchestration

# This starts all three primals and runs the demo
./run-integrated-demo.sh
```

This script will:
1. ✅ Check for all three primals
2. ✅ Start Songbird (message routing) on port 8080
3. ✅ Start Squirrel (AI gateway) on port 9090
4. ✅ Start ToadStool (orchestrator) on port 7878
5. ✅ Load API keys from testing-secrets
6. ✅ Run the demo scenarios
7. ✅ Show how all three work together!

### **Run Visual Demo Only** (Simulation)
```bash
# Just run the visual demo (simulates services)
./demo.sh hybrid       # Smart routing (default)
./demo.sh local-only   # Local AI only
./demo.sh cloud-only   # Cloud APIs only
```

### **Expected Output**
```
🧠 ToadStool AI Orchestration Demo
═══════════════════════════════════

Scenario 1: Local AI Processing
  Request: "Analyze this code"
  Routed to: Local Llama 3
  Latency: 145ms
  Cost: $0.00
  ✅ Complete

Scenario 2: Cloud AI Fallback
  Request: "Write a novel chapter"
  Routed to: Claude API
  Latency: 2.3s
  Cost: $0.15
  ✅ Complete

Scenario 3: Hybrid Pipeline
  Request: "Research + Analyze + Report"
  Stage 1 (Research): Perplexity API - 1.2s, $0.05
  Stage 2 (Analysis): Local Llama - 0.3s, $0.00
  Stage 3 (Report): Claude API - 2.1s, $0.10
  Total: 3.6s, $0.15
  vs 100% Cloud: 8.5s, $0.45
  Savings: 67%!
  ✅ Complete

Summary:
  Local requests: 1,247 (94%)
  Cloud requests: 83 (6%)
  Total cost: $12.45
  vs Cloud-only: $298.55
  Savings: $286.10 (96%)!
```

---

## 🎓 Key Learnings

After running this demo, you'll understand:

1. **AI Orchestration**: How to coordinate multiple AI models and APIs
2. **Message Routing**: Using Songbird for distributed AI agent communication
3. **Smart Selection**: Automatic routing between local and cloud AI
4. **Cost Optimization**: 90%+ savings with hybrid approach
5. **Privacy Management**: Keeping sensitive data on local models
6. **GPU Management**: ToadStool's universal compute for AI workloads

---

## 🚀 Next Steps

### **Extend The Demo**
- Add more local models (Mistral, CodeLlama, etc.)
- Integrate more cloud APIs (Google, Cohere, etc.)
- Create custom routing rules
- Build your own AI pipeline

### **Production Deployment**
- Scale with multiple ToadStool nodes
- Add load balancing across models
- Implement caching for common requests
- Monitor costs and optimize routing

---

## 📚 Related Documentation

- **[ToadStool GPU Runtime](../../crates/runtime/gpu/README.md)** - GPU compute for local AI
- **[Songbird Integration](../../docs/songbird-integration.md)** - Message routing
- **[Squirrel AI Gateway](../../docs/squirrel-integration.md)** - AI model management

---

**🧠 ToadStool + Songbird + Squirrel = Universal AI Orchestration**

*Privacy-preserving, cost-optimized, intelligence-amplifying* 🚀

---

*Last Updated: December 8, 2025*

