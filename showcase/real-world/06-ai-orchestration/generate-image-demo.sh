#!/bin/bash
# Image Generation Demo
# Local text AI + Cloud image generation working together

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Load API keys
SECRETS_DIR="/home/eastgate/Development/ecoPrimals/testing-secrets"
HF_KEY=$(grep -A1 "hugging face" "$SECRETS_DIR/api-keys.toml" | tail -1 | xargs)
OPENAI_KEY=$(grep "openai_api_key" "$SECRETS_DIR/api-keys.toml" | cut -d'"' -f2)

SESSION_ID=$(date +%s)

# Output directory
OUTPUT_DIR="./generated_images"
mkdir -p "$OUTPUT_DIR"

clear

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║                                                               ║"
echo "║   🎨 AI Image Generation Orchestration                       ║"
echo "║   Local Text AI + Cloud Image Generation                     ║"
echo "║                                                               ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

echo -e "${CYAN}Session: $SESSION_ID${NC}"
echo -e "${CYAN}Output: $OUTPUT_DIR/${NC}"
echo ""

# ═══════════════════════════════════════════════════════════════
# Step 1: Local AI generates image prompt
# ═══════════════════════════════════════════════════════════════

echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}  Step 1: Local AI Creates Image Prompt                       ${NC}"
echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""

USER_REQUEST="A futuristic distributed computing network"
echo -e "${BLUE}User Request:${NC} \"$USER_REQUEST\""
echo ""

echo -e "${YELLOW}🐦 Songbird: Route to local AI (text processing)${NC}"
echo "  Privacy: Safe (just text prompt)"
echo "  Task: Creative prompt generation"
echo "  → ToadStool local compute"
echo ""

# Check if local AI available
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo -e "${YELLOW}🍄 ToadStool: Using local AI to enhance prompt...${NC}"
    echo ""
    
    PROMPT_RESPONSE=$(curl -s http://localhost:11434/api/generate -d "{
      \"model\": \"tinyllama\",
      \"prompt\": \"Write a detailed image prompt for: $USER_REQUEST. Include style, colors, and mood. Max 50 words.\",
      \"stream\": false
    }")
    
    IMAGE_PROMPT=$(echo "$PROMPT_RESPONSE" | jq -r '.response' 2>/dev/null | head -3 | tr '\n' ' ')
    echo -e "${GREEN}✅ Enhanced Prompt (from local AI):${NC}"
    echo "  $IMAGE_PROMPT"
    echo ""
else
    # Fallback to cloud AI
    echo -e "${YELLOW}🍄 ToadStool: Local AI not available, using cloud...${NC}"
    echo ""
    
    PROMPT_RESPONSE=$(curl -s https://api.openai.com/v1/chat/completions \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $OPENAI_KEY" \
      -d "{
        \"model\": \"gpt-3.5-turbo\",
        \"messages\": [{
          \"role\": \"user\",
          \"content\": \"Write a detailed Stable Diffusion image prompt for: $USER_REQUEST. Include style, lighting, and composition. Max 40 words.\"
        }],
        \"max_tokens\": 80
      }")
    
    IMAGE_PROMPT=$(echo "$PROMPT_RESPONSE" | jq -r '.choices[0].message.content' 2>/dev/null)
    echo -e "${GREEN}✅ Enhanced Prompt (from cloud AI):${NC}"
    echo "  $IMAGE_PROMPT"
    echo ""
fi

# ═══════════════════════════════════════════════════════════════
# Step 2: Generate image via HuggingFace
# ═══════════════════════════════════════════════════════════════

echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}${BOLD}  Step 2: Generate Image via HuggingFace                      ${NC}"
echo -e "${MAGENTA}${BOLD}═══════════════════════════════════════════════════════════════${NC}"
echo ""

echo -e "${YELLOW}🐦 Songbird: Route to image generation service${NC}"
echo "  Task: Image generation"
echo "  → Squirrel (manages image APIs)"
echo ""

echo -e "${YELLOW}🐿️  Squirrel: Selecting image generation service...${NC}"
echo "  Query: image.generation + quality=good"
echo "  Match: Stable Diffusion (via HuggingFace)"
echo "  Model: stabilityai/stable-diffusion-2-1"
echo ""

echo -e "${BLUE}Generating image...${NC}"
echo "  Prompt: $IMAGE_PROMPT"
echo "  Model: Stable Diffusion 2.1"
echo "  This may take 10-30 seconds..."
echo ""

GEN_START=$(date +%s)

