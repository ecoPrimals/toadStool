#!/bin/bash
# Setup NeuroBench for neuromorphic benchmarking
#
# Usage: ./scripts/setup-neurobench.sh

set -e

echo "=== NeuroBench Setup ==="
echo ""

# Check Python
if ! command -v python3 &> /dev/null; then
    echo "✗ Python 3 not found"
    exit 1
fi
echo "✓ Python: $(python3 --version)"

# Create virtual environment if needed
VENV_DIR="$(dirname "$0")/../.venv-neurobench"
if [ ! -d "$VENV_DIR" ]; then
    echo ""
    echo "Creating virtual environment..."
    python3 -m venv "$VENV_DIR"
fi

# Activate and install
source "$VENV_DIR/bin/activate"
echo "✓ Virtual environment: $VENV_DIR"

echo ""
echo "Installing NeuroBench..."
pip install --upgrade pip > /dev/null
pip install neurobench snntorch torch torchaudio torchvision > /dev/null 2>&1 || {
    echo "Installing with --no-deps for missing packages..."
    pip install neurobench || echo "NeuroBench may have additional dependencies"
}

echo ""
echo "Verifying installation..."
python3 -c "
import sys
try:
    import neurobench
    print(f'✓ NeuroBench version: {neurobench.__version__}')
except ImportError as e:
    print(f'✗ NeuroBench import error: {e}')
    sys.exit(1)

try:
    import snntorch
    print(f'✓ SNNTorch version: {snntorch.__version__}')
except ImportError:
    print('! SNNTorch not available (optional)')

try:
    import torch
    print(f'✓ PyTorch version: {torch.__version__}')
except ImportError:
    print('✗ PyTorch not installed')
"

echo ""
echo "=== Available Benchmarks ==="
python3 -c "
try:
    from neurobench.benchmarks import BENCHMARKS
    for name in BENCHMARKS:
        print(f'  - {name}')
except Exception as e:
    print('Benchmarks:')
    print('  - Keyword FSCIL')
    print('  - DVS Gesture')
    print('  - Event Camera Object Detection')
    print('  - NHP Motor Prediction')
    print('  - Chaotic Function Prediction')
"

echo ""
echo "To run benchmarks:"
echo "  source $VENV_DIR/bin/activate"
echo "  python -c 'from neurobench.benchmarks import run_benchmark; run_benchmark(\"dvs_gesture\")'"
echo ""
