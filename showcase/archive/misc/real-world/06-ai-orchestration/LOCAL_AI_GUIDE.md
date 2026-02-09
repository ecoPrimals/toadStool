# 🍄 Local AI on ToadStool Compute - Complete Guide

**Run AI models locally + connect to cloud APIs**

---

## 🎯 What You'll Get

### **Local AI on ToadStool**
- Real models running on your machine
- TinyLlama, Llama 3.2, Phi-3
- 100% private (data never leaves)
- Zero cost
- Fast response times

### **Cloud AI via Squirrel**
- OpenAI GPT-3.5/GPT-4
- Anthropic Claude
- High quality when needed

### **Image Generation**
- Stable Diffusion 2.1
- HuggingFace API
- Local prompt enhancement
- Free tier access

---

## 🚀 Quick Start (3 Steps)

### **Step 1: Setup Local AI** (5-10 minutes, one-time)

```bash
cd showcase/real-world/06-ai-orchestration
./setup-local-ai.sh
```

**What happens:**
1. Installs Ollama (local AI runtime)
2. Downloads TinyLlama model (~1GB)
3. Starts local AI server
4. Tests that it works

**Requirements:**
- ~2GB disk space
- Internet connection (for download)
- Linux/macOS (Windows via WSL)

---

### **Step 2: Run Hybrid Demo** (5 minutes)

```bash
./local-cloud-hybrid.sh
```

**What you'll see:**
1. **Local AI** processes private data (ToadStool)
2. **Cloud AI** handles complex queries (Squirrel)
3. **Hybrid pipeline** combines both for optimal cost/quality
4. **Real responses** - not simulated!

---

### **Step 3: Generate Images** (30 seconds)

```bash
./generate-image-demo.sh
```

**What happens:**
1. Local AI enhances your prompt
2. Songbird routes to image service
3. Stable Diffusion generates image
4. **Image saved locally**: `./generated_images/generated_*.png`

---

## 🏗️ Architecture

### **ToadStool Local Compute**
```
┌─────────────────────────┐
│  🍄 ToadStool Compute   │
│                         │
│  ┌──────────────────┐   │
│  │  Ollama Runtime  │   │
│  ├──────────────────┤   │
│  │  TinyLlama 1.1B  │   │
│  │  Llama 3.2 1B/3B │   │
│  │  Phi-3 Mini      │   │
│  └──────────────────┘   │
│                         │
│  http://localhost:11434 │
└─────────────────────────┘
```

**Properties:**
- Runs on this machine
- 100% private
- Zero cost
- Always available
- No rate limits

---

### **Hybrid Orchestration**

```
User Request
     ↓
🐦 Songbird (deterministic routing)
     ↓
     ├─→ Privacy=HIGH? → 🍄 ToadStool Local AI
     │                    ↓
     │                   Free, Private, Fast
     │
     └─→ Quality=HIGH? → 🐿️ Squirrel Cloud AI
                          ↓
                         Powerful, Costly, Accurate
```

---

## 📊 Performance Comparison

### **Local AI (ToadStool)**

| Model | Size | Speed | Quality | Cost | Privacy |
|-------|------|-------|---------|------|---------|
| TinyLlama | 1.1GB | Fast | Good | $0 | 100% |
| Llama 3.2 1B | 1GB | Fast | Good+ | $0 | 100% |
| Llama 3.2 3B | 2GB | Medium | Very Good | $0 | 100% |
| Phi-3 Mini | 2GB | Medium | Excellent | $0 | 100% |

**Best for:**
- Code completion
- Simple questions
- Private data
- Drafts
- High-volume tasks

---

### **Cloud AI (Squirrel)**

| Model | Speed | Quality | Cost/1K | Privacy |
|-------|-------|---------|---------|---------|
| GPT-3.5 | Fast | Excellent | $0.002 | Cloud |
| GPT-4 | Slow | Superior | $0.03 | Cloud |
| Claude 3 | Medium | Excellent | $0.015 | Cloud |

**Best for:**
- Complex reasoning
- Professional writing
- Latest knowledge
- High quality needed

---

### **Image Generation**

| Service | Model | Speed | Quality | Cost | Privacy |
|---------|-------|-------|---------|------|---------|
| HuggingFace | SD 2.1 | 10-30s | Good | Free | Cloud |
| Local SD | SD 1.5 | 5-15s | Good | $0 | 100% |

**Note**: Local SD requires GPU (future feature)

---

## 💡 Use Cases

### **1. Code Review (Privacy Critical)**

```bash
# Request routes to local AI automatically
Privacy: HIGH → ToadStool Local AI
Cost: $0.00
Data: Never leaves machine
```

**Example:**
```
User: "Review this code for security issues: [sensitive code]"
→ 🐦 Songbird: Privacy=HIGH → Route to local
→ 🍄 ToadStool: Process with TinyLlama
→ Result: Private review, zero cost
```

---

### **2. Business Document (Quality Critical)**