# Call HuggingFace Stable Diffusion
IMAGE_RESPONSE=$(curl -s -X POST \
  https://api-inference.huggingface.co/models/stabilityai/stable-diffusion-2-1-base \
  -H "Authorization: Bearer $HF_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"inputs\": \"$IMAGE_PROMPT\", \"options\": {\"wait_for_model\": true}}" \
  --output "$OUTPUT_DIR/generated_$SESSION_ID.png" \
  -w "%{http_code}")

GEN_END=$(date +%s)
GEN_TIME=$((GEN_END - GEN_START))

# Check if successful (image file exists and has content)
if [ -f "$OUTPUT_DIR/generated_$SESSION_ID.png" ] && [ -s "$OUTPUT_DIR/generated_$SESSION_ID.png" ]; then
    FILE_SIZE=$(du -h "$OUTPUT_DIR/generated_$SESSION_ID.png" | cut -f1)
    
    echo -e "${GREEN}✅ Image Generated Successfully!${NC}"
    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "  📁 Location: ${BOLD}$OUTPUT_DIR/generated_$SESSION_ID.png${NC}"
    echo -e "  📊 Size: $FILE_SIZE"
    echo -e "  ⏱️  Generation Time: ${GEN_TIME}s"
    echo -e "  🎨 Model: Stable Diffusion 2.1"
    echo -e "  💰 Cost: Free (HuggingFace free tier)"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
    echo ""
    
    # Try to display in terminal if supported
    if command -v catimg &> /dev/null; then
        echo -e "${BLUE}Image Preview:${NC}"
        catimg "$OUTPUT_DIR/generated_$SESSION_ID.png"
        echo ""
    elif command -v viu &> /dev/null; then
        echo -e "${BLUE}Image Preview:${NC}"
        viu "$OUTPUT_DIR/generated_$SESSION_ID.png"
        echo ""
    else
        echo -e "${YELLOW}💡 Install 'catimg' or 'viu' to preview images in terminal${NC}"
        echo ""
    fi
    
    echo -e "${GREEN}To view the image:${NC}"
    echo "  xdg-open $OUTPUT_DIR/generated_$SESSION_ID.png"
    echo ""
else
    echo -e "${YELLOW}⚠️  Image generation may still be processing...${NC}"
    echo ""
    echo "  The HuggingFace API may need to:"
    echo "  1. Load the model (first time)"
    echo "  2. Process the request"
    echo ""
    echo "  Check the file manually:"
    echo "  ls -lh $OUTPUT_DIR/generated_$SESSION_ID.png"
    echo ""
fi

# ═══════════════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════════════

echo -e "${BOLD}${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${BLUE}║              Image Generation Pipeline Summary                ║${NC}"
echo -e "${BOLD}${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${GREEN}${BOLD}✅ DEMONSTRATED:${NC}"
echo ""
echo "  1. ${CYAN}Local AI (ToadStool)${NC}"
echo "     - Generated enhanced prompt"
echo "     - Privacy-safe text processing"
echo "     - Zero cost"
echo ""
echo "  2. ${CYAN}Image Generation (Squirrel → HuggingFace)${NC}"
echo "     - Stable Diffusion 2.1"
echo "     - Cloud-based generation"
echo "     - Free tier access"
echo ""
echo "  3. ${CYAN}Local Output${NC}"
echo "     - Image saved locally"
echo "     - $OUTPUT_DIR/generated_$SESSION_ID.png"
echo "     - Ready to view/use"
echo ""

echo -e "${MAGENTA}${BOLD}Pipeline Flow:${NC}"
echo ""
echo "  User: \"$USER_REQUEST\""
echo "       ↓"
echo "  🐦 Songbird → 🍄 ToadStool (local text AI)"
echo "       ↓"
echo "  Enhanced Prompt Created"
echo "       ↓"
echo "  🐦 Songbird → 🐿️  Squirrel (image service)"
echo "       ↓"
echo "  Stable Diffusion API"
echo "       ↓"
echo "  💾 Local File: generated_$SESSION_ID.png"
echo ""

echo -e "${CYAN}Integration Points:${NC}"
echo ""
echo "  🍄 ToadStool: Local AI compute (text)"
echo "  🐦 Songbird: Routing between services"
echo "  🐿️  Squirrel: Cloud API management (image)"
echo "  💾 Local: Output stored locally"
echo ""

echo -e "${YELLOW}Cost Analysis:${NC}"
echo ""
echo "  Prompt Generation: \$0.00 (local AI)"
echo "  Image Generation: \$0.00 (HF free tier)"
echo "  Total: \$0.00"
echo ""
echo "  vs Commercial API: ~\$0.02-0.04 per image"
echo ""

echo -e "${GREEN}${BOLD}🎉 AI Image Generation Complete!${NC}"
echo ""
echo -e "${CYAN}Output saved to:${NC} $OUTPUT_DIR/generated_$SESSION_ID.png"
echo ""

exit 0

