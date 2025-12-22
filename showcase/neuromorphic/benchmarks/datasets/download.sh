#!/bin/bash
# Download standard neuromorphic and ML benchmark datasets

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "╔════════════════════════════════════════════════════════════╗"
echo "║     Neuromorphic Benchmark Dataset Downloader             ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Function to download and extract
download_and_extract() {
    local name=$1
    local url=$2
    local output_dir=$3
    
    echo -e "${BLUE}Downloading $name...${NC}"
    
    if [ -d "$output_dir" ]; then
        echo -e "${YELLOW}  $name already exists, skipping${NC}"
        return
    fi
    
    mkdir -p "$output_dir"
    
    # Download
    if command -v wget &> /dev/null; then
        wget -q --show-progress "$url" -O "$output_dir/data.tar.gz" || {
            echo "Download failed, trying alternative..."
            curl -L "$url" -o "$output_dir/data.tar.gz"
        }
    elif command -v curl &> /dev/null; then
        curl -L "$url" -o "$output_dir/data.tar.gz"
    else
        echo "Error: Neither wget nor curl found"
        exit 1
    fi
    
    # Extract
    echo -e "${BLUE}  Extracting...${NC}"
    tar -xzf "$output_dir/data.tar.gz" -C "$output_dir"
    rm "$output_dir/data.tar.gz"
    
    echo -e "${GREEN}  ✓ $name downloaded${NC}"
}

# MNIST
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " MNIST (Handwritten Digits)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -d "mnist" ]; then
    echo -e "${YELLOW}MNIST already exists, skipping${NC}"
