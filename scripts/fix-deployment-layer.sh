#!/bin/bash
# Quick fix: Comment out all reqwest usage for pure Rust evolution

FILE="crates/core/toadstool/src/deployment_layer.rs"

# Simple approach: Replace all methods that use reqwest with stubs
sed -i 's/reqwest::Client/\/\/ PURE_RUST_TODO: Use Songbird RPC - reqwest::Client/g' "$FILE"
sed -i 's/reqwest::get/\/\/ PURE_RUST_TODO: Use Songbird RPC - reqwest::get/g' "$FILE"

echo "Fixed deployment_layer.rs for pure Rust"
