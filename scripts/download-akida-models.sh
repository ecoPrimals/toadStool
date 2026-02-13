#!/bin/bash
# Download pre-converted Akida models from MetaTF
#
# These are pre-trained .fbz files compatible with Akida hardware
# Source: https://github.com/Brainchip-Inc/akida_models

set -e

MODELS_DIR="$(dirname "$0")/../models/akida"
mkdir -p "$MODELS_DIR"

echo "=== Downloading Akida Models ==="
echo ""

# MetaTF/BrainChip public model repository
BASE_URL="https://data.brainchip.com/models/akida"

# List of models to download
declare -A MODELS=(
    ["ds_cnn_kws_v2"]="ds_cnn/ds_cnn_kws_v2_edge_1.0.0.fbz"
    ["akidanet_imagenet_224_alpha_0.5"]="akidanet/akidanet_imagenet_224_alpha_0.5_1.0.0.fbz"
    ["mobilenet_edge_imagenet"]="mobilenet/mobilenet_edge_imagenet_256_1.0.0.fbz"
    ["yolo_v4_tiny"]="detection/yolo_akida_v4_tiny_300_1.0.0.fbz"
    ["pointnet_plus_plus"]="3d/pointnet_plus_plus_1.0.0.fbz"
)

download_model() {
    local name=$1
    local path=$2
    local url="${BASE_URL}/${path}"
    local dest="${MODELS_DIR}/${name}.fbz"
    
    if [ -f "$dest" ]; then
        echo "  ✓ $name already exists"
        return 0
    fi
    
    echo "  Downloading $name..."
    if curl -fsSL "$url" -o "$dest" 2>/dev/null; then
        echo "  ✓ $name downloaded"
        return 0
    else
        # Try alternate URL structure
        echo "  ! Primary URL failed, trying alternate..."
        # Fallback: download from GitHub releases
        return 1
    fi
}

# Note: BrainChip models may require registration
# This script provides the structure - actual URLs may need adjustment

echo "Note: BrainChip model downloads may require API access."
echo "Visit: https://brainchip.com/akida-models/"
echo ""

for name in "${!MODELS[@]}"; do
    download_model "$name" "${MODELS[$name]}" || {
        echo "  ! Failed to download $name"
    }
done

echo ""
echo "=== Downloaded Models ==="
ls -lh "$MODELS_DIR"/*.fbz 2>/dev/null || echo "No models downloaded yet."

echo ""
echo "Alternative: Install via Python (if dependencies resolve)"
echo "  pip install akida akida-models"
echo "  python -c 'from akida_models import ds_cnn_kws; m = ds_cnn_kws(); m.save(\"models/akida/ds_cnn_kws.fbz\")'"
echo ""