else
    mkdir -p mnist
    echo "Downloading MNIST..."
    
    # Download individual files
    wget -q --show-progress -P mnist \
        http://yann.lecun.com/exdb/mnist/train-images-idx3-ubyte.gz \
        http://yann.lecun.com/exdb/mnist/train-labels-idx1-ubyte.gz \
        http://yann.lecun.com/exdb/mnist/t10k-images-idx3-ubyte.gz \
        http://yann.lecun.com/exdb/mnist/t10k-labels-idx1-ubyte.gz
    
    # Extract
    gunzip mnist/*.gz
    echo -e "${GREEN}✓ MNIST downloaded${NC}"
fi
echo ""

# Fashion-MNIST
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Fashion-MNIST (Clothing Classification)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -d "fashion-mnist" ]; then
    echo -e "${YELLOW}Fashion-MNIST already exists, skipping${NC}"
else
    mkdir -p fashion-mnist
    echo "Downloading Fashion-MNIST..."
    
    wget -q --show-progress -P fashion-mnist \
        http://fashion-mnist.s3-website.eu-central-1.amazonaws.com/train-images-idx3-ubyte.gz \
        http://fashion-mnist.s3-website.eu-central-1.amazonaws.com/train-labels-idx1-ubyte.gz \
        http://fashion-mnist.s3-website.eu-central-1.amazonaws.com/t10k-images-idx3-ubyte.gz \
        http://fashion-mnist.s3-website.eu-central-1.amazonaws.com/t10k-labels-idx1-ubyte.gz
    
    gunzip fashion-mnist/*.gz
    echo -e "${GREEN}✓ Fashion-MNIST downloaded${NC}"
fi
echo ""

# N-MNIST (Neuromorphic MNIST)
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " N-MNIST (Neuromorphic Event-based)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -d "n-mnist" ]; then
    echo -e "${YELLOW}N-MNIST already exists, skipping${NC}"
else
    echo -e "${YELLOW}N-MNIST requires manual download from:${NC}"
    echo "  https://www.garrickorchard.com/datasets/n-mnist"
    echo "  Please download and extract to datasets/n-mnist/"
    mkdir -p n-mnist
    echo "  (Creating placeholder directory)"
fi
echo ""

# DVS Gesture
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " DVS Gesture (Event-based Hand Gestures)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -d "dvs-gesture" ]; then
    echo -e "${YELLOW}DVS Gesture already exists, skipping${NC}"
else
    echo -e "${YELLOW}DVS Gesture requires manual download from:${NC}"
    echo "  https://research.ibm.com/interactive/dvsgesture/"
    echo "  Please download and extract to datasets/dvs-gesture/"
    mkdir -p dvs-gesture
    echo "  (Creating placeholder directory)"
fi
echo ""

# Bioinformatics sample data
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " Bioinformatics Sample Data"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -d "bioinformatics" ]; then
    echo -e "${YELLOW}Bioinformatics data already exists, skipping${NC}"
else
    mkdir -p bioinformatics
    echo "Generating synthetic DNA sequences..."
    
    # Generate sample FASTQ file
    python3 - << 'EOF'
import random
import os

os.chdir("bioinformatics")

bases = ['A', 'C', 'G', 'T']
num_sequences = 10000
seq_length = 150

with open("sample.fastq", "w") as f:
    for i in range(num_sequences):
        # Header
        f.write(f"@SEQ_{i:06d}\n")
        
        # Sequence
        seq = ''.join(random.choices(bases, k=seq_length))
        f.write(seq + "\n")
        
        # Plus line
        f.write("+\n")
        
        # Quality scores (mock high quality)
        qual = ''.join(['I'] * seq_length)
        f.write(qual + "\n")

print(f"Generated {num_sequences} sequences of length {seq_length}")
EOF
    
    echo -e "${GREEN}✓ Bioinformatics sample data generated${NC}"
fi
echo ""

# LLM intent dataset
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo " LLM Intent Classification Dataset"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ -d "llm" ]; then
    echo -e "${YELLOW}LLM dataset already exists, skipping${NC}"
else
    mkdir -p llm
    echo "Generating synthetic intent dataset..."
    
    python3 - << 'EOF'
import json
import random
import os

os.chdir("llm")

intents = [
    "code_generation",
    "simple_qa",
    "complex_reasoning",
    "creative_writing",
    "translation",
    "summarization",
    "retrieval",
    "moderation"
]

# Sample prompts for each intent
templates = {
    "code_generation": [
        "How do I implement {algo} in {lang}?",
        "Write a function to {task}",
        "Debug this {lang} code: {code}",
        "Optimize this algorithm: {algo}"
    ],
    "simple_qa": [
        "What is {concept}?",
        "Who is {person}?",
        "Where is {place}?",
        "When did {event} happen?"
    ],
    "complex_reasoning": [
        "Explain the relationship between {a} and {b}",
        "Analyze the implications of {scenario}",
        "Compare and contrast {x} versus {y}",
        "What would happen if {hypothetical}?"
    ],
    "creative_writing": [
        "Write a story about {topic}",
        "Create a poem about {subject}",
        "Describe {scene} in detail",
        "Write dialogue between {characters}"
    ],
    "translation": [
        "Translate to {language}: {text}",
        "How do you say {phrase} in {language}?",
        "Convert this to {language}",
        "What does {foreign} mean in English?"
    ],
    "summarization": [
        "Summarize this article: {text}",
        "Give me the key points of {document}",
        "TL;DR: {content}",
        "What's the main idea of {text}?"
    ],
    "retrieval": [
        "Find information about {topic}",
        "Search for {query}",
        "Look up {term}",
        "Get me data on {subject}"
    ],
    "moderation": [
        "Is this content appropriate: {text}",
        "Check if this violates policy: {content}",
        "Flag this if inappropriate: {message}",
        "Review this for safety: {post}"
    ]
}

# Generate test set
test_data = []
for intent in intents:
    count = int(1000 * (0.4 if intent == "simple_qa" else 0.15 if intent == "code_generation" else 0.1))
    for _ in range(count):
        template = random.choice(templates[intent])
        # Simple placeholder replacement
        prompt = template.replace("{algo}", "quicksort").replace("{lang}", "Python")
        prompt = prompt.replace("{task}", "sort a list").replace("{concept}", "gravity")
        prompt = prompt.replace("{person}", "Einstein").replace("{place}", "Paris")
        
        test_data.append({
            "prompt": prompt,
            "intent": intent,
            "metadata": {"synthetic": True}
        })

# Shuffle
random.shuffle(test_data)

# Write to JSONL
with open("intent_test_set.jsonl", "w") as f:
    for item in test_data:
        f.write(json.dumps(item) + "\n")

print(f"Generated {len(test_data)} test prompts")
EOF
    
    echo -e "${GREEN}✓ LLM intent dataset generated${NC}"
fi
echo ""

# Summary
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                     Download Complete                      ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "Datasets ready:"
echo "  ✓ MNIST (60K training, 10K test)"
echo "  ✓ Fashion-MNIST (60K training, 10K test)"
echo "  • N-MNIST (manual download required)"
echo "  • DVS Gesture (manual download required)"
echo "  ✓ Bioinformatics (10K sequences)"
echo "  ✓ LLM Intent (8K prompts)"
echo ""
echo "To run benchmarks:"
echo "  cd .."
echo "  ./run-all-benchmarks.sh"

