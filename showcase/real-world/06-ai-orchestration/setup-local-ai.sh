#!/bin/bash
# Setup Local AI on ToadStool Compute
# Uses Ollama for easy local model management

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                                                               ║"
echo "║   🍄 ToadStool Local AI Setup                                ║"
echo "║   Pull models for local compute                              ║"
echo "║                                                               ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

# Check if Ollama is installed
if command -v ollama &> /dev/null; then
    echo -e "${GREEN}✅ Ollama is installed${NC}"
    OLLAMA_VERSION=$(ollama --version 2>&1 | head -1)
    echo "   Version: $OLLAMA_VERSION"
else
    echo -e "${YELLOW}📦 Ollama not found. Installing...${NC}"
    echo ""
    echo "Choose installation method:"
    echo "  1) Auto-install (curl script)"
    echo "  2) Manual instructions"
    echo "  3) Skip (I'll install later)"
    echo ""
    read -p "Choice (1-3): " CHOICE
    
    case $CHOICE in
        1)
            echo ""
            echo -e "${BLUE}Installing Ollama...${NC}"
            curl -fsSL https://ollama.com/install.sh | sh
            echo -e "${GREEN}✅ Ollama installed${NC}"
            ;;
        2)
            echo ""
            echo -e "${CYAN}Manual Installation:${NC}"
            echo ""
            echo "  Linux:"
            echo "    curl -fsSL https://ollama.com/install.sh | sh"
            echo ""
            echo "  macOS:"
            echo "    brew install ollama"
            echo ""
            echo "  Or download from: https://ollama.com/download"
            echo ""
            exit 0
            ;;
        3)
            echo ""
            echo -e "${YELLOW}Skipping installation. Run this script again after installing Ollama.${NC}"
            exit 0
            ;;
    esac
fi

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Local AI Models for ToadStool Compute${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Start Ollama service if not running
if ! pgrep -x "ollama" > /dev/null; then
    echo -e "${YELLOW}Starting Ollama service...${NC}"
    ollama serve > /tmp/ollama.log 2>&1 &
    sleep 3
    echo -e "${GREEN}✅ Ollama service started${NC}"
else
    echo -e "${GREEN}✅ Ollama service already running${NC}"
fi

echo ""
echo -e "${CYAN}Available models:${NC}"
echo ""
echo "  1. ${BOLD}TinyLlama 1.1B${NC} (1.1GB) - Fast, CPU-friendly"
echo "     Perfect for demos, code completion, simple tasks"
echo ""
echo "  2. ${BOLD}Llama 3.2 3B${NC} (~2GB) - Balanced"  
echo "     Good quality, reasonable speed"
echo ""
echo "  3. ${BOLD}Llama 3.2 1B${NC} (~1GB) - Ultra-fast"
echo "     Smallest Llama 3, great for local compute"
echo ""
echo "  4. ${BOLD}Phi-3 Mini${NC} (~2GB) - Microsoft model"
echo "     Excellent for reasoning tasks"
echo ""
echo "  5. ${BOLD}All models${NC} - Download all (good for showcase)"
echo ""

read -p "Select model(s) to download (1-5): " MODEL_CHOICE

case $MODEL_CHOICE in
    1)
        echo ""
        echo -e "${BLUE}📥 Pulling TinyLlama 1.1B...${NC}"
        ollama pull tinyllama
        echo -e "${GREEN}✅ TinyLlama ready${NC}"
        ;;
    2)
        echo ""
        echo -e "${BLUE}📥 Pulling Llama 3.2 3B...${NC}"
        ollama pull llama3.2:3b
        echo -e "${GREEN}✅ Llama 3.2 3B ready${NC}"
        ;;
    3)
        echo ""
        echo -e "${BLUE}📥 Pulling Llama 3.2 1B...${NC}"
        ollama pull llama3.2:1b
        echo -e "${GREEN}✅ Llama 3.2 1B ready${NC}"
        ;;
    4)
        echo ""
        echo -e "${BLUE}📥 Pulling Phi-3 Mini...${NC}"
        ollama pull phi3
        echo -e "${GREEN}✅ Phi-3 Mini ready${NC}"
        ;;
    5)
        echo ""
        echo -e "${BLUE}📥 Pulling all models (this will take a few minutes)...${NC}"
        ollama pull tinyllama
        ollama pull llama3.2:1b
        ollama pull llama3.2:3b
        ollama pull phi3
        echo -e "${GREEN}✅ All models ready${NC}"
        ;;
esac

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Testing Local AI${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Test the model
echo -e "${YELLOW}🧪 Testing local AI...${NC}"
echo ""

TEST_RESPONSE=$(curl -s http://localhost:11434/api/generate -d '{
  "model": "tinyllama",
  "prompt": "Say hello from ToadStool local AI in one sentence.",
  "stream": false
}' 2>&1)

if echo "$TEST_RESPONSE" | grep -q "response"; then
    echo -e "${GREEN}✅ Local AI is working!${NC}"
    echo ""
    echo -e "${CYAN}Test Response:${NC}"
    echo "$TEST_RESPONSE" | jq -r '.response' 2>/dev/null
else
    echo -e "${RED}❌ Test failed${NC}"
    echo "$TEST_RESPONSE"
fi

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅ Local AI Setup Complete!${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${CYAN}What's Available:${NC}"
echo ""
echo "  🍄 ToadStool Compute: Local AI models running"
echo "  🔗 Endpoint: http://localhost:11434"
echo "  📦 Models installed:"
ollama list 2>/dev/null || echo "     (Check with: ollama list)"
echo ""

echo -e "${YELLOW}Next Steps:${NC}"
echo ""
echo "  1. Run the hybrid demo:"
echo "     ${CYAN}./local-cloud-hybrid.sh${NC}"
echo ""
echo "  2. Test local AI:"
echo "     ${CYAN}curl http://localhost:11434/api/generate -d '{\"model\":\"tinyllama\",\"prompt\":\"Hello\",\"stream\":false}'${NC}"
echo ""
echo "  3. Generate images:"
echo "     ${CYAN}./generate-image-demo.sh${NC}"
echo ""

echo -e "${GREEN}🎉 Ready for local + cloud AI orchestration!${NC}"
echo ""

exit 0

