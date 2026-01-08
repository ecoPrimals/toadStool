#!/usr/bin/env bash
# Check if Vulkan Compute might be an alternative path to AMD GPU

echo "Checking Vulkan support..."
echo ""

if command -v vulkaninfo &> /dev/null; then
    echo "✅ vulkaninfo found"
    vulkaninfo --summary 2>&1 | grep -A 5 "GPU"
else
    echo "❌ vulkaninfo not found - install vulkan-tools"
    echo "   sudo apt install vulkan-tools"
fi

echo ""
echo "Vulkan ICD loaders:"
ls -la /usr/share/vulkan/icd.d/ 2>&1 || echo "No Vulkan ICDs found"

echo ""
echo "Note: Vulkan Compute is an alternative path to AMD GPU"
echo "      that doesn't rely on ROCm/HSA/OpenCL stack"
