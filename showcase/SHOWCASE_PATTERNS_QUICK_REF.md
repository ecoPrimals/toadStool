# 🚀 Showcase Patterns - Quick Reference

**Purpose**: Quick lookup for showcase patterns across ecoPrimals  
**Use**: Copy patterns that work, learn from successful demos

---

## 📁 Directory Structure Patterns

### ⭐ Best: Progressive 4-Level Structure

```
showcase/
├── 00_START_HERE.md           # Entry point (5-min quick start)
├── QUICK_START.md             # Fast onboarding
├── MASTER_INDEX.md            # Complete reference
│
├── 01-standalone/             # Level 0: Local capabilities
│   ├── README.md
│   ├── 01-basic-features/
│   ├── 02-advanced-features/
│   └── 03-performance/
│
├── 02-integration/            # Level 1: One-way interaction
│   ├── README.md
│   ├── 01-primal-a/
│   └── 02-primal-b/
│
├── 03-federation/             # Level 2: Multi-node
│   ├── README.md
│   ├── 01-mesh-formation/
│   └── 02-distributed/
│
├── 04-multi-primal/           # Level 3: Ecosystem
│   ├── README.md
│   ├── 01-two-primal/
│   └── 02-complete-pipeline/
│
├── 05-real-world/             # Level 4: Production scenarios
│   └── ...
│
├── scripts/                   # Utilities
└── outputs/                   # Demo outputs (gitignored)
```

**Used by**: NestGate (best example), ToadStool  
**Grade**: A+

---

## 🎬 Demo Script Patterns

### ⭐ Best: Safe, Visual, Graceful Degradation

```bash
#!/bin/bash

# ===================================================================
# [PRIMAL] Demo: [NAME]
# ===================================================================
# What this demonstrates: [clear bullet points]
# Prerequisites: [list with fallbacks]
# ===================================================================

set -e

# Colors for readability
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration with fallbacks
DEMO_MODE=true  # Always works
SERVICE_ENDPOINT="${SERVICE_URL:-http://localhost:8080}"

echo ""
echo "=================================================================="
echo "  [DEMO TITLE]"
echo "=================================================================="
echo ""

# Step 1: Prerequisites check
echo "Step 1: Checking prerequisites..."
if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO MODE] Simulating operations...${NC}"
else
    # Check real service
    if ! curl -s "$SERVICE_ENDPOINT/health" > /dev/null 2>&1; then
        echo "   ⚠️  Service not available, switching to demo mode"
        DEMO_MODE=true
    fi
fi

# Step 2-N: Main demo steps
echo "Step 2: [Main operation]..."
if [ "$DEMO_MODE" = true ]; then
    # Simulated operation
    echo -e "${YELLOW}   [DEMO] Simulating...${NC}"
    sleep 0.5
else
    # Real operation
    RESPONSE=$(curl -s "$SERVICE_ENDPOINT/api/...")
fi

# Always include: Visual flow diagram
echo ""
echo "   ┌──────────────────────────────────┐"
echo "   │         WORKFLOW FLOW            │"
echo "   └──────────────────────────────────┘"
echo ""
echo "   User → Service A → Service B → Results"

# Always include: Summary
echo ""
echo "=================================================================="
echo "  Demo Complete!"
echo "=================================================================="
echo ""
echo "What we demonstrated:"
echo "  ✅ [Achievement 1]"
echo "  ✅ [Achievement 2]"
echo ""
echo "Next steps:"
echo "  - Try: [related demo]"
echo "  - Learn: [related concept]"
echo ""
```

**Key Features**:
- ✅ Demo mode (always works)
- ✅ Color output (readability)
- ✅ Visual diagrams (understanding)
- ✅ Clear summary (learning)
- ✅ Graceful degradation (no dependencies)

**Used by**: ToadStool (Level 3), NestGate, Songbird  
**Grade**: A+

---

## 📚 README Patterns

### ⭐ Best: Clear, Progressive, Action-Oriented

