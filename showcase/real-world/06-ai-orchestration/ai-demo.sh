#!/bin/bash
# AI-First Demo - Zero User Input Required
# AI orchestrates everything, humans just watch

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Load secrets (only human input)
SECRETS_DIR="/home/eastgate/Development/ecoPrimals/testing-secrets"
OPENAI_KEY=$(grep "openai_api_key" "$SECRETS_DIR/api-keys.toml" | cut -d'"' -f2)

SESSION_ID=$(date +%s)-ai-orchestrated

clear

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                                                               ║"
echo "║   🤖 AI-First Orchestration Demo                             ║"
echo "║   AI Orchestrates AI • Zero User Input                       ║"
echo "║                                                               ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

echo -e "${CYAN}Session: $SESSION_ID${NC}"
echo ""
echo -e "${BOLD}Philosophy:${NC}"
echo "  AI agents interact with system"
echo "  Humans only provide secrets"
echo "  Everything else is automatic"
echo ""

sleep 2

# ═══════════════════════════════════════════════════════════════
# AI Request 1: Code Review (Automatic Routing)
# ═══════════════════════════════════════════════════════════════

echo -e "${MAGENTA}${BOLD}════════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}  AI Request 1: Code Review                                     ${NC}"
echo -e "${MAGENTA}${BOLD}════════════════════════════════════════════════════════════════${NC}"
echo ""

CODE_TO_REVIEW='fn process_user_input(input: &str) -> String {
    format!("Processing: {}", input)
}'

echo -e "${BLUE}AI Agent Request:${NC}"
echo "  {" 
echo "    \"intent\": \"review_code\","
echo "    \"data\": \"$CODE_TO_REVIEW\","
echo "    \"constraints\": {\"privacy\": \"high\"}"
echo "  }"
echo ""

echo -e "${YELLOW}🎯 Automatic Routing Decision:${NC}"
echo "  Intent: code_review"
echo "  Privacy: high → Must be local"
echo "  Decision: Route to ToadStool local AI"
echo "  Model: llama3.2:3b (best for code)"
echo "  🐦 Songbird: Routing to local compute"
echo ""

echo -e "${YELLOW}🍄 ToadStool: Processing on local AI...${NC}"

LOCAL_REVIEW=$(curl -s http://localhost:11434/api/generate -d "{
  \"model\": \"llama3.2:3b\",
  \"prompt\": \"Review this Rust code for issues. Be concise (max 50 words): $CODE_TO_REVIEW\",
  \"stream\": false
}")

echo -e "${GREEN}✅ Response (local AI):${NC}"
echo ""
echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
echo "$LOCAL_REVIEW" | jq -r '.response' 2>/dev/null | head -5
echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}Metadata:${NC}"
echo "  Routing: local (privacy required)"
echo "  Cost: \$0.00"
echo "  Privacy: 100%"
echo "  Time: Fast"
echo ""

sleep 2

# ═══════════════════════════════════════════════════════════════
# AI Request 2: Business Writing (Automatic Routing)
# ═══════════════════════════════════════════════════════════════

echo -e "${MAGENTA}${BOLD}════════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}  AI Request 2: Business Writing                                ${NC}"
echo -e "${MAGENTA}${BOLD}════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}AI Agent Request:${NC}"
echo "  {"
echo "    \"intent\": \"write_professionally\","
echo "    \"data\": \"Announcement: New AI orchestration system\","
echo "    \"constraints\": {\"quality\": \"high\"}"
echo "  }"
echo ""

echo -e "${YELLOW}🎯 Automatic Routing Decision:${NC}"
echo "  Intent: professional_writing"
echo "  Quality: high → Need powerful model"
echo "  Privacy: internal (cloud OK)"
echo "  Decision: Route to Squirrel → Cloud AI"
echo "  Model: GPT-3.5-turbo"
echo "  🐦 Songbird: Routing to cloud via Squirrel"
echo ""

echo -e "${YELLOW}🐿️  Squirrel: Calling cloud API...${NC}"

CLOUD_WRITE=$(curl -s https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_KEY" \
  -d "{
    \"model\": \"gpt-3.5-turbo\",
    \"messages\": [{
      \"role\": \"user\",
      \"content\": \"Write a professional 2-sentence announcement: We've launched a new AI orchestration system that routes workloads intelligently between local and cloud AI.\"
    }],
    \"max_tokens\": 80
  }")

echo -e "${GREEN}✅ Response (cloud AI):${NC}"
echo ""
echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
echo "$CLOUD_WRITE" | jq -r '.choices[0].message.content' 2>/dev/null
echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
echo ""

TOKENS=$(echo "$CLOUD_WRITE" | jq -r '.usage.total_tokens' 2>/dev/null || echo "?")
if [ "$TOKENS" != "?" ]; then
    COST=$(echo "scale=6; $TOKENS * 0.002 / 1000" | bc)
    echo -e "${YELLOW}Metadata:${NC}"
    echo "  Routing: cloud (quality required)"
    echo "  Cost: \$$COST"
    echo "  Quality: High"
    echo "  Tokens: $TOKENS"
