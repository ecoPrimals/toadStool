#!/usr/bin/env python3
"""
Download Akida Model Zoo models for testing

Usage:
    python scripts/setup-akida-models.py

Requirements:
    pip install akida-models tensorflow
"""

import os
import sys
from pathlib import Path

MODELS_DIR = Path(__file__).parent.parent / "models" / "akida"

def main():
    print("=== Akida Model Zoo Setup ===")
    print()
    
    # Check dependencies
    try:
        import akida_models
        print(f"✓ akida-models version: {akida_models.__version__}")
    except ImportError:
        print("✗ akida-models not installed")
        print("  Run: pip install akida-models")
        sys.exit(1)
    
    try:
        import tensorflow as tf
        print(f"✓ tensorflow version: {tf.__version__}")
    except ImportError:
        print("✗ tensorflow not installed")
        print("  Run: pip install tensorflow")
        sys.exit(1)
    
    # Create models directory
    MODELS_DIR.mkdir(parents=True, exist_ok=True)
    print(f"✓ Models directory: {MODELS_DIR}")
    print()
    
    # Download models
    models_to_download = [
        ("ds_cnn_kws", "Keyword Spotting (DS-CNN)", "ds_cnn_kws"),
        ("akidanet_imagenet", "ImageNet Classification (AkidaNet 0.5)", "akidanet_imagenet"),
        # Add more as needed
    ]
    
    for func_name, description, filename in models_to_download:
        print(f"Downloading {description}...")
        try:
            # Import model function
            model_func = getattr(akida_models, func_name)
            
            # Get model
            model = model_func()
            
            # Save as Akida model
            model_path = MODELS_DIR / f"{filename}.fbz"
            if hasattr(model, 'save'):
                model.save(str(model_path))
                print(f"  ✓ Saved to {model_path}")
            else:
                # Convert Keras to Akida if needed
                print(f"  ! Model is Keras, needs conversion to Akida")
                keras_path = MODELS_DIR / f"{filename}.keras"
                model.save(str(keras_path))
                print(f"  ✓ Saved Keras model to {keras_path}")
                print(f"  → Convert with: akida_models.akidanet_imagenet_pretrained()")
                
        except Exception as e:
            print(f"  ✗ Failed: {e}")
    
    print()
    print("=== Available Models ===")
    for f in MODELS_DIR.iterdir():
        size_mb = f.stat().st_size / (1024 * 1024)
        print(f"  {f.name} ({size_mb:.1f} MB)")
    
    print()
    print("To use in Rust:")
    print("  let model_bytes = std::fs::read(\"models/akida/ds_cnn_kws.fbz\")?;")
    print("  device.load_model(&model_bytes)?;")

if __name__ == "__main__":
    main()