```markdown
# [Level Name]: [Purpose]

**Goal**: [One-line goal]  
**Time**: [Estimated time]  
**Prerequisites**: [Simple list]

---

## 🎯 What You'll Learn

- [Learning outcome 1]
- [Learning outcome 2]
- [Learning outcome 3]

---

## 🚀 Quick Start

```bash
# Run all demos in this level
./run_all.sh

# Or run individually
cd 01-first-demo && ./demo.sh
```

---

## 📋 Demos in This Level

### Demo 1: [Name]
**Time**: 5 minutes  
**Shows**: [What it demonstrates]

```bash
cd 01-first-demo
./demo.sh
```

**What you'll see**:
- [Visual/output 1]
- [Visual/output 2]

---

### Demo 2: [Name]
[Same pattern]

---

## 💡 Key Concepts

- **[Concept 1]**: [Brief explanation]
- **[Concept 2]**: [Brief explanation]

---

## ➡️ Next Steps

After completing this level:
- **Next Level**: [Link to next]
- **Related**: [Related demos]
- **Deep Dive**: [Architecture docs]

---

*Updated: [Date]*
```

**Key Features**:
- ✅ Clear goals
- ✅ Time estimates
- ✅ Copy-paste commands
- ✅ Progressive structure
- ✅ Next steps

**Used by**: All primals  
**Grade**: A

---

## 🎯 Discovery Pattern (Capability-Based)

### ⭐ Best: Zero-Config, O(1) Complexity

```bash
# Pattern from ToadStool/NestGate demos

echo "Step 2: Discovering NestGate via capabilities..."

if [ "$DEMO_MODE" = true ]; then
    echo -e "${YELLOW}   [DEMO] Simulating discovery...${NC}"
    NESTGATE_URL="http://localhost:8080"
    NESTGATE_CAPABILITIES="persistent_storage,versioning,metadata"
else
    # Real discovery via registry
    DISCOVERY=$(curl -s "$REGISTRY_URL/api/v1/services?capability=persistent_storage")
    NESTGATE_URL=$(echo "$DISCOVERY" | jq -r '.[0].endpoint')
    NESTGATE_CAPABILITIES=$(echo "$DISCOVERY" | jq -r '.[0].capabilities[]' | tr '\n' ',')
fi

echo "   ✅ Discovered NestGate at: $NESTGATE_URL"
echo "   ✅ Capabilities: $NESTGATE_CAPABILITIES"
```

**Key Features**:
- ✅ No hardcoded endpoints
- ✅ Capability-based selection
- ✅ Automatic failover
- ✅ O(1) discovery time

**Used by**: ToadStool (Level 3), Squirrel, Songbird  
**Grade**: A+

---

## 🌐 Federation Pattern (Zero-Config Mesh)

### ⭐ Best: mDNS Discovery, Dynamic Joining

```bash
# Pattern from Songbird showcase

echo "Step 1: Starting tower with mDNS discovery..."

# Tower advertises itself
./bin/songbird --tower-name "$TOWER_NAME" \
               --discovery mdns \
               --capabilities "orchestration,coordination" \
               --auto-join

echo "Step 2: Waiting for mesh formation..."

# Automatic discovery and mesh formation
sleep 3

echo "Step 3: Querying mesh status..."
MESH_STATUS=$(curl -s http://localhost:8080/api/v1/mesh/status)
PEER_COUNT=$(echo "$MESH_STATUS" | jq '.peers | length')

echo "   ✅ Mesh formed with $PEER_COUNT peers"
echo "   ✅ Capabilities shared across mesh"
```

**Key Features**:
- ✅ Zero configuration
- ✅ mDNS/DNS-SD
- ✅ Automatic mesh formation
- ✅ Dynamic joining/leaving

**Used by**: Songbird (best example), planned for ToadStool/BearDog  
**Grade**: A+

---

## 🔒 Encryption Pattern (Zero-Knowledge)