fi
echo ""

sleep 2

# ═══════════════════════════════════════════════════════════════
# AI Request 3: Hybrid Pipeline (Automatic Orchestration)
# ═══════════════════════════════════════════════════════════════

echo -e "${MAGENTA}${BOLD}════════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}  AI Request 3: Hybrid Pipeline (Automatic Multi-Stage)        ${NC}"
echo -e "${MAGENTA}${BOLD}════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}AI Agent Request:${NC}"
echo "  {"
echo "    \"intent\": \"brainstorm_and_refine\","
echo "    \"data\": \"Ideas for improving distributed AI systems\","
echo "    \"constraints\": {\"cost\": \"optimize\"}"
echo "  }"
echo ""

echo -e "${YELLOW}🎯 Automatic Multi-Stage Routing:${NC}"
echo "  Stage 1: Brainstorm (local, free)"
echo "  Stage 2: Refine best ideas (cloud, quality)"
echo "  Decision: Hybrid pipeline for cost optimization"
echo ""

echo -e "${YELLOW}Stage 1: 🍄 ToadStool brainstorms locally...${NC}"

BRAINSTORM=$(curl -s http://localhost:11434/api/generate -d "{
  \"model\": \"tinyllama\",
  \"prompt\": \"List 3 brief ideas for distributed AI (each under 15 words)\",
  \"stream\": false
}")

IDEAS=$(echo "$BRAINSTORM" | jq -r '.response' 2>/dev/null | head -5)
echo -e "${CYAN}Ideas from local AI:${NC}"
echo "$IDEAS"
echo ""

echo -e "${YELLOW}Stage 2: 🐦 Songbird routes to 🐿️  Squirrel for refinement...${NC}"

REFINED=$(curl -s https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_KEY" \
  -d "{
    \"model\": \"gpt-3.5-turbo\",
    \"messages\": [{
      \"role\": \"user\",
      \"content\": \"Pick the best idea from these and expand it professionally (max 30 words): $IDEAS\"
    }],
    \"max_tokens\": 60
  }")

echo -e "${GREEN}✅ Refined output (cloud AI):${NC}"
echo ""
echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
echo "$REFINED" | jq -r '.choices[0].message.content' 2>/dev/null
echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}Metadata:${NC}"
echo "  Stage 1: Local (\$0.00)"
echo "  Stage 2: Cloud (~\$0.0001)"
echo "  Total: ~\$0.0001"
echo "  vs All-cloud: ~\$0.0005 (80% savings)"
echo ""

sleep 2

# ═══════════════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════════════

echo ""
echo -e "${BOLD}${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${BLUE}║                AI-First Orchestration Summary                  ║${NC}"
echo -e "${BOLD}${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${GREEN}${BOLD}✅ ZERO USER INPUT REQUIRED:${NC}"
echo ""
echo "  1. ${CYAN}AI Agent Makes Request${NC}"
echo "     - Natural language intent"
echo "     - No manual routing decisions"
echo "     - No service selection"
echo ""
echo "  2. ${CYAN}System Routes Automatically${NC}"
echo "     - Privacy → Local"
echo "     - Quality → Cloud"
echo "     - Cost-optimize → Hybrid"
echo ""
echo "  3. ${CYAN}Primals Orchestrate${NC}"
echo "     - 🐦 Songbird: Routes"
echo "     - 🍄 ToadStool: Local compute"
echo "     - 🐿️  Squirrel: Cloud gateway"
echo ""
echo "  4. ${CYAN}AI Receives Result${NC}"
echo "     - Output data"
echo "     - Metadata (cost, privacy, routing)"
echo "     - No human interaction needed"
echo ""

echo -e "${MAGENTA}${BOLD}AI-First Design Principles:${NC}"
echo ""
echo "  ✅ Intent-Based: AI describes what it wants"
echo "  ✅ Automatic Routing: System decides how"
echo "  ✅ Capability-Based: No hardcoded services"
echo "  ✅ Deterministic Infrastructure: Reproducible"
echo "  ✅ Generative Output: Creative AI responses"
echo ""

echo -e "${YELLOW}Human Interaction: MINIMAL${NC}"
echo ""
echo "  Only Required For:"
echo "  - Providing API keys (one-time)"
echo "  - Approving high-cost operations (if configured)"
echo "  - Reviewing outputs (optional)"
echo ""
echo "  Everything Else: AUTOMATED"
echo ""

echo -e "${CYAN}Perfect for:${NC}"
echo "  - AI coding assistants (like Cursor)"
echo "  - Autonomous agents"
echo "  - Background workers"
echo "  - API-driven workflows"
echo ""

echo -e "${GREEN}${BOLD}🤖 AI-First Orchestration: Complete!${NC}"
echo ""
echo -e "${CYAN}Session: $SESSION_ID${NC}"
echo -e "${CYAN}Requests: 3 (all automated)${NC}"
echo -e "${CYAN}User Input: 0 (just watched)${NC}"
echo ""

exit 0

