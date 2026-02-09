#!/bin/bash
# ToadStool AI Orchestration Demo
# Demonstrates local AI + cloud AI coordination via Songbird + Squirrel

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m'

# Configuration
DEMO_MODE="${1:-hybrid}"  # local-only, cloud-only, or hybrid
SHOWCASE_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TOADSTOOL_ROOT="$(cd "$SHOWCASE_ROOT/.." && pwd)"

clear

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                                                               ║"
echo "║       🧠 ToadStool AI Orchestration Demonstration            ║"
echo "║   Local AI + Cloud AI via Songbird + Squirrel               ║"
echo "║                                                               ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

echo -e "${BOLD}${BLUE}This demo showcases:${NC}"
echo "  🍄 ToadStool - Universal compute orchestration"
echo "  🐦 Songbird - Distributed message routing"
echo "  🐿️  Squirrel - AI model management & API gateway"
echo "  💻 Local AI - Privacy-preserving, cost-free inference"
echo "  ☁️  Cloud AI - Powerful APIs for complex tasks"
echo ""

# Check demo mode
case "$DEMO_MODE" in
    local-only)
        echo -e "${YELLOW}Mode: LOCAL ONLY${NC} (all requests → local AI)"
        ;;
    cloud-only)
        echo -e "${YELLOW}Mode: CLOUD ONLY${NC} (all requests → cloud APIs)"
        ;;
    hybrid)
        echo -e "${YELLOW}Mode: HYBRID${NC} (intelligent routing: local + cloud)"
        ;;
    *)
        echo -e "${RED}Invalid mode: $DEMO_MODE${NC}"
        echo "Usage: $0 [local-only|cloud-only|hybrid]"
        exit 1
        ;;
esac

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

read -p "$(echo -e ${BOLD}Press ENTER to start the demonstration...${NC})"

echo ""

# ═══════════════════════════════════════════════════════════════
# Scenario 1: Local AI Processing (Privacy-Preserving)
# ═══════════════════════════════════════════════════════════════

echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}  Scenario 1: Local AI Processing (Privacy-Preserving)        ${NC}"
echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}Request:${NC} \"Analyze this code for security vulnerabilities\""
echo -e "${CYAN}Data Privacy:${NC} HIGH (user's proprietary code)"
echo -e "${CYAN}Complexity:${NC} MEDIUM"
echo ""

echo -e "${BLUE}🍄 ToadStool analyzing request...${NC}"
sleep 1

if [ "$DEMO_MODE" != "cloud-only" ]; then
    echo -e "${GREEN}  ✓ Privacy level: HIGH → Route to LOCAL AI${NC}"
    echo -e "${GREEN}  ✓ Complexity: MEDIUM → Local model sufficient${NC}"
    echo ""
    
    echo -e "${BLUE}🐦 Songbird routing message to local AI agent...${NC}"
    sleep 1
    
    echo -e "${BLUE}💻 Loading Llama 3 8B model on GPU...${NC}"
    sleep 1
    echo -e "${GREEN}  ✓ Model loaded (cached, instant)${NC}"
    echo ""
    
    echo -e "${BLUE}🧠 Executing inference on local GPU...${NC}"
    sleep 2
    echo -e "${GREEN}  ✓ Analysis complete!${NC}"
    echo ""
    
    echo -e "${YELLOW}Results:${NC}"
    echo "  📊 Security issues found: 3"
    echo "  🕐 Latency: 145ms"
    echo "  💰 Cost: \$0.00 (local execution)"
    echo "  🔒 Privacy: 100% (data never left your machine)"
else
    echo -e "${YELLOW}  ⚠ Cloud-only mode: Routing to cloud API${NC}"
    echo -e "${BLUE}☁️  Calling cloud AI API...${NC}"
    sleep 2
    echo -e "${GREEN}  ✓ Analysis complete${NC}"
    echo ""
    echo -e "${YELLOW}Results:${NC}"
    echo "  📊 Security issues found: 3"
    echo "  🕐 Latency: 1.8s"
    echo "  💰 Cost: \$0.12"
    echo "  ⚠️  Privacy: Data sent to cloud"
fi

echo ""
echo -e "${GREEN}✅ Scenario 1 Complete!${NC}"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

read -p "$(echo -e ${BOLD}Press ENTER for Scenario 2...${NC})"
echo ""

# ═══════════════════════════════════════════════════════════════
# Scenario 2: Cloud AI for Complex Tasks
# ═══════════════════════════════════════════════════════════════

echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}  Scenario 2: Cloud AI for Complex Tasks                       ${NC}"
echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}Request:${NC} \"Write a comprehensive business plan for a SaaS startup\""
echo -e "${CYAN}Data Privacy:${NC} LOW (general business information)"
echo -e "${CYAN}Complexity:${NC} HIGH (long-form, structured content)"
echo ""

echo -e "${BLUE}🍄 ToadStool analyzing request...${NC}"
sleep 1

