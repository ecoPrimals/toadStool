#!/bin/bash
# Capability-Based AI Router Demo
# Routes based on WHAT you need, not WHO provides it

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m'

clear

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                                                               ║"
echo "║   🎯 Capability-Based AI Routing                             ║"
echo "║   No Vendor Lock-In • Pure Capabilities                      ║"
echo "║                                                               ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

echo -e "${BOLD}${BLUE}Philosophy:${NC}"
echo "  Instead of: \"Use OpenAI for this, Claude for that\""
echo "  We say: \"I need text generation with these capabilities\""
echo ""
echo "  🎯 Services advertise capabilities"
echo "  🎯 Workloads specify requirements"
echo "  🎯 System matches automatically"
echo "  🎯 Vendors can be swapped without code changes"
echo ""

read -p "$(echo -e ${BOLD}Press ENTER to see capability matching...${NC})"
echo ""

# ═══════════════════════════════════════════════════════════════
# Scenario 1: Code Review (Privacy Required)
# ═══════════════════════════════════════════════════════════════

echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}  Scenario 1: Code Review                                      ${NC}"
echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Workload Requirements:${NC}"
echo "  capability: ai.text.generation"
echo "  min_tokens: 2000"
echo "  privacy_level: private"
echo "  max_cost: \$0.00 (must be free)"
echo ""

echo -e "${YELLOW}🔍 Querying Songbird registry for matching services...${NC}"
sleep 1

echo ""
echo -e "${CYAN}Available Services from Registry:${NC}"
echo ""
echo "  Service: text-generation-fast-001"
echo "    ✅ capability: ai.text.generation"
echo "    ✅ tokens: 4096"
echo "    ❌ privacy: cloud (requirement: on_premise)"
echo "    ❌ cost: \$0.0005/1K (requirement: \$0.00)"
echo "    Score: 40% (fails constraints)"
echo ""
echo "  Service: text-generation-powerful-001"
echo "    ✅ capability: ai.text.generation"
echo "    ✅ tokens: 200000"
echo "    ❌ privacy: cloud (requirement: on_premise)"
echo "    ❌ cost: \$0.003/1K (requirement: \$0.00)"
echo "    Score: 40% (fails constraints)"
echo ""
echo "  Service: text-generation-local-001"
echo "    ✅ capability: ai.text.generation"
echo "    ✅ tokens: 8192"
echo "    ✅ privacy: on_premise"
echo "    ✅ cost: \$0.00"
echo "    ✅ latency: 200ms"
echo "    Score: 100% ⭐"
echo ""

echo -e "${GREEN}✅ Selected: text-generation-local-001${NC}"
echo -e "${CYAN}   Endpoint resolved: http://localhost:11434${NC}"
echo -e "${CYAN}   Reason: Only service meeting privacy requirements${NC}"
echo ""

# ═══════════════════════════════════════════════════════════════
# Scenario 2: Business Document (Flexibility)
# ═══════════════════════════════════════════════════════════════

read -p "$(echo -e ${BOLD}Press ENTER for Scenario 2...${NC})"
echo ""

echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}  Scenario 2: Business Document Generation                     ${NC}"
echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${BLUE}Workload Requirements:${NC}"
echo "  capability: ai.text.generation"
echo "  min_tokens: 4000"
echo "  privacy_level: internal (cloud OK)"
echo "  max_cost: \$0.50"
echo "  preference: high quality, advanced reasoning"
echo ""

echo -e "${YELLOW}🔍 Querying registry with relaxed constraints...${NC}"
sleep 1

echo ""
echo -e "${CYAN}Capability Matching:${NC}"
echo ""
echo "  Service: text-generation-local-001"
echo "    ✅ capability: ai.text.generation"
echo "    ✅ tokens: 8192"
echo "    ✅ cost: \$0.00"
echo "    ⚠️  quality: good (preference: high)"
echo "    ⚠️  reasoning: basic (preference: advanced)"
echo "    Score: 70%"
echo ""
echo "  Service: text-generation-fast-001"
echo "    ✅ capability: ai.text.generation"
echo "    ✅ tokens: 4096"
echo "    ✅ cost: \$0.0015/1K (~\$0.03 for request)"
echo "    ⚠️  quality: good (preference: high)"
echo "    ❌ reasoning: none (preference: advanced)"
echo "    Score: 65%"
echo ""
echo "  Service: text-generation-powerful-001"
echo "    ✅ capability: ai.text.generation"
echo "    ✅ tokens: 200000"
echo "    ✅ cost: \$0.015/1K (~\$0.30 for request)"
echo "    ✅ quality: excellent"
echo "    ✅ reasoning: advanced"
echo "    Score: 95% ⭐"
echo ""

echo -e "${GREEN}✅ Selected: text-generation-powerful-001${NC}"
echo -e "${CYAN}   Endpoint resolved via Songbird${NC}"
echo -e "${CYAN}   Reason: Best capability match for requirements${NC}"
echo ""