```bash
# Request routes to cloud AI for quality
Quality: HIGH → Squirrel Cloud AI (GPT-4)
Cost: ~$0.15
Data: OK for cloud (not sensitive)
```

**Example:**
```
User: "Write professional business proposal"
→ 🐦 Songbird: Quality=HIGH → Route to cloud
→ 🐿️ Squirrel: Select GPT-4
→ Result: High-quality output
```

---

### **3. Hybrid Pipeline (Optimal)**

```bash
# Best of both worlds
Draft: Local AI (free)
Refine: Cloud AI (minimal cost)
Total: 80% savings vs all-cloud
```

**Example:**
```
User: "Brainstorm ideas then write report"
→ 🍄 ToadStool: Generate 10 ideas (free)
→ 🐦 Songbird: Route refinement to cloud
→ 🐿️ Squirrel: Polish best 3 ideas
→ Result: Quality output, minimal cost
```

---

## 🎨 Image Generation Pipeline

### **Full Workflow**

```
1. User: "Generate image of futuristic network"
   ↓
2. 🐦 Songbird: Route prompt to local AI
   ↓
3. 🍄 ToadStool: Enhance prompt with TinyLlama
   Output: "Futuristic network: neon blue circuits,
           holographic nodes, cyberpunk style, 4K"
   ↓
4. 🐦 Songbird: Route to image service
   ↓
5. 🐿️ Squirrel: Select Stable Diffusion (HuggingFace)
   ↓
6. Generate image (10-30 seconds)
   ↓
7. 💾 Save locally: ./generated_images/generated_*.png
```

**Cost:**
- Prompt enhancement: $0.00 (local)
- Image generation: $0.00 (HF free tier)
- **Total: $0.00**

---

## 📁 File Structure

```
showcase/real-world/06-ai-orchestration/
├── setup-local-ai.sh          # One-time setup
├── local-cloud-hybrid.sh      # Text generation demo
├── generate-image-demo.sh     # Image generation demo
├── generated_images/          # Output directory
│   └── generated_*.png        # Your images
├── LOCAL_AI_GUIDE.md          # This file
└── ... (other demos)
```

---

## 🔧 Troubleshooting

### **"Ollama not found"**

```bash
# Install manually
curl -fsSL https://ollama.com/install.sh | sh

# Or on macOS
brew install ollama
```

---

### **"Model not found"**

```bash
# Download models manually
ollama pull tinyllama
ollama pull llama3.2:1b
ollama pull llama3.2:3b
```

---

### **"Connection refused"**

```bash
# Start Ollama service
ollama serve

# Or in background
ollama serve > /tmp/ollama.log 2>&1 &
```

---

### **"Image generation fails"**

Possible reasons:
1. HuggingFace API rate limit (wait a minute)
2. Model loading (first time can take 1-2 minutes)
3. Network issue (check internet connection)

**Solution**: Run again after 1-2 minutes

---

## 🌟 Advanced: Multi-Tower Setup

### **Adding Tower B**

Once you have local AI working on Tower A, add Tower B:

```bash
# On Tower B
cd ecoPrimals/toadstool/showcase/real-world/06-ai-orchestration
./setup-local-ai.sh

# Download different model for variety
ollama pull llama3.2:3b
```

**Songbird will then route:**
- Simple tasks → Tower A (TinyLlama, faster)
- Complex tasks → Tower B (Llama 3.2 3B, smarter)
- Cloud tasks → Squirrel (OpenAI, most powerful)

---

## 💰 Cost Comparison

### **Monthly Usage: 10,000 requests**

**All Cloud:**
```
10,000 requests × $0.03 = $300/month
```

**All Local:**
```
10,000 requests × $0.00 = $0/month
(but limited capability)
```

**Hybrid (ToadStool):**
```
8,000 local (simple) × $0.00 = $0
2,000 cloud (complex) × $0.03 = $60
Total: $60/month (80% savings!)
```

---

## 🎉 Summary

**You now have:**

✅ **Local AI** running on ToadStool compute
- Real models (TinyLlama, Llama 3.2)
- 100% private
- Zero cost
- Fast response

✅ **Cloud AI** via Squirrel gateway
- OpenAI GPT-3.5/4
- High quality
- Pay per use

✅ **Hybrid orchestration**
- Best of both worlds
- 80% cost savings
- Automatic routing

✅ **Image generation**
- Stable Diffusion
- Local prompt enhancement
- Free tier access

✅ **Production ready**
- Real models, real responses
- Mesh-ready architecture
- Add towers easily

---

## 🚀 Next Steps

1. **Run the demos** (all 3 scripts)
2. **Try your own prompts** (modify scripts)
3. **Add more models** (`ollama pull <model>`)
4. **Deploy to Tower B** (multi-tower mesh)
5. **Monitor costs** (track cloud API usage)

---

**🌿 Welcome to distributed AI orchestration!**

*Last Updated: December 8, 2025*  
*Status: Production Ready ✅*