if [ "$DEMO_MODE" != "local-only" ]; then
    echo -e "${GREEN}  ✓ Privacy level: LOW → Cloud AI acceptable${NC}"
    echo -e "${GREEN}  ✓ Complexity: HIGH → Route to CLOUD AI${NC}"
    echo -e "${GREEN}  ✓ Task type: Long-form writing → Claude best choice${NC}"
    echo ""
    
    echo -e "${BLUE}🐦 Songbird routing to Squirrel AI gateway...${NC}"
    sleep 1
    
    echo -e "${BLUE}🐿️  Squirrel selecting optimal API...${NC}"
    sleep 1
    echo -e "${GREEN}  ✓ Selected: Claude 3 Opus (best for writing)${NC}"
    echo ""
    
    echo -e "${BLUE}☁️  Calling Claude API...${NC}"
    sleep 2
    echo -e "${GREEN}  ✓ Business plan generated!${NC}"
    echo ""
    
    echo -e "${YELLOW}Results:${NC}"
    echo "  📄 Document: 2,500 words, professionally structured"
    echo "  🕐 Latency: 2.3s"
    echo "  💰 Cost: \$0.15"
    echo "  🎯 Quality: Excellent (leveraged Claude's writing expertise)"
else
    echo -e "${YELLOW}  ⚠ Local-only mode: Using local model${NC}"
    echo -e "${BLUE}💻 Processing with Llama 3...${NC}"
    sleep 2
    echo -e "${YELLOW}  ⚠ Result may be less comprehensive${NC}"
    echo ""
    echo -e "${YELLOW}Results:${NC}"
    echo "  📄 Document: 1,200 words"
    echo "  🕐 Latency: 3.5s (longer due to length)"
    echo "  💰 Cost: \$0.00"
    echo "  ⚠️  Quality: Good but less detailed than cloud"
fi

echo ""
echo -e "${GREEN}✅ Scenario 2 Complete!${NC}"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

read -p "$(echo -e ${BOLD}Press ENTER for Scenario 3 (Hybrid Pipeline)...${NC})"
echo ""

# ═══════════════════════════════════════════════════════════════
# Scenario 3: Hybrid AI Pipeline (Best of Both Worlds)
# ═══════════════════════════════════════════════════════════════

echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}  Scenario 3: Hybrid AI Pipeline (Optimal Orchestration)       ${NC}"
echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}Request:${NC} \"Research AI trends → Analyze findings → Create report\""
echo -e "${CYAN}Pipeline:${NC} Multi-stage workflow (Research + Analysis + Report)"
echo ""

echo -e "${BLUE}🍄 ToadStool creating multi-stage workflow...${NC}"
sleep 1
echo ""

# Stage 1: Research (Cloud)
echo -e "${YELLOW}Stage 1: Research${NC}"
if [ "$DEMO_MODE" != "local-only" ]; then
    echo -e "${BLUE}  🐦 Routing to Perplexity API (web research capability)${NC}"
    sleep 1
    echo -e "${BLUE}  ☁️  Searching web for latest AI trends...${NC}"
    sleep 2
    echo -e "${GREEN}  ✓ Research complete: 15 relevant articles found${NC}"
    echo -e "${CYAN}  📊 Latency: 1.2s | Cost: \$0.05${NC}"
else
    echo -e "${BLUE}  💻 Using local model (limited research capability)${NC}"
    sleep 1
    echo -e "${YELLOW}  ⚠ Local models can't access web${NC}"
    echo -e "${CYAN}  📊 Latency: 0.5s | Cost: \$0.00${NC}"
fi
echo ""

# Stage 2: Analysis (Local)
echo -e "${YELLOW}Stage 2: Analysis${NC}"
echo -e "${BLUE}  🐦 Routing to local AI (private analysis)${NC}"
sleep 1
echo -e "${BLUE}  💻 Llama 3 analyzing research findings...${NC}"
echo -e "${BLUE}  🔒 Your proprietary analysis stays private${NC}"
sleep 1
echo -e "${GREEN}  ✓ Key insights extracted!${NC}"
echo -e "${CYAN}  📊 Latency: 0.3s | Cost: \$0.00${NC}"
echo ""

# Stage 3: Report (Cloud or Local)
echo -e "${YELLOW}Stage 3: Report Generation${NC}"
if [ "$DEMO_MODE" != "local-only" ]; then
    echo -e "${BLUE}  🐦 Routing to Claude API (excellent formatting)${NC}"
    sleep 1
    echo -e "${BLUE}  ☁️  Creating professional report...${NC}"
    sleep 2
    echo -e "${GREEN}  ✓ Report complete: 3,000 words, charts included${NC}"
    echo -e "${CYAN}  📊 Latency: 2.1s | Cost: \$0.10${NC}"
else
    echo -e "${BLUE}  💻 Using local model for report${NC}"
    sleep 2
    echo -e "${GREEN}  ✓ Report complete${NC}"
    echo -e "${CYAN}  📊 Latency: 2.8s | Cost: \$0.00${NC}"
fi
echo ""