# ═══════════════════════════════════════════════════════════════
# Scenario 3: New Service Added
# ═══════════════════════════════════════════════════════════════

read -p "$(echo -e ${BOLD}Press ENTER for Scenario 3...${NC})"
echo ""

echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}  Scenario 3: New Service Joins (No Code Changes!)            ${NC}"
echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}📡 New service registers with Songbird...${NC}"
sleep 1

echo ""
echo -e "${CYAN}Service Registration:${NC}"
echo "  {" 
echo "    \"service_id\": \"ultra-fast-ai-001\","
echo "    \"service_type\": \"ai.text.generation\","
echo "    \"capabilities\": {"
echo "      \"max_tokens\": 8192,"
echo "      \"streaming\": true,"
echo "      \"latency_ms\": 300,"
echo "      \"cost_per_1k\": 0.0002"
echo "    },"
echo "    \"endpoint\": \"https://new-provider.ai/v1/generate\""
echo "  }"
echo ""

echo -e "${GREEN}✅ Service registered with Songbird${NC}"
echo -e "${CYAN}   ToadStool auto-discovers via service query${NC}"
echo -e "${CYAN}   Squirrel adds to available backends${NC}"
echo -e "${CYAN}   Now available for routing!${NC}"
echo ""

sleep 1

echo -e "${BLUE}Next request for low-cost text generation:${NC}"
echo "  Workload: Simple summarization"
echo "  Requirements: cost < \$0.01, speed > 500ms"
echo ""

echo -e "${YELLOW}🔍 Re-querying registry...${NC}"
sleep 1

echo ""
echo -e "${CYAN}Now includes new service:${NC}"
echo ""
echo "  Service: ultra-fast-ai-001 ⭐ NEW!"
echo "    ✅ capability: ai.text.generation"
echo "    ✅ cost: \$0.0002/1K"
echo "    ✅ latency: 300ms"
echo "    Score: 92%"
echo ""

echo -e "${GREEN}✅ Selected: ultra-fast-ai-001${NC}"
echo -e "${CYAN}   Automatically discovered and selected!${NC}"
echo -e "${CYAN}   Zero code changes required!${NC}"
echo ""

# ═══════════════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════════════

read -p "$(echo -e ${BOLD}Press ENTER for summary...${NC})"
echo ""

echo -e "${BOLD}${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${BLUE}║                    Capability-Based Routing                   ║${NC}"
echo -e "${BOLD}${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${GREEN}${BOLD}🎯 Key Principles:${NC}"
echo ""
echo "  1. ${CYAN}Describe Requirements, Not Vendors${NC}"
echo "     \"I need text generation with privacy\""
echo "     NOT \"Use OpenAI GPT-4\""
echo ""
echo "  2. ${CYAN}Services Advertise Capabilities${NC}"
echo "     Services register with Songbird"
echo "     ToadStool queries capabilities"
echo "     No hardcoded vendor names"
echo ""
echo "  3. ${CYAN}Automatic Matching${NC}"
echo "     System scores each service"
echo "     Selects best match"
echo "     Falls back gracefully"
echo ""
echo "  4. ${CYAN}Zero Code Changes for New Services${NC}"
echo "     New service registers with Songbird"
echo "     System auto-discovers"
echo "     Immediately available"
echo ""

echo -e "${YELLOW}Benefits:${NC}"
echo ""
echo "  ✅ No vendor lock-in"
echo "  ✅ Services can be swapped"
echo "  ✅ New providers without code changes"
echo "  ✅ Best match automatically selected"
echo "  ✅ Cost/performance optimization"
echo "  ✅ Privacy constraints enforced"
echo ""

echo -e "${CYAN}Architecture:${NC}"
echo ""
echo "  Workload → ToadStool (orchestrator)"
echo "               ↓"
echo "          Query Songbird (\"what services can do X?\")"
echo "               ↓"
echo "          Score matches (capability + cost + privacy)"
echo "               ↓"
echo "          Route to best service"
echo "               ↓"
echo "          Squirrel executes"
echo "               ↓"
echo "          Result"
echo ""

echo -e "${MAGENTA}${BOLD}Example Adding New Service:${NC}"
echo ""
echo "  ${CYAN}# No code changes in ToadStool!${NC}"
echo "  ${CYAN}# Just register with Songbird:${NC}"
echo ""
echo "  curl -X POST http://localhost:8080/registry/register \\"
echo "    -d '{"
echo "      \"service_id\": \"my-ai-service\","
echo "      \"capabilities\": {\"text.generation\": true},"
echo "      \"endpoint\": \"https://my-ai.com/api\""
echo "    }'"
echo ""
echo "  ${GREEN}✅ Done! ToadStool will discover and use it.${NC}"
echo ""

echo -e "${GREEN}${BOLD}🎉 Capability-Based Routing: The Future!${NC}"
echo ""

exit 0

