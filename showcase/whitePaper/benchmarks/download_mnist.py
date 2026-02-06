#!/usr/bin/env python3
"""Download MNIST dataset for FHE benchmarking."""

import urllib.request
import gzip
import os
import struct
import numpy as np

# MNIST URLs
BASE_URL = "https://ossci-datasets.s3.amazonaws.com/mnist/"
FILES = [
    "train-images-idx3-ubyte.gz",
    "train-labels-idx1-ubyte.gz",
    "t10k-images-idx3-ubyte.gz",
    "t10k-labels-idx1-ubyte.gz",
]

DATA_DIR = "../data/datasets/mnist"

def download_file(url, filepath):
    """Download a file from URL to filepath."""
    print(f"Downloading {url}...")
    urllib.request.urlretrieve(url, filepath)
    print(f"  → Saved to {filepath}")

def extract_images(filepath):
    """Extract images from MNIST file format."""
    with gzip.open(filepath, 'rb') as f:
        magic, num, rows, cols = struct.unpack(">IIII", f.read(16))
        images = np.frombuffer(f.read(), dtype=np.uint8).reshape(num, rows * cols)
    return images

def extract_labels(filepath):
    """Extract labels from MNIST file format."""
    with gzip.open(filepath, 'rb') as f:
        magic, num = struct.unpack(">II", f.read(8))
        labels = np.frombuffer(f.read(), dtype=np.uint8)
    return labels

def main():
    # Create data directory
    os.makedirs(DATA_DIR, exist_ok=True)
    
    # Download all MNIST files
    for filename in FILES:
        url = BASE_URL + filename
        filepath = os.path.join(DATA_DIR, filename)
        
        if os.path.exists(filepath):
            print(f"✓ {filename} already exists")
        else:
            download_file(url, filepath)
    
    # Extract and validate
    print("\n" + "="*60)
    print("Validating MNIST dataset...")
    print("="*60)
    
    train_images = extract_images(os.path.join(DATA_DIR, "train-images-idx3-ubyte.gz"))
    train_labels = extract_labels(os.path.join(DATA_DIR, "train-labels-idx1-ubyte.gz"))
    test_images = extract_images(os.path.join(DATA_DIR, "t10k-images-idx3-ubyte.gz"))
    test_labels = extract_labels(os.path.join(DATA_DIR, "t10k-labels-idx1-ubyte.gz"))
    
    print(f"\n✓ Training set: {train_images.shape[0]} images ({train_images.shape[1]} features)")
    print(f"  Labels: {train_labels.shape[0]} ({np.unique(train_labels)})")
    print(f"\n✓ Test set: {test_images.shape[0]} images ({test_images.shape[1]} features)")
    print(f"  Labels: {test_labels.shape[0]} ({np.unique(test_labels)})")
    
    # Save as numpy arrays for easy loading
    np.save(os.path.join(DATA_DIR, "train_images.npy"), train_images)
    np.save(os.path.join(DATA_DIR, "train_labels.npy"), train_labels)
    np.save(os.path.join(DATA_DIR, "test_images.npy"), test_images)
    np.save(os.path.join(DATA_DIR, "test_labels.npy"), test_labels)
    
    print(f"\n✓ Saved numpy arrays to {DATA_DIR}/")
    print("\n" + "="*60)
    print("✅ MNIST dataset ready for FHE benchmarking!")
    print("="*60)

if __name__ == "__main__":
    main()