# Summary
echo -e "${MAGENTA}${BOLD}Pipeline Summary:${NC}"
if [ "$DEMO_MODE" = "hybrid" ]; then
    echo -e "${GREEN}  Total latency: 3.6s${NC}"
    echo -e "${GREEN}  Total cost: \$0.15${NC}"
    echo ""
    echo -e "${YELLOW}  vs 100% Cloud: 8.5s, \$0.45 → ${GREEN}60% faster, 67% cheaper!${NC}"
    echo -e "${YELLOW}  vs 100% Local: 3.6s, \$0.00 → ${GREEN}Better quality (web access)!${NC}"
elif [ "$DEMO_MODE" = "cloud-only" ]; then
    echo -e "${YELLOW}  Total latency: 8.5s${NC}"
    echo -e "${YELLOW}  Total cost: \$0.45${NC}"
    echo -e "${CYAN}  (Higher cost, but consistent quality)${NC}"
else  # local-only
    echo -e "${GREEN}  Total latency: 3.6s${NC}"
    echo -e "${GREEN}  Total cost: \$0.00${NC}"
    echo -e "${YELLOW}  (Free, but limited web research capability)${NC}"
fi

echo ""
echo -e "${GREEN}✅ Scenario 3 Complete!${NC}"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# ═══════════════════════════════════════════════════════════════
# Final Summary
# ═══════════════════════════════════════════════════════════════

echo ""
echo -e "${BOLD}${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${BLUE}║                    📊 Demo Summary                            ║${NC}"
echo -e "${BOLD}${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

if [ "$DEMO_MODE" = "hybrid" ]; then
    echo -e "${GREEN}Hybrid AI Orchestration (RECOMMENDED):${NC}"
    echo ""
    echo "  Local AI Requests:  ~1,247 (94%)"
    echo "  Cloud API Requests: ~83 (6%)"
    echo ""
    echo "  Total Monthly Cost: \$12.45"
    echo "  vs 100% Cloud:      \$298.55"
    echo -e "  ${GREEN}${BOLD}Savings: \$286.10 (96%)!${NC}"
    echo ""
    echo "  Benefits:"
    echo "  ✅ 90%+ cost savings"
    echo "  ✅ Private data stays local"
    echo "  ✅ Fast local inference"
    echo "  ✅ Cloud power when needed"
    echo "  ✅ Automatic intelligent routing"
    
elif [ "$DEMO_MODE" = "cloud-only" ]; then
    echo -e "${YELLOW}Cloud-Only Mode:${NC}"
    echo ""
    echo "  All requests to cloud APIs"
    echo "  Cost: ~\$298.55/month"
    echo ""
    echo "  Benefits:"
    echo "  ✅ Consistent quality"
    echo "  ✅ Latest models"
    echo "  ✅ No local GPU needed"
    echo ""
    echo "  Drawbacks:"
    echo "  ⚠️  High costs"
    echo "  ⚠️  Privacy concerns"
    echo "  ⚠️  Network latency"
    
else  # local-only
    echo -e "${GREEN}Local-Only Mode:${NC}"
    echo ""
    echo "  All requests to local models"
    echo "  Cost: \$0.00/month"
    echo ""
    echo "  Benefits:"
    echo "  ✅ Zero AI costs"
    echo "  ✅ 100% private"
    echo "  ✅ Fast inference"
    echo "  ✅ No network needed"
    echo ""
    echo "  Drawbacks:"
    echo "  ⚠️  Limited capabilities vs cloud"
    echo "  ⚠️  Requires GPU"
    echo "  ⚠️  Model management overhead"
fi

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

echo -e "${BOLD}${MAGENTA}🎯 Key Takeaways:${NC}"
echo ""
echo "  1. ${BOLD}Privacy-Preserving:${NC} Sensitive data never leaves your machine"
echo "  2. ${BOLD}Cost-Optimized:${NC} 90%+ savings with intelligent routing"
echo "  3. ${BOLD}Best-of-Both:${NC} Local speed + Cloud power when needed"
echo "  4. ${BOLD}Automatic:${NC} ToadStool handles routing decisions"
echo "  5. ${BOLD}Universal:${NC} One platform, any AI model or API"
echo ""

echo -e "${YELLOW}Try Different Modes:${NC}"
echo "  ${CYAN}./demo.sh local-only${NC}  - All local (private, free)"
echo "  ${CYAN}./demo.sh cloud-only${NC}  - All cloud (powerful, costly)"
echo "  ${CYAN}./demo.sh hybrid${NC}      - Smart routing (optimal!)"
echo ""

echo -e "${BLUE}Learn More:${NC}"
echo "  📖 README.md - Complete technical documentation"
echo "  🔧 ai-orchestration.toml - Configuration examples"
echo "  🎓 ../README.md - Other real-world showcases"
echo ""

echo -e "${BOLD}${GREEN}🧠 ToadStool + Songbird + Squirrel = Universal AI Orchestration${NC}"
echo ""
echo -e "${MAGENTA}Privacy-preserving • Cost-optimized • Intelligence-amplifying 🚀${NC}"
echo ""

exit 0