### ⭐ Best: End-to-End, No Plaintext

```bash
# Pattern from BearDog showcase

echo "Step 1: Encrypting data with BearDog..."

# Encrypt locally
ENCRYPTED=$(beardog encrypt \
    --key-id "$KEY_ID" \
    --input "$DATA_FILE" \
    --output "$ENCRYPTED_FILE")

echo "Step 2: Storing encrypted data in NestGate..."

# NestGate never sees plaintext
curl -X POST "$NESTGATE_URL/api/v1/storage/store" \
    -H "Content-Type: application/octet-stream" \
    --data-binary "@$ENCRYPTED_FILE"

echo ""
echo "   ⚠️  NestGate CANNOT read this data"
echo "   ✅ NestGate CAN store and serve it"
echo "   ✅ Only holder of key can decrypt"
```

**Key Features**:
- ✅ Client-side encryption
- ✅ Zero-knowledge storage
- ✅ End-to-end security
- ✅ Clear security boundaries

**Used by**: BearDog + NestGate, ToadStool (Level 3)  
**Grade**: A+

---

## 🎨 Visual Diagram Patterns

### ⭐ Best: ASCII Art Flow Diagrams

```bash
# Pattern: Simple flow
echo "   User → Service A → Service B → Results"

# Pattern: Branching
echo "              User"
echo "               │"
echo "        ┌──────┼──────┐"
echo "        │      │      │"
echo "    Service   Service Service"
echo "       A        B       C"

# Pattern: Complete pipeline
echo "   ┌──────────────────────────────────────┐"
echo "   │      COMPLETE ML PIPELINE            │"
echo "   └──────────────────────────────────────┘"
echo ""
echo "                User"
echo "                 │"
echo "        1. Submit ML job"
echo "                 ↓"
echo "           🎵 Songbird"
echo "                 │"
echo "       ┌─────────┼─────────┐"
echo "       │         │         │"
echo "  2. Route   3. Encrypt  4. Store"
echo "       │         │         │"
echo "       ↓         ↓         ↓"
echo "  🍄 ToadStool 🐻 BearDog 🗄️ NestGate"

# Pattern: Zero-knowledge
echo "   Plaintext → 🐻 Encrypt → Ciphertext → 🗄️ Store"
echo "                    │"
echo "                    ↓"
echo "               Key stays here"
echo "              (never leaves)"
```

**Key Features**:
- ✅ Clear visual flow
- ✅ Emojis for primals
- ✅ Step numbers
- ✅ Direction arrows
- ✅ Boxes for sections

**Used by**: All primals (ToadStool Level 3 has best examples)  
**Grade**: A+

---

## 📊 Progress Tracking Pattern

### ⭐ Best: Master Index with Status

```markdown
# Master Showcase Index

## Progress Overview

| Level | Status | Completion | Demos |
|-------|--------|------------|-------|
| **Level 0: Standalone** | ✅ Complete | 100% | 3/3 |
| **Level 1: Integration** | 🚧 Building | 60% | 3/5 |
| **Level 2: Federation** | 📝 Planned | 0% | 0/4 |
| **Level 3: Multi-Primal** | ✅ Complete | 100% | 5/5 |

## Demo Status

### Level 0: Standalone
- ✅ 01-basic-storage (Complete)
- ✅ 02-metadata (Complete)
- ✅ 03-performance (Complete)

### Level 1: Integration
- ✅ 01-workload-results (Complete)
- 🚧 02-ml-checkpoints (In Progress)
- 📝 03-dataset-mgmt (Planned)
```

**Key Features**:
- ✅ Visual progress
- ✅ Status icons
- ✅ Completion percentages
- ✅ Demo counts
- ✅ Clear next steps

**Used by**: ToadStool, NestGate  
**Grade**: A+

---

## 🎯 Quick Reference: Which Pattern to Use

