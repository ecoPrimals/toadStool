#!/bin/bash
# Quick API Validation Test (Non-Interactive)
# Tests that API keys work with real calls

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

echo ""
echo "🔬 Testing API Keys..."
echo ""

# Load API keys
SECRETS_DIR="/home/eastgate/Development/ecoPrimals/testing-secrets"
ANTHROPIC_KEY=$(grep "anthropic_api_key" "$SECRETS_DIR/api-keys.toml" | cut -d'"' -f2)
OPENAI_KEY=$(grep "openai_api_key" "$SECRETS_DIR/api-keys.toml" | cut -d'"' -f2)

TIMESTAMP=$(date +%s)

# Test 1: OpenAI
echo -e "${BLUE}Testing OpenAI GPT-3.5-Turbo...${NC}"
OPENAI_RESPONSE=$(curl -s https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_KEY" \
  -d "{
    \"model\": \"gpt-3.5-turbo\",
    \"messages\": [{
      \"role\": \"user\",
      \"content\": \"Say: Validated at $TIMESTAMP\"
    }],
    \"max_tokens\": 50
  }" 2>&1)

if echo "$OPENAI_RESPONSE" | grep -q "$TIMESTAMP"; then
    echo -e "${GREEN}✅ OpenAI API Working!${NC}"
    OPENAI_TEXT=$(echo "$OPENAI_RESPONSE" | jq -r '.choices[0].message.content' 2>/dev/null)
    echo -e "   Response: ${CYAN}$OPENAI_TEXT${NC}"
    OPENAI_TOKENS=$(echo "$OPENAI_RESPONSE" | jq -r '.usage.total_tokens' 2>/dev/null || echo "?")
    echo -e "   Tokens: ${CYAN}$OPENAI_TOKENS${NC}"
elif echo "$OPENAI_RESPONSE" | grep -q "error"; then
    echo -e "${RED}❌ OpenAI Error:${NC}"
    echo "$OPENAI_RESPONSE" | jq '.error.message' 2>/dev/null || echo "$OPENAI_RESPONSE"
else
    echo -e "${YELLOW}⚠️  OpenAI response unexpected${NC}"
    echo "$OPENAI_RESPONSE" | head -5
fi

echo ""

# Test 2: Anthropic Claude
echo -e "${BLUE}Testing Anthropic Claude...${NC}"
CLAUDE_RESPONSE=$(curl -s https://api.anthropic.com/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: $ANTHROPIC_KEY" \
  -H "anthropic-version: 2023-06-01" \
  -d "{
    \"model\": \"claude-3-haiku-20240307\",
    \"messages\": [{
      \"role\": \"user\",
      \"content\": \"Say: Validated at $TIMESTAMP\"
    }],
    \"max_tokens\": 50
  }" 2>&1)

if echo "$CLAUDE_RESPONSE" | grep -q "$TIMESTAMP"; then
    echo -e "${GREEN}✅ Claude API Working!${NC}"
    CLAUDE_TEXT=$(echo "$CLAUDE_RESPONSE" | jq -r '.content[0].text' 2>/dev/null)
    echo -e "   Response: ${CYAN}$CLAUDE_TEXT${NC}"
    CLAUDE_TOKENS=$(echo "$CLAUDE_RESPONSE" | jq -r '.usage.input_tokens + .usage.output_tokens' 2>/dev/null || echo "?")
    echo -e "   Tokens: ${CYAN}$CLAUDE_TOKENS${NC}"
elif echo "$CLAUDE_RESPONSE" | grep -q "error"; then
    echo -e "${RED}❌ Claude Error:${NC}"
    echo "$CLAUDE_RESPONSE" | jq '.error.message' 2>/dev/null || echo "$CLAUDE_RESPONSE"
else
    echo -e "${YELLOW}⚠️  Claude response unexpected${NC}"
    echo "$CLAUDE_RESPONSE" | head -5
fi

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

echo -e "${GREEN}${BOLD}API Validation Complete!${NC}"
echo ""
echo -e "${YELLOW}Validation Hash:${NC} $(echo "test-$TIMESTAMP" | sha256sum | cut -d' ' -f1 | head -c 16)"
echo ""

exit 0