| Need | Use This Pattern | Find Example |
|------|------------------|--------------|
| **Directory structure** | 4-level progressive | NestGate showcase |
| **Demo script** | Safe + visual + degrading | ToadStool Level 3 |
| **README** | Clear + progressive | All primals |
| **Service discovery** | Capability-based | ToadStool + NestGate |
| **Federation** | mDNS zero-config | Songbird showcase |
| **Encryption** | End-to-end zero-knowledge | BearDog + NestGate |
| **Visual diagrams** | ASCII art flows | ToadStool Level 3 |
| **Progress tracking** | Master index | ToadStool, NestGate |

---

## 💡 Pattern Selection Guide

### For Standalone Demos (Level 0)
- Use: Safe demo script pattern
- Include: Visual diagrams
- Focus: What this primal does independently

### For Integration Demos (Level 1)
- Use: Capability-based discovery
- Include: Flow diagrams showing A → B
- Focus: How two primals work together

### For Federation Demos (Level 2)
- Use: mDNS discovery pattern
- Include: Multi-node diagrams
- Focus: How multiple instances coordinate

### For Multi-Primal Demos (Level 3)
- Use: All patterns combined
- Include: Complete pipeline diagrams
- Focus: Ecosystem value proposition

---

## 📚 Best Examples by Primal

### 🎵 Songbird
**Best for**: Federation patterns, mesh formation, zero-config  
**See**: `showcase/02-federation/demos/01-mesh-formation.sh`

### 🗄️ NestGate
**Best for**: Progressive structure, live operations, safety  
**See**: `showcase/01_isolated/01_storage_basics/`

### 🐻 BearDog
**Best for**: Hardware integration, receipts, genetic mixing  
**See**: `showcase/02-hardware-integration/demo-genetic-realistic.sh`

### 🍄 ToadStool
**Best for**: Multi-primal workflows, visual diagrams, demos that work  
**See**: `showcase/multi-primal-nestgate/03-coordinated-compute/`

### 🐿️ Squirrel
**Best for**: AI routing, cost optimization, multi-provider  
**See**: `showcase/demos/03-multi-provider/demo-smart-routing.sh`

---

## ✅ Checklist for New Demo

Use this checklist when creating a new demo:

- [ ] Demo script uses safe pattern (demo mode, colors, etc.)
- [ ] README explains goal, time, prerequisites
- [ ] Visual flow diagram included
- [ ] Capability-based discovery (if multi-primal)
- [ ] Graceful degradation (works without services)
- [ ] Clear summary at end
- [ ] Next steps suggested
- [ ] Master index updated
- [ ] Tested in demo mode
- [ ] Tested with real services (if available)

---

## 🚀 Copy-Paste Templates

### New Demo Script

```bash
curl -s https://raw.githubusercontent.com/.../demo-template.sh > my-new-demo.sh
chmod +x my-new-demo.sh
```

### New README

```bash
cat > README.md << 'EOF'
# [Demo Name]

**Goal**: [Clear goal]
**Time**: [X minutes]

## Quick Start

```bash
./demo.sh
```

## What This Shows
- [Point 1]
- [Point 2]

## Next Steps
- [Next demo]
EOF
```

---

## 🎓 Learning Progression

1. **Study**: Read showcase READMEs across primals
2. **Copy**: Use patterns that work
3. **Adapt**: Modify for your specific demo
4. **Test**: Run in demo mode first
5. **Enhance**: Add real service integration
6. **Document**: Update master index
7. **Share**: Show others what you built!

---

**Questions?** Check the full review: [ECOSYSTEM_SHOWCASE_REVIEW_DEC_21_2025.md](./ECOSYSTEM_SHOWCASE_REVIEW_DEC_21_2025.md)

**Ready to build?** Use the patterns above and follow: [NEXT_SHOWCASE_ACTIONS_DEC_21_2025.md](./NEXT_SHOWCASE_ACTIONS_DEC_21_2025.md)

---

*Quick Reference - December 21, 2025*  
*Patterns from: Songbird, NestGate, BearDog, ToadStool, Squirrel*

